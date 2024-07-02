use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use log::{debug, info};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Request, Response};
use serde_json::Value;

use crate::error::Result;
pub use crate::models::{CollectionModel, EnvironmentModel, RequestModel};
use crate::models::{GraphGLBody, HttpAuth, HttpBody};

pub mod error;
mod models;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug)]
pub struct ApiClientRequest {
    collection: CollectionModel,
    request: RequestModel,
    global_variables: Option<HashMap<String, String>>,
    override_variables: Option<HashMap<String, String>>,
    environment: Option<EnvironmentModel>,
}

impl ApiClientRequest {
    pub fn new(collection: CollectionModel, request: RequestModel) -> Self {
        Self {
            collection,
            request,
            global_variables: None,
            override_variables: None,
            environment: None,
        }
    }

    pub fn with_global_variables(mut self, vars: HashMap<String, String>) -> Self {
        self.global_variables = Some(vars);
        self
    }

    pub fn with_override_variables(mut self, vars: HashMap<String, String>) -> Self {
        self.override_variables = Some(vars);
        self
    }

    pub fn with_environment(mut self, env: EnvironmentModel) -> Self {
        self.environment = Some(env);
        self
    }

    fn prepare(self) -> Result<Request> {
        let hb = {
            let mut hb = handlebars::Handlebars::new();
            hb.set_strict_mode(true);
            hb
        };

        let global_vars = self.global_variables.unwrap_or_default();
        let env = self.environment.unwrap_or_default();
        let override_vars = self.override_variables.unwrap_or_default();

        let mut variables = HashMap::new();
        variables.extend(
            global_vars
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<HashMap<&str, &str>>(),
        );
        variables.extend(self.collection.vars.as_map());
        variables.extend(env.vars.as_map());
        variables.extend(self.request.vars.pre_request.as_map());
        variables.extend(
            override_vars
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<HashMap<&str, &str>>(),
        );

        debug!("Request variables: {:#?}", variables);

        let url = hb.render_template(&self.request.http.url, &variables)?;

        let method =
            reqwest::Method::from_str(self.request.http.method.as_str()).expect("invalid method");
        let url = reqwest::Url::parse(&url).expect("invalid url");

        let headers = {
            let mut h = HeaderMap::new();

            for i in self.collection.headers.items() {
                let key = hb.render_template(&i.key, &variables)?;
                let val = hb.render_template(&i.value, &variables)?;

                // TODO: Handle error
                h.insert(
                    HeaderName::from_str(&key).expect("invalid header name"),
                    HeaderValue::from_str(&val).expect("invalid header value"),
                );
            }

            for i in self.request.http.headers.items() {
                let key = hb.render_template(&i.key, &variables)?;
                let val = hb.render_template(&i.value, &variables)?;

                // TODO: Handle error
                h.insert(
                    HeaderName::from_str(&key).expect("invalid header name"),
                    HeaderValue::from_str(&val).expect("invalid header value"),
                );
            }

            h
        };

        let mut req = reqwest::Client::new()
            .request(method, url)
            .headers(headers)
            .query(&self.request.http.params.get_query_params());

        if let Some(auth) = self.request.http.auth.or(self.collection.auth) {
            req = match auth {
                HttpAuth::Basic(b) => {
                    let username = hb.render_template(&b.username, &variables)?;
                    let password = Some(hb.render_template(&b.password, &variables)?);

                    req.basic_auth(username, password)
                }
                HttpAuth::Bearer(t) => {
                    let token = hb.render_template(&t.token, &variables)?;
                    req.bearer_auth(token)
                }
            }
        }

        if let Some(body) = self.request.http.body {
            req = match body {
                HttpBody::Text(t) => {
                    let text = hb.render_template(&t.text, &variables)?;
                    req.header("Content-Type", "text/plain").body(text)
                }
                HttpBody::Json(j) => {
                    // TODO: Find a better way than re/deserializing.
                    let json_str = serde_json::to_string(&j.json)?;
                    let json_str = hb.render_template(&json_str, &variables)?;
                    let json: Value = serde_json::from_str(&json_str)?;

                    req.json(&json)
                }
                HttpBody::GraphQL(g) => {
                    let query = hb.render_template(&g.graphql.query, &variables)?;

                    let variables = {
                        let mut vars = HashMap::new();

                        for (k, v) in g.graphql.variables.iter() {
                            let key = hb.render_template(k, &variables)?;
                            let value = hb.render_template(v, &variables)?;

                            vars.insert(key, value);
                        }

                        vars
                    };

                    let payload = GraphGLBody { query, variables };

                    req.json(&payload)
                }
                HttpBody::Binary(b) => {
                    let body = hb.render_template(&b.binary, &variables)?;

                    // TODO Manage Error
                    req.header("Content-Type", "application/x-www-form-urlencoded")
                        .body(BASE64_STANDARD.decode(body).expect("invalid base64"))
                }
                HttpBody::Form(f) => {
                    let mut form = HashMap::new();
                    for i in f.form.items() {
                        form.insert(
                            hb.render_template(&i.key, &variables)?,
                            hb.render_template(&i.value, &variables)?,
                        );
                    }

                    req.form(&form)
                }
            }
        }

        req = req.timeout(Duration::from_secs(60));

        Ok(req.build()?)
    }

    pub async fn execute(self) -> Result<Response> {
        let request = self.prepare()?;

        info!("{} {}", request.method(), request.url());

        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()?;
        let resp = client.execute(request).await?;

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use once_cell::sync::Lazy;
    use reqwest::StatusCode;
    use rstest::rstest;
    use serde_json::Value;
    use wiremock::{http, matchers, Match, Request};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::models::{
        HttpAuth, HttpBasicAuth, HttpBearerToken, HttpBinaryBody, HttpBody, HttpFormBody,
        HttpJsonBody, HttpMethod, HttpParamsModel, HttpRequestModel, HttpTextBody, KeyValueList,
        KeyValuePair, RequestVarsModel,
    };
    use crate::{ApiClientRequest, CollectionModel, RequestModel};

    static TRACING: Lazy<()> = Lazy::new(|| {
        env_logger::init();
    });

    pub struct TestServer {
        pub mock: MockServer,
        pub base_url: String,
    }

    pub async fn spawn_mock_server() -> TestServer {
        Lazy::force(&TRACING);

        let mock = MockServer::start().await;
        let base_url = mock.uri();

        TestServer { mock, base_url }
    }

    // Check that a header with the specified name doesn't exist.
    pub struct HeaderIsMissingMatcher(http::HeaderName);

    impl Match for HeaderIsMissingMatcher {
        fn matches(&self, request: &Request) -> bool {
            request.headers.get(&self.0).is_none()
        }
    }

    // Check that the body contains exactly the following form items
    pub struct FormDataMatcher(HashMap<String, String>);

    impl Match for FormDataMatcher {
        fn matches(&self, request: &Request) -> bool {
            let values: HashMap<String, String> = match serde_urlencoded::from_bytes(&request.body)
            {
                Ok(v) => v,
                Err(_) => return false,
            };

            values == self.0
        }
    }

    #[tokio::test]
    async fn api_client_performs_basic_request() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn api_client_uses_specified_path() {
        let path = "/foo/bar/baz";

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::path(path))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: format!("{}{}", test_server.base_url, path),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[rstest]
    #[case(HttpMethod::Get)]
    #[case(HttpMethod::Post)]
    #[case(HttpMethod::Delete)]
    #[tokio::test]
    async fn api_client_uses_specified_method(#[case] method: HttpMethod) {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::method(method.as_str()))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                method,
                url: test_server.base_url,
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn api_client_sends_headers() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::header("X-Test-Header-1", "some-test-value"))
            .and(matchers::header("X-Test-Header-2", "other-test-value"))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                headers: KeyValueList::new(vec![
                    KeyValuePair {
                        key: "X-Test-Header-1".to_string(),
                        value: "some-test-value".to_string(),
                        enabled: Some(true),
                    },
                    KeyValuePair {
                        key: "X-Test-Header-2".to_string(),
                        value: "other-test-value".to_string(),
                        enabled: Some(true),
                    },
                ]),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn api_client_ignores_disabled_headers() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::header(
            "explicit-enabled",
            "explicit-enabled-value",
        ))
        .and(matchers::header(
            "implicit-enabled",
            "implicit-enabled-value",
        ))
        .and(HeaderIsMissingMatcher("disabled".try_into().unwrap()))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&test_server.mock)
        .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                headers: KeyValueList::new(vec![
                    KeyValuePair {
                        key: "explicit-enabled".to_string(),
                        value: "explicit-enabled-value".to_string(),
                        enabled: Some(true),
                    },
                    KeyValuePair {
                        key: "implicit-enabled".to_string(),
                        value: "implicit-enabled-value".to_string(),
                        enabled: None,
                    },
                    KeyValuePair {
                        key: "disabled".to_string(),
                        value: "disabled-value".to_string(),
                        enabled: Some(false),
                    },
                ]),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn api_client_sends_query_params() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::query_param("param1", "value1"))
            .and(matchers::query_param("param2", "value2"))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                params: HttpParamsModel {
                    query: KeyValueList::new(vec![
                        KeyValuePair {
                            key: "param1".to_string(),
                            value: "value1".to_string(),
                            enabled: Some(true),
                        },
                        KeyValuePair {
                            key: "param2".to_string(),
                            value: "value2".to_string(),
                            enabled: Some(true),
                        },
                    ]),
                },
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn api_client_ignores_disabled_query_params() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::query_param(
            "explicit-enabled",
            "explicit-enabled-value",
        ))
        .and(matchers::query_param(
            "implicit-enabled",
            "implicit-enabled-value",
        ))
        .and(matchers::query_param_is_missing("disabled"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&test_server.mock)
        .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                params: HttpParamsModel {
                    query: KeyValueList::new(vec![
                        KeyValuePair {
                            key: "explicit-enabled".to_string(),
                            value: "explicit-enabled-value".to_string(),
                            enabled: Some(true),
                        },
                        KeyValuePair {
                            key: "implicit-enabled".to_string(),
                            value: "implicit-enabled-value".to_string(),
                            enabled: None,
                        },
                        KeyValuePair {
                            key: "disabled".to_string(),
                            value: "disabled-value".to_string(),
                            enabled: Some(false),
                        },
                    ]),
                },
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[rstest]
    #[case::basic(
        HttpAuth::Basic(HttpBasicAuth{username: "user".to_string(), password: "pass".to_string()}),
        "Basic dXNlcjpwYXNz",
    )]
    #[case::bearer(
        HttpAuth::Bearer(HttpBearerToken{token: "bearer-token".to_string()}),
        "Bearer bearer-token"
    )]
    #[tokio::test]
    async fn api_client_sends_auth(#[case] auth: HttpAuth, #[case] expected: &str) {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::header("Authorization", expected))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                auth: Some(auth),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_sends_text_body() {
        let body = "some text value";

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::body_string(body))
            .and(matchers::header("Content-Type", "text/plain"))
            .and(matchers::header("Content-Length", body.len()))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                body: Some(HttpBody::Text(HttpTextBody {
                    text: body.to_string(),
                })),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_sends_json_body() {
        let body: Value = serde_json::from_str(
            r#"
        {
            "name": "some-name",
            "flag": true,
            "id": 123,
            "data": {
                "foo": "bar"
            }
        }
        "#,
        )
        .unwrap();

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::body_json(&body))
            .and(matchers::header("Content-Type", "application/json"))
            // TODO: Check len
            // .and(matchers::header("Content-Length", body.len()))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                body: Some(HttpBody::Json(HttpJsonBody { json: body })),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_sends_binary_body() {
        let body: Vec<u8> = vec![
            0xa3, 0x2d, 0x30, 0x1f, 0xc9, 0x5f, 0xc1, 0xdf, 0x9f, 0x8e, 0x1d, 0xff, 0x56, 0xb7,
            0xef, 0xac, 0x0f, 0x4f, 0xe6, 0x62, 0x82, 0x91, 0xbc, 0xb9, 0xb9, 0x4a, 0x20, 0xfa,
            0x68, 0x3c, 0x18, 0x8e,
        ];

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::body_bytes(body.clone()))
            .and(matchers::header(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .and(matchers::header("Content-Length", body.len()))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                body: Some(HttpBody::Binary(HttpBinaryBody {
                    binary: BASE64_STANDARD.encode(body),
                })),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_sends_form_body() {
        let form = vec![
            KeyValuePair {
                key: "name".to_string(),
                value: "Firstname Lastname".to_string(),
                enabled: Some(true),
            },
            KeyValuePair {
                key: "email".to_string(),
                value: "firstname.lastname@example.org".to_string(),
                enabled: Some(true),
            },
        ];

        let mut expected_data = HashMap::new();
        expected_data.insert("name".to_string(), "Firstname Lastname".to_string());
        expected_data.insert(
            "email".to_string(),
            "firstname.lastname@example.org".to_string(),
        );
        let expected_len = serde_urlencoded::to_string(&expected_data).unwrap().len();

        let test_server = spawn_mock_server().await;
        Mock::given(FormDataMatcher(expected_data))
            .and(matchers::header(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .and(matchers::header("Content-Length", expected_len))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                body: Some(HttpBody::Form(HttpFormBody {
                    form: KeyValueList::new(form),
                })),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_ignores_disabled_form_body() {
        let form = vec![
            KeyValuePair {
                key: "findme1".to_string(),
                value: "".to_string(),
                enabled: Some(true),
            },
            KeyValuePair {
                key: "findme2".to_string(),
                value: "".to_string(),
                enabled: None,
            },
            KeyValuePair {
                key: "ignoreme".to_string(),
                value: "".to_string(),
                enabled: Some(false),
            },
        ];
        let mut expected_data = HashMap::new();
        expected_data.insert("findme1".to_string(), "".to_string());
        expected_data.insert("findme2".to_string(), "".to_string());
        let expected_len = serde_urlencoded::to_string(&expected_data).unwrap().len();

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::any())
            .and(FormDataMatcher(expected_data))
            .and(matchers::header(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .and(matchers::header("Content-Length", expected_len))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                url: test_server.base_url,
                body: Some(HttpBody::Form(HttpFormBody {
                    form: KeyValueList::new(form),
                })),
                ..Default::default()
            },
            vars: Default::default(),
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_url() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let variables = [("url", test_server.base_url)];

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: "{{url}}".to_string(),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_basic_auth() {
        let test_server = spawn_mock_server().await;
        Mock::given(matchers::header(
            "Authorization",
            "Basic YS11c2VybmFtZTphLXBhc3N3b3Jk",
        ))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&test_server.mock)
        .await;

        let variables = [("username", "a-username"), ("password", "a-password")];

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                auth: Some(HttpAuth::Basic(HttpBasicAuth {
                    username: "{{username}}".to_string(),
                    password: "{{password}}".to_string(),
                })),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_bearer_auth() {
        let token = "mmOjB9WuQXBxCFwvxn7v7qWn3sonKzIy";

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::header(
            "Authorization",
            format!("Bearer {}", token),
        ))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&test_server.mock)
        .await;

        let variables = [("token", token)];

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                auth: Some(HttpAuth::Bearer(HttpBearerToken {
                    token: "{{token}}".to_string(),
                })),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_headers() {
        let header_name = "X-Test-Header";
        let header_value = "some-test-value";

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::header(header_name, header_value))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let variables = [("header_name", header_name), ("header_value", header_value)];

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                headers: KeyValueList::from([("{{header_name}}", "{{header_value}}")]),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_text_body() {
        let key = "some-test-key";
        let value = "some-test-value";

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::body_string(format!("{} / {}", key, value)))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let variables = [("key", key), ("value", value)];

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                body: Some(HttpBody::Text(HttpTextBody {
                    text: "{{key}} / {{value}}".to_string(),
                })),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_json_body() {
        let key = "some-test-key";
        let value = "some-test-value";

        let variables = [("key", key), ("value", value)];
        let expected_body = HashMap::from(variables);

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::body_json(expected_body))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let body: Value = serde_json::from_str(
            r#"
        {
            "key": "{{key}}",
            "value": "{{value}}"
        }
        "#,
        )
        .unwrap();

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                body: Some(HttpBody::Json(HttpJsonBody { json: body })),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_binary_body() {
        let data = "c29tZS1kYXRh"; // "some-data"

        let variables = [("data", data)];

        let test_server = spawn_mock_server().await;
        Mock::given(matchers::body_bytes("some-data".as_bytes()))
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&test_server.mock)
            .await;

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                body: Some(HttpBody::Binary(HttpBinaryBody {
                    binary: "{{data}}".to_string(),
                })),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }

    #[tokio::test]
    async fn test_client_applies_templating_to_form_body() {
        let key = "some-test-key";
        let value = "some-test-value";

        let variables = [("key", key), ("value", value)];

        let test_server = spawn_mock_server().await;
        Mock::given(FormDataMatcher(HashMap::from([(
            key.to_string(),
            value.to_string(),
        )])))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&test_server.mock)
        .await;

        let request = RequestModel {
            http: HttpRequestModel {
                method: HttpMethod::Get,
                url: test_server.base_url,
                body: Some(HttpBody::Form(HttpFormBody {
                    form: KeyValueList::from([("{{key}}", "{{value}}")]),
                })),
                ..Default::default()
            },
            vars: RequestVarsModel {
                pre_request: KeyValueList::from(variables),
                ..Default::default()
            },
        };

        let api_request = ApiClientRequest::new(CollectionModel::default(), request);

        api_request.execute().await.expect("request failed");
    }
}

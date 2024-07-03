use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Deserialize)]
pub(crate) struct KeyValueList(Vec<KeyValuePair>);

impl KeyValueList {
    #[allow(dead_code)]
    pub(crate) fn new(values: Vec<KeyValuePair>) -> Self {
        Self(values)
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = &KeyValuePair> {
        self.0.iter().filter(|i| i.enabled.unwrap_or(true))
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for KeyValueList
where
    K: Into<String>,
    V: Into<String>,
{
    fn from(arr: [(K, V); N]) -> Self {
        Self(
            arr.into_iter()
                .map(|(k, v)| KeyValuePair {
                    key: k.into(),
                    value: v.into(),
                    enabled: Some(true),
                })
                .collect(),
        )
    }
}

impl<'a> KeyValueList {
    pub(crate) fn as_map(&'a self) -> HashMap<&'a str, &'a str> {
        self.items()
            .map(|p| (p.key.as_str(), p.value.as_str()))
            .collect()
    }

    fn as_tuple_list(&'a self) -> Vec<(&'a str, &'a str)> {
        self.items()
            .map(|p| (p.key.as_str(), p.value.as_str()))
            .collect()
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct EnvironmentModel {
    #[serde(default)]
    pub(crate) vars: KeyValueList,
}

#[derive(Default, Debug, Deserialize)]
enum RequestType {
    #[default]
    #[serde(alias = "http")]
    Http,
}

#[derive(Default, Debug, Deserialize)]
struct RequestMetaModel {
    _name: String,
    #[serde(alias = "type")]
    _type_: RequestType,
    _seq: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct KeyValuePair {
    pub(crate) key: String,
    pub(crate) value: String,
    // TODO: check serde_bool
    pub(crate) enabled: Option<bool>,
}

#[derive(Default, Debug, Deserialize)]
pub(crate) struct HttpParamsModel {
    #[serde(default)]
    pub(crate) query: KeyValueList,
}

impl HttpParamsModel {
    pub(crate) fn get_query_params(&self) -> Vec<(&str, &str)> {
        self.query.as_tuple_list()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpBasicAuth {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpBearerToken {
    pub(crate) token: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum HttpAuth {
    None,
    Basic(HttpBasicAuth),
    Bearer(HttpBearerToken),
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Connect => "CONNECT",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Trace => "TRACE",
            HttpMethod::Patch => "PATCH",
        }
    }
}

#[derive(Default, Debug, Deserialize)]
pub struct CollectionModel {
    #[serde(default)]
    pub(crate) headers: KeyValueList,
    pub(crate) auth: Option<HttpAuth>,
    #[serde(default)]
    pub(crate) vars: KeyValueList,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GraphGLBody {
    pub(crate) query: String,
    #[serde(default)]
    pub(crate) variables: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum HttpBody {
    Text(HttpTextBody),
    Json(HttpJsonBody),
    GraphQL(HttpGraphQLBody),
    Binary(HttpBinaryBody),
    Form(HttpFormBody),
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpTextBody {
    pub(crate) text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpJsonBody {
    pub(crate) json: Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpGraphQLBody {
    pub(crate) graphql: GraphGLBody,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpBinaryBody {
    pub(crate) binary: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HttpFormBody {
    pub(crate) form: KeyValueList,
}

#[derive(Default, Debug, Deserialize)]
pub(crate) struct HttpRequestModel {
    pub(crate) method: HttpMethod,
    pub(crate) url: String, // validate len > 0
    pub(crate) auth: Option<HttpAuth>,
    #[serde(default)]
    pub(crate) headers: KeyValueList,
    #[serde(default)]
    pub(crate) params: HttpParamsModel,
    #[serde(default)]
    pub(crate) body: Option<HttpBody>,
}

#[derive(Default, Debug, Deserialize)]
pub(crate) struct RequestVarsModel {
    #[serde(alias = "pre-request", default)]
    pub(crate) pre_request: KeyValueList,
    #[serde(alias = "post-request", default)]
    pub(crate) _post_request: KeyValueList,
}

#[derive(Default, Debug, Deserialize)]
pub struct RequestModel {
    // _meta: RequestMetaModel,
    pub(crate) http: HttpRequestModel,
    #[serde(default)]
    pub(crate) vars: RequestVarsModel,
}

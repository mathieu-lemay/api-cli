use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde::de::{MapAccess, Visitor};
use serde_json::Value;
use crate::error::ApiClientError;

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct NameValueList(Vec<NameValuePair>);

impl NameValueList {
    #[allow(dead_code)]
    pub(crate) fn new(values: Vec<NameValuePair>) -> Self {
        Self(values)
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = &NameValuePair> {
        self.0.iter().filter(|i| i.enabled.unwrap_or(true))
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for NameValueList
where
    K: Into<String>,
    V: Into<String>,
{
    fn from(arr: [(K, V); N]) -> Self {
        Self(
            arr.into_iter()
                .map(|(k, v)| NameValuePair {
                    name: k.into(),
                    value: v.into(),
                    enabled: Some(true),
                })
                .collect(),
        )
    }
}

impl<'a> NameValueList {
    pub(crate) fn as_map(&'a self) -> HashMap<&'a str, &'a str> {
        self.items()
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect()
    }

    fn as_tuple_list(&'a self) -> Vec<(&'a str, &'a str)> {
        self.items()
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect()
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct EnvironmentModel {
    #[serde(default)]
    pub(crate) variables: NameValueList,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NameValuePair {
    pub(crate) name: String,
    pub(crate) value: String,
    // TODO: check serde_bool
    pub(crate) enabled: Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct HttpParamsModel {
    #[serde(default)]
    pub(crate) query: NameValueList,
}

impl HttpParamsModel {
    pub(crate) fn get_query_params(&self) -> Vec<(&str, &str)> {
        self.query.as_tuple_list()
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct HttpBasicAuth {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct HttpBearerToken {
    pub(crate) token: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum HttpAuth {
    #[default]
    None,
    Basic(HttpBasicAuth),
    Bearer(HttpBearerToken),
    Inherit,
}

impl FromStr for HttpAuth {
    type Err = ApiClientError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inherit" => Ok(HttpAuth::Inherit),
            _ => Err(ApiClientError::from(format!("Invalid http auth type: {}", s))),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CollectionModel {
    #[serde(default)]
    pub(crate) request: CollectionRequestModel,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CollectionRequestModel {
    #[serde(default)]
    pub(crate) headers: NameValueList,
    pub(crate) auth: Option<HttpAuth>,
    #[serde(default)]
    pub(crate) variables: NameValueList,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GraphQLBody {
    pub(crate) query: String,
    #[serde(default)]
    pub(crate) variables: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum HttpBody {
    Text(HttpTextBody),
    Json(HttpJsonBody),
    GraphQL(HttpGraphQLBody),
    Binary(HttpBinaryBody),
    Form(HttpFormBody),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpTextBody {
    pub(crate) text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpJsonBody {
    pub(crate) json: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpGraphQLBody {
    pub(crate) graphql: GraphQLBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpBinaryBody {
    pub(crate) binary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpFormBody {
    pub(crate) form: NameValueList,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct HttpRequestModel {
    pub(crate) method: HttpMethod,
    pub(crate) url: String, // validate len > 0
    #[serde(deserialize_with = "string_or_struct")]
    pub(crate) auth: HttpAuth,
    #[serde(default)]
    pub(crate) headers: NameValueList,
    #[serde(default)]
    pub(crate) params: HttpParamsModel,
    #[serde(default)]
    pub(crate) body: Option<HttpBody>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct RequestVarsModel {
    #[serde(alias = "pre-request", default)]
    pub(crate) pre_request: NameValueList,
    #[serde(alias = "post-request", default)]
    pub(crate) _post_request: NameValueList,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct HttpRequestRuntimeModel {
    pub(crate) variables: NameValueList,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RequestModel {
    // _meta: RequestMetaModel,
    pub(crate) http: HttpRequestModel,
    pub(crate) runtime: HttpRequestRuntimeModel,
}

fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = ApiClientError>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = ApiClientError>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

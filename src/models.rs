use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::{Error as _, MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize};
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
        self.0.iter().filter(|i| !i.disabled.unwrap_or(false))
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
                    disabled: None,
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
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NameValuePair {
    pub(crate) name: String,
    pub(crate) value: String,
    // TODO: check serde_bool
    pub(crate) disabled: Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct FormValueList(Vec<FormValue>);

impl FormValueList {
    #[allow(dead_code)]
    pub(crate) fn new(values: Vec<FormValue>) -> Self {
        Self(values)
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = &FormValue> {
        self.0.iter().filter(|i| !i.disabled.unwrap_or(false))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FormValue {
    pub(crate) name: String,
    #[serde(deserialize_with = "string_or_list")]
    pub(crate) value: String,
    pub(crate) content_type: Option<String>,
    #[serde(default, rename = "type")]
    pub(crate) type_: FormValueType,
    pub(crate) disabled: Option<bool>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FormValueType {
    #[default]
    Text,
    File,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct HttpParamList(Vec<HttpParam>);

impl HttpParamList {
    #[allow(dead_code)]
    pub(crate) fn new(values: Vec<HttpParam>) -> Self {
        Self(values)
    }

    pub(crate) fn items(&self) -> impl Iterator<Item = &HttpParam> {
        self.0.iter().filter(|i| !i.disabled.unwrap_or(false))
    }
}

impl<'a> HttpParamList {
    pub(crate) fn get_query_params(&'a self) -> Vec<(&'a str, &'a str)> {
        self.items()
            .filter(|i| i.type_ == HttpParamType::Query)
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect()
    }

    // TODO: Handle path params
    pub(crate) fn get_path_params(&'a self) -> Vec<(&'a str, &'a str)> {
        self.items()
            .filter(|i| i.type_ == HttpParamType::Path)
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpParam {
    pub(crate) name: String,
    pub(crate) value: String,
    #[serde(rename = "type")]
    pub(crate) type_: HttpParamType,
    // TODO: check serde_bool
    pub(crate) disabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HttpParamType {
    Query,
    Path,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct EnvironmentModel {
    #[serde(default)]
    pub(crate) variables: NameValueList,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct HttpBasicAuth {
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
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
            _ => Err(ApiClientError::from(format!(
                "Invalid http auth type: {}",
                s
            ))),
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
    #[serde(rename = "form-urlencoded")]
    FormUrlEncoded(HttpFormBody),
    #[serde(rename = "multipart-form")]
    MultipartForm(HttpMultipartFormBody),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpTextBody {
    pub(crate) data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpJsonBody {
    pub(crate) data: Value,
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
    pub(crate) data: FormValueList,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HttpMultipartFormBody {
    pub(crate) data: FormValueList,
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
    pub(crate) params: HttpParamList,
    #[serde(default)]
    pub(crate) body: Option<HttpBody>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct HttpRequestRuntimeModel {
    pub(crate) variables: NameValueList,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RequestModel {
    // _meta: RequestMetaModel,
    pub(crate) http: HttpRequestModel,
    #[serde(default)]
    pub(crate) runtime: HttpRequestRuntimeModel,
}

fn string_or_list<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrList;

    impl<'de> Visitor<'de> for StringOrList {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or list of one string")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_owned())
        }

        fn visit_string<E>(self, value: String) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<String, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let v = match seq.next_element::<String>()? {
                Some(v) => v,
                None => return Err(A::Error::custom("array is empty")),
            };

            if seq.next_element::<String>()?.is_some() {
                return Err(A::Error::custom("array contains more than one element"));
            };

            Ok(v)
        }
    }

    deserializer.deserialize_any(StringOrList)
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

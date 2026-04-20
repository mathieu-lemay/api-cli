use std::error;
use std::fmt::{self, Debug};

pub type Result<T> = exn::Result<T, ApiClientError>;

#[derive(Debug)]
pub struct ApiClientError(String);
impl error::Error for ApiClientError {}

impl fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}

impl ApiClientError {
    pub fn new(msg: String) -> Self {
        Self(msg)
    }
}

impl From<String> for ApiClientError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ApiClientError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

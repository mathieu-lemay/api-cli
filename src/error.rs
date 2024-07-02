use std::ffi::OsString;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::Path;
use std::{error, io};

pub type Result<T> = std::result::Result<T, ApiClientError>;
pub struct ApiClientError(Box<ErrorImpl>);

#[derive(Debug)]
enum ErrorKind {
    #[allow(dead_code)] // Value will show up in the error message
    IoError(Option<OsString>),
    ReqwestError,
    #[allow(dead_code)] // Value will show up in the error message
    SerdeJson(Option<OsString>),
    #[allow(dead_code)] // Value will show up in the error message
    SerdeYaml(Option<OsString>),
    TemplateRenderError,
}

#[derive(Debug)]
pub struct ErrorImpl {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

impl ApiClientError {
    pub fn from_io_error_with_path(error: io::Error, path: &Path) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::IoError(Some(path.as_os_str().to_owned())),
            error: Box::new(error),
        }))
    }

    pub fn from_serde_json_error_with_path(error: serde_json::Error, path: &Path) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::SerdeJson(Some(path.as_os_str().to_owned())),
            error: Box::new(error),
        }))
    }

    pub fn from_serde_yaml_error_with_path(error: serde_yaml::Error, path: &Path) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::SerdeYaml(Some(path.as_os_str().to_owned())),
            error: Box::new(error),
        }))
    }
}

impl From<io::Error> for ApiClientError {
    fn from(e: io::Error) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::IoError(None),
            error: Box::new(e),
        }))
    }
}

impl From<reqwest::Error> for ApiClientError {
    fn from(e: reqwest::Error) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::ReqwestError,
            error: Box::new(e),
        }))
    }
}

impl From<serde_json::Error> for ApiClientError {
    fn from(e: serde_json::Error) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::SerdeJson(None),
            error: Box::new(e),
        }))
    }
}

impl From<handlebars::RenderError> for ApiClientError {
    fn from(e: handlebars::RenderError) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::TemplateRenderError,
            error: Box::new(e),
        }))
    }
}

impl Display for ApiClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&format!("{:?}: {}", &self.0.kind, &self.0.error), f)
    }
}

impl Debug for ApiClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&format!("{:?}", &self.0), f)
    }
}

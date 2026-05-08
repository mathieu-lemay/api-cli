use std::str::FromStr;

use exn::bail;

use crate::error::{ApiClientError, Result};
use crate::models::HttpAuth;

pub enum Auth {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
}

pub fn get_auth(request_auth: &HttpAuth, collection_auth: &Option<HttpAuth>) -> Result<Auth> {
    let request_auth = match request_auth {
        HttpAuth::Inherit => collection_auth.as_ref().unwrap_or(&HttpAuth::None),
        _ => request_auth,
    };

    let a = match request_auth {
        HttpAuth::Basic(basic) => Auth::Basic {
            username: basic.username.clone(),
            password: basic.password.clone(),
        },
        HttpAuth::Bearer(token) => Auth::Bearer {
            token: token.token.clone(),
        },
        HttpAuth::None => Auth::None,
        HttpAuth::Inherit => bail!(ApiClientError::from("Invalid auth: inherit")),
    };

    Ok(a)
}

use std::collections::HashMap;
use std::io::stdout;
use std::time::Duration;

use api_cli::error::Result;
use reqwest::Response;
use serde::Serialize;

#[derive(Serialize)]
struct Status {
    code: u16,
    text: String,
}

impl From<&Response> for Status {
    fn from(value: &Response) -> Self {
        let status = value.status();

        Self {
            code: status.as_u16(),
            text: status
                .canonical_reason()
                .map_or_else(String::new, |s| s.to_string()),
        }
    }
}

#[derive(Serialize)]
struct ResponseData {
    status: Status,
    latency: Duration,
    headers: HashMap<String, String>,
    body: String,
}

pub(crate) async fn json_print(response: Response, latency: Duration) -> Result<()> {
    let status: Status = (&response).into();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_owned(),
                v.to_str().unwrap_or("<INVALID>").to_owned(),
            )
        })
        .collect();

    let body = get_response_body(response).await;

    let data = ResponseData {
        status,
        latency,
        headers,
        body: body.unwrap_or_else(String::new),
    };

    serde_json::to_writer(stdout(), &data)?;

    Ok(())
}

async fn get_response_body(response: Response) -> Option<String> {
    let resp_body = response.bytes().await.expect("error reading response body");
    if resp_body.is_empty() {
        return None;
    }

    String::from_utf8(resp_body.into_iter().collect::<Vec<u8>>()).ok()
}

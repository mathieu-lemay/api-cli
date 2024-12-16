use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

use api_cli::error::Result;
use colored_json::to_colored_json_auto;
use jsonpath_rust::JsonPath;
use owo_colors::Stream::Stdout;
use owo_colors::{OwoColorize, Style as OwoStyle};
use reqwest::Response;
use serde_json::Value;
use tabled::settings::object::Rows;
use tabled::settings::{Remove, Style};
use tabled::{Table, Tabled};
use textwrap::{termwidth, Options};

use crate::commands::RunArgs;

#[derive(Tabled)]
struct HeaderRow<'a, S: AsRef<str> + Display> {
    pub(crate) name: &'a str,
    pub(crate) value: S,
}

pub(crate) async fn pretty_print(
    args: &RunArgs,
    response: Response,
    latency: Duration,
) -> Result<()> {
    let mut request_results = vec![
        ("Status", get_formatted_status(&response)),
        ("Latency", get_formatted_latency(latency)),
    ];

    if !args.no_headers {
        if let Some(h) = get_formatted_headers(&response) {
            request_results.push(("Headers", h));
        }
    }

    if !args.headers_only {
        if let Some(b) = get_formatted_body(response, &args.json_path).await? {
            request_results.push(("Body", b));
        }
    }

    let mut result_table = Table::new(request_results);
    result_table
        .with(Style::modern())
        .with(Remove::row(Rows::first()));
    println!("{}", result_table);

    Ok(())
}

fn get_formatted_status(res: &Response) -> String {
    res.status()
        .if_supports_color(Stdout, |s| {
            let mut status_style = OwoStyle::new();
            status_style = match s.as_u16() {
                100..=199 => status_style.blue(),
                200..=299 => status_style.green(),
                300..=399 => status_style.cyan(),
                400..=499 => status_style.yellow(),
                500..=599 => status_style.red(),
                _ => status_style,
            };
            s.style(status_style)
        })
        .to_string()
}

fn get_formatted_latency(latency: Duration) -> String {
    let formatted_latency = format!("{:?}", latency);
    formatted_latency
        .if_supports_color(Stdout, |d| {
            let mut status_style = OwoStyle::new();
            status_style = match latency.as_secs_f64() {
                1.0..=5.0 => status_style.yellow(),
                5.0.. => status_style.red(),
                _ => status_style.green(),
            };
            d.style(status_style)
        })
        .to_string()
}

fn get_formatted_headers(res: &Response) -> Option<String> {
    let headers = res.headers();

    if headers.is_empty() {
        return None;
    }

    let longest_header_name = headers.keys().map(|k| k.as_str().len()).max().unwrap();

    let max_width = termwidth();
    // 21 assumes "Headers" is the longest row name
    // Add 16 to ensure we can actually see something
    if max_width < 21 + 16 + longest_header_name {
        return Some(
            "Terminal too narrow"
                .if_supports_color(Stdout, |t| t.red())
                .to_string(),
        );
    }

    let width = termwidth() - 21 - longest_header_name;
    let values: Vec<HeaderRow<String>> = headers
        .iter()
        .map(|(k, v)| HeaderRow {
            name: k.as_str(),
            value: {
                let val = v.to_str().unwrap_or("");

                textwrap::wrap(val, width).join("\n")
            },
        })
        .collect();

    let mut table = Table::new(values);
    table.with(Style::modern()).with(Remove::row(Rows::first()));

    Some(table.to_string())
}

async fn get_formatted_body(res: Response, json_path: &Option<String>) -> Result<Option<String>> {
    let resp_body = res.bytes().await.expect("error reading response body");
    if resp_body.is_empty() {
        return Ok(None);
    }

    let width = termwidth() - 16; // Assumes "headers" is the longest in the first col.

    if let Ok(v) = serde_json::from_slice::<Value>(&resp_body) {
        let rendered_json = match json_path {
            Some(json_path) => {
                // TODO: Handle errors
                let path = JsonPath::from_str(json_path).unwrap();

                path.find_slice(&v)
                    .into_iter()
                    .map(|s| to_colored_json_auto(&s.to_data()).expect("error colorizing json"))
                    .collect::<Vec<String>>()
                    .join("\n")
            }
            None => to_colored_json_auto(&v).expect("error colorizing json"),
        };

        let body = textwrap::wrap(&rendered_json, Options::new(width).break_words(true));

        return Ok(Some(body.join("\n")));
    } else if let Ok(s) = String::from_utf8(resp_body.into_iter().collect::<Vec<u8>>()) {
        let body = textwrap::wrap(&s, Options::new(width));
        return Ok(Some(body.join("\n")));
    };

    Ok(None)
}

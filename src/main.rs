use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::{env, fs};

use clap::Parser;
use colored_json::to_colored_json_auto;
use jsonpath_rust::{find_slice, JsonPathInst};
use log::debug;
use once_cell::sync::Lazy;
use owo_colors::Stream::Stdout;
use owo_colors::{OwoColorize, Style as OwoStyle};
use reqwest::Response;
use serde::Deserialize;
use serde_json::Value;
use tabled::settings::object::Rows;
use tabled::settings::{Disable, Style};
use tabled::{Table, Tabled};
use textwrap::{termwidth, Options};

use api_cli::error::{ApiClientError, Result};
use api_cli::{ApiClientRequest, CollectionModel, RequestModel};

static API_CLI_BASE_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| {
    env::var("API_CLI_BASE_DIRECTORY")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut d = dirs::data_dir().unwrap_or(PathBuf::from("."));
            d.push("api-cli-collections");

            d
        })
});

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    collection: String,
    request: String,

    #[arg(short, long, help = "Select an environment for the request")]
    environment: Option<String>,

    #[arg(short, long, help = "Apply a json-path filter to the response")]
    json_path: Option<String>,
}

#[derive(Tabled)]
struct HeaderRow<'a, S: AsRef<str> + Display> {
    pub(crate) name: &'a str,
    pub(crate) value: S,
}

fn read_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    let data: String = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            return Err(ApiClientError::from_io_error_with_path(e, path));
        }
    };

    serde_yaml::from_str::<T>(&data)
        .map_err(|e| ApiClientError::from_serde_yaml_error_with_path(e, path))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let collection_file = {
        let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
        p.push(&cli.collection);
        p.push("collection.yaml");

        p
    };
    let collection: CollectionModel = read_file(collection_file.as_path())?;
    debug!("Collection: {:#?}", collection);

    let request_file = {
        let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
        p.push(&cli.collection);
        p.push(format!("{}.yaml", cli.request));

        p
    };
    let req: RequestModel = read_file(request_file.as_path())?;
    debug!("Request: {:#?}", req);

    let mut req = ApiClientRequest::new(collection, req);

    let global_variables: HashMap<String, String> = env::vars()
        .filter(|(k, _)| k.starts_with("API_CLI_VAR_"))
        .map(|(k, v)| (k.strip_prefix("API_CLI_VAR_").unwrap().to_string(), v))
        .collect();

    req = req.with_global_variables(global_variables);

    if let Some(e) = cli.environment {
        let environment_file = {
            let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
            p.push(&cli.collection);
            p.push("environments");
            p.push(format!("{}.yaml", e));

            p
        };
        let env = read_file(environment_file.as_path())?;
        debug!("Environment: {:#?}", env);

        req = req.with_environment(env);
    };

    let request_start = Instant::now();
    let res = req.execute().await.expect("error performing request");
    let request_duration = request_start.elapsed();

    let mut request_results = vec![
        ("Status", get_formatted_status(&res)),
        ("Latency", get_formatted_latency(request_duration)),
    ];

    if let Some(h) = get_formatted_headers(&res) {
        request_results.push(("Headers", h));
    }
    if let Some(b) = get_formatted_body(res, &cli.json_path).await? {
        request_results.push(("Body", b));
    }

    let mut result_table = Table::new(request_results);
    result_table
        .with(Style::modern())
        .with(Disable::row(Rows::first()));
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
    table
        .with(Style::modern())
        .with(Disable::row(Rows::first()));

    Some(table.to_string())
}

async fn get_formatted_body(res: Response, json_path: &Option<String>) -> Result<Option<String>> {
    let resp_body = res.bytes().await.expect("error reading response body");
    let width = termwidth() - 16; // Assumes "headers" is the longest in the first col.

    if let Ok(v) = serde_json::from_slice::<Value>(&resp_body) {
        let rendered_json = match json_path {
            Some(json_path) => {
                // TODO: Handle errors
                let path = JsonPathInst::from_str(json_path).unwrap();

                find_slice(&path, &v)
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

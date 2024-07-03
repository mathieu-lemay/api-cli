use std::env;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::{fs, path::PathBuf};

use serde::Deserialize;

use api_cli::error::{ApiClientError, Result};

use crate::commands::API_CLI_BASE_DIRECTORY;

pub fn read_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    let data: String = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            return Err(ApiClientError::from_io_error_with_path(e, path));
        }
    };

    serde_yaml::from_str::<T>(&data)
        .map_err(|e| ApiClientError::from_serde_yaml_error_with_path(e, path))
}

pub fn get_collections_directory() -> PathBuf {
    PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str())
}

pub fn get_collection_file_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
    p.push(name);
    p.push("collection.yaml");

    p
}

pub fn get_environment_file_path(collection_name: &str, environment_name: &str) -> PathBuf {
    let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
    p.push(collection_name);
    p.push("environments");
    p.push(format!("{}.yaml", environment_name));

    p
}

pub fn get_request_file_path(collection_name: &str, request_name: &str) -> PathBuf {
    let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
    p.push(collection_name);
    p.push(format!("{}.yaml", request_name));

    p
}

pub fn open_file_in_editor(file_path: &PathBuf) -> Result<ExitStatus> {
    let editor = env::var("EDITOR").unwrap_or("vi".to_string());

    let status = Command::new(editor).args([file_path]).status()?;

    Ok(status)
}
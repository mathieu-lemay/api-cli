use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{env, fs};

use api_cli::error::{ApiClientError, Result};
use exn::{ensure, ResultExt};
use log::debug;
use serde::Deserialize;

use super::API_CLI_BASE_DIRECTORY;

pub fn read_file<T: for<'a> Deserialize<'a>>(path: &Path) -> exn::Result<T, ApiClientError> {
    let data: String = fs::read_to_string(path)
        .or_raise(|| ApiClientError::new(format!("unable to open file: {:?}", path)))?;

    serde_yaml::from_str::<T>(&data)
        .or_raise(|| ApiClientError::new(format!("file is not valid yaml: {:?}", path)))
}

pub fn get_collections_directory() -> PathBuf {
    PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str())
}

pub fn get_collection_file_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
    p.push(name);
    p.push("opencollection.yml");

    p
}

pub fn get_environment_file_path(collection_name: &str, environment_name: &str) -> PathBuf {
    let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
    p.push(collection_name);
    p.push("environments");
    p.push(format!("{}.yml", environment_name));

    p
}

pub fn get_request_file_path(collection_name: &str, request_name: &str) -> PathBuf {
    let mut p = PathBuf::from(API_CLI_BASE_DIRECTORY.as_os_str());
    p.push(collection_name);
    // TODO: Use `:` everywhere
    p.push(format!("{}.yml", request_name.replace(':', "/")));

    p
}

pub fn open_file_in_editor(collection_dir: &PathBuf, file_path: &PathBuf) -> Result<ExitStatus> {
    let editor = env::var("EDITOR").unwrap_or("vi".to_string());

    debug!("Opening file {:?} in {}", file_path, editor);

    let status = Command::new(editor)
        .args([file_path])
        .current_dir(collection_dir)
        .status()
        .or_raise(|| "Error starting editor".into())?;

    Ok(status)
}

/// Get the path to the collection directory if it exists
pub(super) fn ensure_collection_directory(collection_name: &str) -> Result<PathBuf> {
    let collection_path = get_collection_file_path(collection_name);

    ensure!(
        collection_path.exists(),
        ApiClientError::new(format!("Collection not found: {}", collection_name))
    );

    let collection_directory = collection_path.parent().unwrap().to_owned();

    Ok(collection_directory)
}

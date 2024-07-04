use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::Path;

use api_cli::error::{ApiClientError, Result};
use api_cli::RequestModel;

use crate::commands::utils::get_collection_file_path;
use crate::commands::{RequestCmd, RequestCreateArgs, RequestEditArgs, RequestListArgs};

use super::utils::{get_request_file_path, open_file_in_editor};

pub fn run_request_command(cmd: RequestCmd) -> Result<()> {
    match cmd {
        RequestCmd::Create(args) => create_request(args),
        RequestCmd::Edit(args) => edit_request(args),
        RequestCmd::List(args) => list_requests(args),
    }
}

fn create_request(args: RequestCreateArgs) -> Result<()> {
    let request_path = get_request_file_path(&args.collection_name, &args.name);

    if request_path.exists() {
        return Err(ApiClientError::new_request_already_exists(args.name));
    }

    fs::create_dir_all(request_path.parent().unwrap())?;

    let writer = File::create(&request_path)?;
    serde_yaml::to_writer(writer, &RequestModel::default())?;

    if args.edit {
        open_file_in_editor(&request_path)?;
    }

    Ok(())
}

fn edit_request(args: RequestEditArgs) -> Result<()> {
    let request_path = get_request_file_path(&args.collection_name, &args.name);

    if !request_path.exists() {
        return Err(ApiClientError::new_request_not_found(args.name));
    }

    open_file_in_editor(&request_path)?;

    Ok(())
}

fn list_requests(args: RequestListArgs) -> Result<()> {
    let request_names = find_requests(args.collection_name)?;

    for n in request_names {
        println!("{}", n);
    }

    Ok(())
}

fn find_requests(collection_name: String) -> Result<Vec<String>> {
    let collection_path = get_collection_file_path(&collection_name);
    if !collection_path.exists() {
        return Err(ApiClientError::new_collection_not_found(collection_name));
    }

    let collection_directory = collection_path.parent().unwrap();

    let mut request_names = Vec::new();

    request_names.extend(find_requests_in_directory(
        collection_directory,
        collection_directory,
    )?);
    request_names.sort();

    Ok(request_names)
}

fn find_requests_in_directory(collection_dir: &Path, dir: &Path) -> Result<Vec<String>> {
    let mut request_names = Vec::new();

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        // TODO: Put collection def somewhere else, or put requests in their own subfolder
        let name = path.file_name().unwrap();
        if name == "collection.yaml" || name == "environments" {
            continue;
        }

        if path.is_dir() {
            request_names.extend(find_requests_in_directory(collection_dir, &path)?);
            continue;
        }

        if path.extension().unwrap_or(OsStr::new("")) != "yaml" {
            continue;
        }

        let name = path
            .strip_prefix(collection_dir)
            .unwrap()
            .to_string_lossy()
            .replace('/', ":")
            .strip_suffix(".yaml")
            .unwrap()
            .to_string();

        request_names.push(name);
    }

    Ok(request_names)
}

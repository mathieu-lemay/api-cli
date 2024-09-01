use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::PathBuf;

use api_cli::error::{ApiClientError, Result};
use api_cli::EnvironmentModel;

use super::utils::{ensure_collection_directory, get_environment_file_path, open_file_in_editor};
use super::{EnvironmentCmd, EnvironmentCreateArgs, EnvironmentEditArgs, EnvironmentListArgs};

pub fn run_environment_command(cmd: EnvironmentCmd) -> Result<()> {
    match cmd {
        EnvironmentCmd::Create(args) => create_environment(args),
        EnvironmentCmd::Edit(args) => edit_environment(args),
        EnvironmentCmd::List(args) => list_environments(args),
    }
}

fn create_environment(args: EnvironmentCreateArgs) -> Result<()> {
    let collection_dir = ensure_collection_directory(&args.collection_name)?;

    let environment_path = get_environment_file_path(&args.collection_name, &args.name);

    if environment_path.exists() {
        return Err(ApiClientError::new_environment_already_exists(args.name));
    }

    fs::create_dir_all(environment_path.parent().unwrap())?;

    let writer = File::create(&environment_path)?;
    serde_yaml::to_writer(writer, &EnvironmentModel::default())?;

    if args.edit {
        open_file_in_editor(&collection_dir, &environment_path)?;
    }

    Ok(())
}

fn edit_environment(args: EnvironmentEditArgs) -> Result<()> {
    let collection_dir = ensure_collection_directory(&args.collection_name)?;

    let environment_path = get_environment_file_path(&args.collection_name, &args.name);

    if !environment_path.exists() {
        return Err(ApiClientError::new_environment_not_found(args.name));
    }

    open_file_in_editor(&collection_dir, &environment_path)?;

    Ok(())
}

fn list_environments(args: EnvironmentListArgs) -> Result<()> {
    let environment_names = find_environments(args.collection_name)?;

    for n in environment_names {
        println!("{}", n);
    }

    Ok(())
}

fn find_environments(collection_name: String) -> Result<Vec<String>> {
    let collection_directory = ensure_collection_directory(&collection_name)?;

    let mut environment_names = find_environments_in_directory(collection_directory)?;
    environment_names.sort();

    Ok(environment_names)
}

fn find_environments_in_directory(collection_dir: PathBuf) -> Result<Vec<String>> {
    let mut environments_dir = collection_dir;
    environments_dir.push("environments");

    if !environments_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut environment_names = Vec::new();

    for entry in fs::read_dir(environments_dir)? {
        let path = entry?.path();

        if path.extension().unwrap_or(OsStr::new("")) != "yaml" {
            continue;
        }

        let name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .strip_suffix(".yaml")
            .unwrap()
            .to_string();

        environment_names.push(name);
    }

    Ok(environment_names)
}

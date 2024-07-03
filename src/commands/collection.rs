use std::fs;
use std::fs::File;

use api_cli::error::{ApiClientError, Result};
use api_cli::CollectionModel;

use crate::commands::utils::{
    get_collection_file_path, get_collections_directory, open_file_in_editor,
};
use crate::commands::{CollectionCmd, CollectionCreateArgs, CollectionEditArgs};

pub fn run_collection_command(cmd: CollectionCmd) -> Result<()> {
    match cmd {
        CollectionCmd::Create(args) => create_collection(args),
        CollectionCmd::Edit(args) => edit_collection(args),
        CollectionCmd::List => list_collections(),
    }
}

fn create_collection(args: CollectionCreateArgs) -> Result<()> {
    let collection_path = get_collection_file_path(&args.name);

    if collection_path.exists() {
        return Err(ApiClientError::new_collection_already_exists(args.name));
    }

    fs::create_dir_all(collection_path.parent().unwrap())?;

    let writer = File::create(&collection_path)?;
    serde_yaml::to_writer(writer, &CollectionModel::default())?;

    if args.edit {
        open_file_in_editor(&collection_path)?;
    }

    Ok(())
}

fn edit_collection(args: CollectionEditArgs) -> Result<()> {
    let collection_path = get_collection_file_path(&args.name);

    if !collection_path.exists() {
        return Err(ApiClientError::new_collection_not_found(args.name));
    }

    open_file_in_editor(&collection_path)?;

    Ok(())
}

fn list_collections() -> Result<()> {
    let collections_directory = get_collections_directory();
    if !collections_directory.exists() {
        fs::create_dir_all(&collections_directory)?;
    }

    let mut collection_names = Vec::new();

    for entry in fs::read_dir(collections_directory)? {
        let entry = entry?;

        let mut path = entry.path();

        if !path.is_dir() {
            continue;
        }

        path.push("collection.yaml");

        if !path.exists() {
            continue;
        }

        let name = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        collection_names.push(name);
    }

    collection_names.sort();

    for n in collection_names {
        println!("{}", n);
    }

    Ok(())
}

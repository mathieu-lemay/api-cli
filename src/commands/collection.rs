use std::fs;
use std::fs::File;

use api_cli::error::{ApiClientError, Result};
use api_cli::CollectionModel;
use exn::{bail, ensure, ResultExt};

use super::utils::{
    ensure_collection_directory,
    get_collection_file_path,
    get_collections_directory,
    open_file_in_editor,
};
use super::{CollectionCmd, CollectionCreateArgs, CollectionEditArgs};

pub fn run_collection_command(cmd: CollectionCmd) -> Result<()> {
    match cmd {
        CollectionCmd::Create(args) => create_collection(args),
        CollectionCmd::Edit(args) => edit_collection(args),
        CollectionCmd::List => list_collections(),
    }
}

fn create_collection(args: CollectionCreateArgs) -> Result<()> {
    let collection_dir_path = ensure_collection_directory(&args.name)?;
    let collection_file_path = get_collection_file_path(&args.name);

    if collection_file_path.exists() {
        bail!(ApiClientError::new(format!(
            "Collection already exists: {}",
            args.name
        )));
    }

    fs::create_dir_all(collection_file_path.parent().unwrap()).or_raise(|| {
        format!(
            "Error creating parent directory: {:?}",
            collection_file_path
        )
        .into()
    })?;

    let writer = File::create(&collection_file_path).or_raise(|| {
        format!("Error creating collection file: {:?}", collection_file_path).into()
    })?;
    serde_yaml::to_writer(writer, &CollectionModel::default())
        .or_raise(|| "Error initializing request".into())?;

    if args.edit {
        open_file_in_editor(&collection_dir_path, &collection_file_path)?;
    }

    Ok(())
}

fn edit_collection(args: CollectionEditArgs) -> Result<()> {
    let collection_dir_path = ensure_collection_directory(&args.name)?;
    let collection_file_path = get_collection_file_path(&args.name);

    ensure!(
        collection_file_path.exists(),
        ApiClientError::new(format!("Invalid collection: {}", args.name))
    );

    open_file_in_editor(&collection_dir_path, &collection_file_path)?;

    Ok(())
}

fn list_collections() -> Result<()> {
    let collection_names = find_collections()?;

    for n in collection_names {
        println!("{}", n);
    }

    Ok(())
}

fn find_collections() -> Result<Vec<String>> {
    let collections_directory = get_collections_directory();
    if !collections_directory.exists() {
        return Ok(vec![]);
    }

    let mut collection_names = Vec::new();

    for entry in fs::read_dir(&collections_directory)
        .or_raise(|| format!("Error reading directory: {:?}", collections_directory).into())?
    {
        let entry = entry.or_raise(|| "Invalid directory entry".into())?;

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

    Ok(collection_names)
}

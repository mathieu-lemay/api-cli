use std::path::PathBuf;
use std::{env, io};

use api_cli::error::Result;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
pub use collection::run_collection_command;
pub use environment::run_environment_command;
use log::debug;
use once_cell::sync::Lazy;
pub use request::run_request_command;
pub use run::execute_request;
use utils::get_collections_directory;

mod collection;
mod environment;
mod request;
mod run;
mod utils;

static APP_NAME: &str = "api-cli";

static API_CLI_BASE_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| {
    env::var("API_CLI_BASE_DIRECTORY")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut d = dirs::data_dir().unwrap_or(PathBuf::from("."));
            d.push(APP_NAME);
            d.push("collections");

            d
        })
});

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Execute a request
    Run(RunArgs),

    /// Generate shell completion
    Completion(CompletionArgs),

    /// Manage collections
    #[command(subcommand)]
    Environment(EnvironmentCmd),

    /// Manage environments
    #[command(subcommand)]
    Collection(CollectionCmd),

    /// Manage requests
    #[command(subcommand)]
    Request(RequestCmd),

    /// Launch a shell in the collections directory
    Cd,
}

#[derive(Args)]
pub struct RunArgs {
    collection: String,
    request: String,

    #[arg(short, long, help = "Select an environment for the request")]
    environment: Option<String>,

    #[arg(short, long, help = "Apply a json-path filter to the response")]
    json_path: Option<String>,

    #[arg(long, help = "Disable display of the headers")]
    no_headers: bool,

    #[arg(long, help = "Display only the headers of the response")]
    headers_only: bool,
}

#[derive(Args)]
pub struct CompletionArgs {
    pub shell: Shell,
}

#[derive(Subcommand)]
pub enum CollectionCmd {
    /// Create a new collection
    Create(CollectionCreateArgs),

    /// Edit a collection
    Edit(CollectionEditArgs),

    /// List available collections
    List,
}

#[derive(Args)]
pub struct CollectionCreateArgs {
    /// Name of the collection to create
    name: String,

    /// Edit after creating
    #[arg(short, long)]
    edit: bool,
}

#[derive(Args)]
pub struct CollectionEditArgs {
    /// Name of the collection to edit
    name: String,
}

#[derive(Subcommand)]
pub enum EnvironmentCmd {
    /// Create a new environment
    Create(EnvironmentCreateArgs),

    /// Edit a environment
    Edit(EnvironmentEditArgs),

    /// List available environment
    List(EnvironmentListArgs),
}

#[derive(Args)]
pub struct EnvironmentCreateArgs {
    /// Name of the collection in which to create the environment
    #[arg(value_name = "COLLECTION")]
    collection_name: String,

    /// Name of the environment to create
    name: String,

    /// Edit after creating
    #[arg(short, long)]
    edit: bool,
}

#[derive(Args)]
pub struct EnvironmentEditArgs {
    /// Name of the collection
    #[arg(value_name = "COLLECTION")]
    collection_name: String,

    /// Name of the environment to create
    name: String,
}

#[derive(Args)]
pub struct EnvironmentListArgs {
    #[arg(value_name = "COLLECTION")]
    collection_name: String,
}

#[derive(Subcommand)]
pub enum RequestCmd {
    /// Create a new request
    Create(RequestCreateArgs),

    /// Edit a request
    Edit(RequestEditArgs),

    /// List available request
    List(RequestListArgs),
}

#[derive(Args)]
pub struct RequestCreateArgs {
    /// Name of the collection in which to create the request
    #[arg(value_name = "COLLECTION")]
    collection_name: String,

    /// Name of the request to create
    name: String,

    /// Edit after creating
    #[arg(short, long)]
    edit: bool,
}

#[derive(Args)]
pub struct RequestEditArgs {
    /// Name of the collection
    #[arg(value_name = "COLLECTION")]
    collection_name: String,

    /// Name of the request to create
    name: String,
}

#[derive(Args)]
pub struct RequestListArgs {
    #[arg(value_name = "COLLECTION")]
    collection_name: String,
}

pub fn generate_shell_completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());

    Ok(())
}

pub fn run_shell() -> Result<()> {
    let shell = env::var("SHELL").unwrap_or("sh".to_string());
    let base_dir = get_collections_directory();

    debug!("running shell: {}", shell);

    let status = std::process::Command::new(shell)
        .env("API_CLI_SUBSHELL", "1")
        .current_dir(base_dir)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(status.into())
    }
}

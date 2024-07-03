use std::path::PathBuf;
use std::{env, io};

use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use once_cell::sync::Lazy;

pub use run::execute_request;

mod run;

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
}

#[derive(Args)]
pub struct RunArgs {
    collection: String,
    request: String,

    #[arg(short, long, help = "Select an environment for the request")]
    environment: Option<String>,

    #[arg(short, long, help = "Apply a json-path filter to the response")]
    json_path: Option<String>,
}

#[derive(Args)]
pub struct CompletionArgs {
    pub shell: Shell,
}

pub fn generate_shell_completion(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

use clap::Parser;

use api_cli::error::Result;
use commands::{execute_request, generate_shell_completion, Cli, Command};

mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => execute_request(args).await?,
        Command::Completion(args) => generate_shell_completion(args.shell),
    }

    Ok(())
}

use api_cli::error::Result;
use clap::Parser;
use commands::{
    execute_request,
    generate_shell_completion,
    run_collection_command,
    run_environment_command,
    run_request_command,
    run_shell,
    Cli,
    Command,
};

mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => execute_request(args).await,
        Command::Completion(args) => generate_shell_completion(args.shell),
        Command::Collection(cmd) => run_collection_command(cmd),
        Command::Environment(cmd) => run_environment_command(cmd),
        Command::Request(cmd) => run_request_command(cmd),
        Command::Cd => run_shell(),
    }
}

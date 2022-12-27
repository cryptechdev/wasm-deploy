#[allow(clippy::large_enum_variant)]
pub mod contract;
pub mod defaults;
pub mod subcommand;
use clap::{CommandFactory, FromArgMatches};
use contract::Contracts;
use subcommand::{execute_custom_args, CustomSubcommand};
use wasm_deploy::{cli::Cli, commands::execute_args, error::DeployError};

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(err) = run().await {
        println!("{err}");
    }
}

async fn run() -> Result<(), DeployError> {
    let cli = Cli::<Contracts, CustomSubcommand>::command();
    let matches = cli.get_matches();
    let args = Cli::<Contracts, CustomSubcommand>::from_arg_matches(&matches)?;
    // You can modify the CLI here
    execute_args(&args).await?;
    execute_custom_args(&args)?;
    Ok(())
}

#[allow(clippy::large_enum_variant)]
pub mod contract;
pub mod defaults;
pub mod subcommand;
use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches};
use contract::Contracts;
use subcommand::{execute_custom_args, CustomSubcommand};
use wasm_deploy::{
    cli::Cli, commands::execute_args, error::DeployError, settings::WorkspaceSettings,
};

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(err) = run().await {
        println!("{err}");
    }
}

async fn run() -> Result<(), DeployError> {
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.parent().unwrap();
    // These settings define where things are located
    // You can use the builder pattern to change them
    // Creating a new instance as below will use the defaults which this project represents
    let settings = WorkspaceSettings::new(workspace_root);
    let cli = Cli::<Contracts, CustomSubcommand>::command();
    let matches = cli.get_matches();
    let args = Cli::<Contracts, CustomSubcommand>::from_arg_matches(&matches)?;
    // You can modify the CLI here
    execute_args(&settings, &args).await?;
    execute_custom_args(&args)?;
    Ok(())
}

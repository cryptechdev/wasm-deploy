pub mod contract;
pub mod defaults;
pub mod subcommand;

use clap::{CommandFactory, FromArgMatches};
use contract::Contracts;
use std::path::PathBuf;
use subcommand::{execute_custom_args, CustomSubcommand};
use wasm_deploy::{cli::Cli, commands::execute_args, settings::WorkspaceSettings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // I recommend using env_logger for logging.
    env_logger::init();
    // This tells wasm-deploy where to find the workspace root.
    // If you have a non-standard workspace layout, you'll probably want to change this.
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.parent().unwrap();
    // These settings define where things are located.
    // You can use the builder pattern to change them.
    // Creating a new instance as below will use the defaults which this project represents.
    let settings = WorkspaceSettings::new(workspace_root)?;
    // Build the CLI and get matches.
    let cli = Cli::<Contracts, CustomSubcommand>::command();
    let matches = cli.get_matches();
    let args = Cli::<Contracts, CustomSubcommand>::from_arg_matches(&matches)?;
    // You can modify the CLI here
    execute_args(&settings, &args).await?;
    // These custom args are entirely options
    // If you don't need them, you can remove this line
    // as well as subcommand.rs
    execute_custom_args(&args)?;
    Ok(())
}

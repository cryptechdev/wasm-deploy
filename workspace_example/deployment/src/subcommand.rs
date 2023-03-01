// You can leave this file empty or unchanged if you dont want custom functionality.
use clap::Parser;
use strum_macros::Display;
use wasm_deploy::{
    cli::{Cli, Commands},
    contract::Contract,
    error::DeployError,
};

//#[async_recursion(?Send)]
pub fn execute_custom_args<C>(cli: &Cli<C, CustomSubcommand>) -> Result<(), DeployError>
where
    C: Contract + Clone,
{
    if let Commands::Custom(command) = &cli.command {
        match command {
            CustomSubcommand::MyCommand => println!("Executing your custom command!"),
        }
    }

    Ok(())
}
// A custom subcommand for user defined functionality.
#[derive(Clone, Parser, Debug, Display)]
#[strum(serialize_all = "snake_case")]
#[clap(rename_all = "snake_case")]
pub enum CustomSubcommand {
    /// This is a command that you can define yourself.
    MyCommand,
}

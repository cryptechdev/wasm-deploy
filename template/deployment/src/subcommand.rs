// You can leave this file empty or unchanged if you don't want custom functionality.
use clap::Subcommand;
use wasm_deploy::{
    cli::{Cli, Commands},
    contract::Deploy,
};

// You may need async recursion for your custom subcommand.
//#[async_recursion(?Send)]
pub fn execute_custom_args<C>(cli: &Cli<C, CustomSubcommand>) -> anyhow::Result<()>
where
    C: Deploy + Clone,
{
    if let Commands::Custom(command) = &cli.command {
        match command {
            CustomSubcommand::MyCommand => println!("Executing your custom command!"),
        }
    }

    Ok(())
}

// A custom subcommand for user defined functionality.
#[derive(Clone, Debug, Subcommand)]
#[clap(rename_all = "kebab_case")]
pub enum CustomSubcommand {
    /// This is a command that you can define yourself.
    MyCommand,
}

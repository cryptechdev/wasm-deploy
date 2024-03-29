use clap::{Parser, Subcommand};
use std::fmt::Debug;
use strum::IntoEnumIterator;

use crate::contract::Deploy;

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli<C, S = EmptySubcommand>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    #[command(subcommand)]
    pub command: Commands<C, S>,

    /// Add additional args to cargo build
    #[arg(long, required = false)]
    pub cargo_args: Vec<String>,
}

#[derive(Parser, Clone, Debug)]
#[clap(rename_all = "snake_case", infer_subcommands = true)]
pub enum Commands<C, S>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    /// Rebuilds deploy
    #[command(visible_alias = "u")]
    Update,

    /// Initializes deploy, adding keys, chains, and envs
    Init,

    /// Builds the contracts
    #[command(visible_alias = "b")]
    Build {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Modify chains
    #[command(arg_required_else_help = true)]
    Chain {
        /// Triggers dialogue to add a chain
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete a chain
        #[arg(short, long, exclusive = true)]
        delete: bool,
    },

    /// Modify keys
    #[command(arg_required_else_help = true)]
    Key {
        /// Triggers dialogue to add a key
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete a key
        #[arg(short, long, exclusive = true)]
        delete: bool,
    },

    /// Modify contracts
    #[command(arg_required_else_help = true)]
    Contract {
        /// Triggers dialogue to add a contract
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete a contract
        #[arg(short, long, exclusive = true)]
        delete: bool,
    },

    /// Builds, optimizes, stores, instantiates and sets configs.
    #[command(visible_alias = "d")]
    Deploy {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>(), value_parser=C::from_str)]
        contracts: Vec<C>,

        /// Deploys but does not recompile first
        #[arg(short, long, required = false)]
        no_build: bool,
    },

    /// Modify deployment environments
    Env {
        /// Triggers dialogue to add a deployment environment
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete deployment environment
        #[arg(short, long, exclusive = true)]
        delete: bool,

        /// Triggers dialogue to select an env to activate
        #[arg(short, long, exclusive = true)]
        select: bool,

        /// Prints the current active env id
        #[arg(short, long, exclusive = true)]
        id: bool,
    },

    /// Generates and imports schemas
    Schema {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Stores code for the contracts
    StoreCode {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Instantiates a contract using the preprogrammed messages
    #[command(visible_alias = "i")]
    Instantiate {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,

        /// Interactive mode
        #[arg(short, long, required = false)]
        interactive: bool,
    },

    /// Migrates contracts
    #[command(visible_alias = "m")]
    Migrate {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,

        /// Interactive mode
        #[arg(short, long, required = false)]
        interactive: bool,
    },

    /// Sets the config of a contract
    SetConfig {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Executes a contract
    #[command(visible_alias = "x")]
    Execute { contract: C },

    /// Sends Cw20 tokens to a contract along with a payload
    Cw20Send { contract: C },

    /// Executes a Cw20 message
    Cw20Execute {},

    /// Queries a Cw20 contract
    Cw20Query {},

    /// Instantiate a Cw20 contract
    Cw20Instantiate {},

    /// Executes a contract with a custom payload
    ExecutePayload {
        #[arg(short, long)]
        contract: C,

        #[arg(short, long)]
        payload: String,
    },

    /// Executes a user defined command
    #[command(flatten)]
    Custom(S),

    /// Sends a query to a contract
    #[command(visible_alias = "q")]
    Query { contract: C },

    /// Sets up the smart contract env with executes
    SetUp {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },
}

fn get_all<C: Deploy + IntoEnumIterator>() -> Vec<String> {
    C::iter().map(|x| x.to_string()).collect()
}

#[derive(Subcommand, Clone, Debug)]
pub enum EmptySubcommand {}

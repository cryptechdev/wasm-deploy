use clap::Parser;

use crate::contract::{Contract, Execute, Query};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli<C, E, Q>
where
    C: Contract,
    E: Execute,
    Q: Query,
{
    #[command(subcommand)]
    pub command: Commands<C, E, Q>,
}

#[derive(Parser, Debug)]
#[clap(rename_all = "snake_case", infer_subcommands = true)]
pub enum Commands<C, E, Q>
where
    C: Contract,
    E: Execute,
    Q: Query,
{
    /// Rebuilds deploy
    Update,

    /// Initializes deploy, adding keys, chains, and envs
    Init,

    /// Builds the contracts
    Build {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Modify chains
    Chain {
        /// Triggers dialogue to add a chain
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete a chain
        #[arg(short, long, exclusive = true)]
        delete: bool,
    },

    /// Modify keys
    Key {
        /// Triggers dialogue to add a key
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete a key
        #[arg(short, long, exclusive = true)]
        delete: bool,
    },

    /// Modify chains
    Contract {
        /// Triggers dialogue to add a contract
        #[arg(short, long, exclusive = true)]
        add: bool,

        /// Triggers dialogue to delete a contract
        #[arg(short, long, exclusive = true)]
        delete: bool,
    },

    /// Builds, optimizes, stores, instantiates and sets configs.
    /// Does not run set_up
    Deploy {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
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
    },

    /// Generates and imports schemas
    Schema {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Stores code for the contracs
    StoreCode {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Instantiates a contract
    Instantiate {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Migrates contracts
    Migrate {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Sets the config of a contract
    SetConfig {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Executes a contract
    #[command(visible_alias = "x")]
    Execute {
        #[command(subcommand)]
        execute_command: Option<E>,
    },

    /// Sends a query to a contract
    #[command(alias = "q")]
    Query {
        #[command(subcommand)]
        contract: Option<Q>,
    },

    /// Sets up the smart contract env with executes
    SetUp {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,
    },

    /// Enables interactive mode
    Interactive,
}

fn get_all<C: Contract>() -> Vec<String> { C::iter().map(|x| x.to_string()).collect() }

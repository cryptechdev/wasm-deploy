use clap::{Parser};
use crate::contract::{Contract, Execute, Query};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli<C, E, Q> 
where C: Contract,
      E: Execute,
      Q: Query
{
    #[command(subcommand)]
    pub command: Commands<C, E, Q>,
}

#[derive(Parser, Debug)]
#[clap(rename_all = "snake_case", infer_subcommands=true)]
pub enum Commands<C, E, Q> 
where C: Contract,
      E: Execute,
      Q: Query   
{
    /// Rebuilds wasm-deploy
    Update { },

    /// Builds the contracts
    Build { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Deploys the contracts
    Deploy { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,

        /// Deploys but does not recompile first
        #[arg(short, long, required=false)]
        no_build: bool,
    },

    /// Generates and imports schemas
    Schema { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Stores code for the contracs
    StoreCode { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Instantiates a contract
    Instantiate { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Migrates contracts
    Migrate { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Sets the config of a contract
    SetConfig { 
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Executes a contract
    #[command(visible_alias="x")]
    Execute { 
        #[command(subcommand)]
        execute_command: Option<E>,
    },

    /// Sends a query to a contract
    #[command(alias="q")]
    Query { 
        #[command(subcommand)]
        contract: Option<Q>,
    },

    /// Sets up the smart contract env with executes
    SetUp {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',')]
        contracts: Vec<C>,
    },

    /// Enables interactive mode
    Interactive { },
}
use clap::{Parser, Subcommand};
use cosm_utils::chain::coin::Denom;
use std::fmt::Debug;
use strum::IntoEnumIterator;

use crate::contract::Deploy;

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(rename_all = "kebab_case", infer_subcommands = true)]
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
#[clap(rename_all = "kebab_case", infer_subcommands = true)]
pub enum Commands<C, S>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    /// Rebuilds deploy
    #[command(visible_alias = "u")]
    Update {
        /// Select which features to enable on wasm-deploy
        /// Defaults to the currently enabled features
        #[arg(short, long, use_value_delimiter = true, value_delimiter = ',')]
        features: Option<Vec<String>>,
    },

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

        /// Triggers dialogue to display as address
        #[arg(short, long, exclusive = true)]
        show: bool,
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
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,

        /// Deploys but does not recompile first
        #[arg(short, long, required = false)]
        no_build: bool,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
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

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
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

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
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

        /// Deploys but does not recompile first
        #[arg(short, long, required = false)]
        no_build: bool,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Sets the config of a contract
    SetConfig {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Executes a contract
    #[command(visible_alias = "x")]
    Execute {
        #[command(subcommand)]
        contract: C,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Sends Cw20 tokens to a contract along with a payload
    Cw20Send {
        #[command(subcommand)]
        contract: C,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Executes a Cw20 message
    Cw20Execute {
        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Queries a Cw20 contract
    Cw20Query {
        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Instantiate a Cw20 contract
    Cw20Instantiate {
        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Executes a contract with a custom payload
    ExecutePayload {
        #[arg(short, long)]
        address: String,

        #[arg(short, long)]
        payload: String,
    },

    /// Queries a contract with a custom payload
    QueryPayload {
        #[arg(short, long)]
        address: String,

        #[arg(short, long)]
        payload: String,
    },

    /// Executes a user defined command
    #[command(flatten)]
    Custom(S),

    /// Sends a query to a contract
    #[command(visible_alias = "q")]
    Query {
        #[command(subcommand)]
        contract: C,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },

    /// Sends a token amount to a given address
    Send {
        /// Address to receive the tokens
        #[arg(long)]
        address: String,

        /// The amount and denom to send
        #[arg(long)]
        denom: Denom,

        /// The amount and denom to send
        #[arg(long)]
        amount: u128,
    },

    /// Sets up the smart contract env with executes
    SetUp {
        /// Name of the contract
        #[arg(short, long, use_value_delimiter=true, value_delimiter=',', default_values=get_all::<C>())]
        contracts: Vec<C>,

        /// Does not execute transactions, prints txs to console
        #[arg(short, long, required = false)]
        dry_run: bool,
    },
}

fn get_all<C: Deploy + IntoEnumIterator>() -> Vec<String> {
    C::iter().map(|x| x.to_string()).collect()
}

#[derive(Subcommand, Clone, Debug)]
pub enum EmptySubcommand {}

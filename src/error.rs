use std::{error::Error, process::ExitStatusError};

use cosm_orc::client::{chain_res::ChainResponse, error::ClientError};
use cosmos_sdk_proto::prost::{DecodeError, EncodeError};
use cosmrs::ErrorReport;
use inquire::InquireError;
use interactive_parse::error::SchemaError;
use ledger_utility::error::LedgerUtilityError;
use thiserror::Error;

pub type DeployResult<T> = core::result::Result<T, DeployError>;

#[derive(Error, Debug)]
pub enum DeployError {
    #[error("{0}")]
    Error(String),

    #[error("{0}")]
    Generic(String),

    #[error(transparent)]
    Keyring(#[from] keyring::Error),

    #[error("{0}")]
    DecodeError(#[from] DecodeError),

    #[error("{0}")]
    ClientError(#[from] ClientError),

    #[error("{0}")]
    EncodeError(#[from] EncodeError),

    #[error("{0}")]
    ExitStatus(#[from] ExitStatusError),

    #[error("{0}")]
    LedgerUtilityError(#[from] LedgerUtilityError),

    #[error("invalid admin address")]
    AdminAddress,

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    InteractiveParse(#[from] SchemaError),

    #[error("{0}")]
    Std(#[from] Box<dyn Error>),

    #[error("{0}")]
    Inquire(#[from] InquireError),

    #[error("{0}")]
    Bip32(#[from] cosmrs::bip32::Error),

    #[error("{0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    ErrorReport(#[from] ErrorReport),

    #[error("{0}")]
    RpcError(#[from] tendermint_rpc::Error),

    #[error("{0}")]
    Clap(#[from] clap::error::Error),

    #[error("invalid mnemonic")]
    Mnemonic,

    #[error("invalid derivation path")]
    DerivationPath,

    #[error("invalid instantiate permissions")]
    InstantiatePerms { source: ErrorReport },

    #[error("cryptographic error")]
    Crypto { source: ErrorReport },

    #[error("Cosmos Sdk Error")]
    AccountId { id: String },

    #[error("Cosmos Sdk Error")]
    CosmosSdk { res: ChainResponse },

    #[error("proto encoding error")]
    ProtoEncoding { source: ErrorReport },

    #[error("proto decoding error")]
    ProtoDecoding { source: ErrorReport },

    #[error("Unsupported shell, must use bash or zsh")]
    UnsupportedShell,

    #[error("Chain already exists")]
    ChainAlreadyExists,

    #[error("Contract already exists")]
    ContractAlreadyExists,

    #[error("Contract not found")]
    ContractNotFound,

    #[error("Env already exists")]
    EnvAlreadyExists,

    #[error("Invalid directory")]
    InvalidDir,

    #[error("Contract does not have an address")]
    NoAddr,

    #[error("Error parsing chain")]
    ChainId { chain_id: String },

    #[error("Error parsing denom")]
    Denom { name: String },

    #[error("Empty response")]
    EmptyResponse,

    #[error("Key already exists")]
    KeyAlreadyExists,

    #[error("Key not found")]
    KeyNotFound { key_name: String },

    #[error("Code id not found")]
    CodeIdNotFound,

    #[error("Env not found")]
    EnvNotFound,

    #[error("Contract address not found")]
    AddrNotFound,

    #[error("{} Config file not found, perhaps you need to run \"deploy init\"?", "Deploy Error")]
    ConfigNotFound {},
}

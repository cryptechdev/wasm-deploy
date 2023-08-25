#[cfg(feature = "ledger")]
use ledger_utility::error::LedgerUtilityError;
use thiserror::Error;

pub type DeployResult<T> = core::result::Result<T, DeployError>;

#[derive(Error, Debug)]
pub enum DeployError {
    #[error("{0}")]
    Generic(String),

    #[error("Unsupported shell, must use bash or zsh")]
    UnsupportedShell,

    #[error("Chain already exists, If you means to do this, delete the existing chain first with `chain -d`")]
    ChainAlreadyExists,

    #[error("Contract already exists")]
    ContractAlreadyExists,

    #[error("Contract {contract_name} not found, consider running \"store_code\"")]
    ContractNotFound { contract_name: String },

    #[error("Env already exists")]
    EnvAlreadyExists,

    #[error("Invalid directory")]
    InvalidDir,

    #[error("Key already exists")]
    KeyAlreadyExists,

    #[error("Key not found")]
    KeyNotFound { key_name: String },

    #[error("Code id not found, consider running \"store_code\"")]
    CodeIdNotFound,

    #[error("Env not found")]
    EnvNotFound,

    #[error("Chain config not found")]
    ChainConfigNotFound,

    #[error("Contract address not found for {name}, consider running \"instantiate\"")]
    AddrNotFound { name: String },

    #[error(
        "{} Config file not found, consider running \"deploy init\"?",
        "Deploy Error"
    )]
    ConfigNotFound {},

    #[error(
        "Both gRPC endpoint and RPC endpoint cannot be null. \
        Update you ChainInfo to add at least one endpoint"
    )]
    MissingClient,

    #[error(
        "The current version of wasm-deploy requires the gRPC endpoint. \
        Update you ChainInfo to include the endpoint address"
    )]
    MissingGRpc,

    #[error(
        "The current version of wasm-deploy requires the RPC endpoint. \
        Update you ChainInfo to include the endpoint address"
    )]
    MissingRpc,

    #[error(
        "This feature has not been implemented for this contract.\
     Implement the relevant trait and try again."
    )]
    TraitNotImplemented,

    #[error("WorkspaceSettings are not initialized")]
    SettingsUninitialized,

    #[error("Response received from client was empty")]
    EmptyResponse,
}

#[cfg(test)]
mod test {
    use super::DeployError;

    fn test_send_sync<T: Send + Sync>(_: T) {}
    #[test]
    fn test_deploy_error() {
        test_send_sync(DeployError::Generic("".to_string()));
    }
}

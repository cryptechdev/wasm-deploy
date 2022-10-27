use std::{process::ExitStatusError, error::Error};

use inquire::InquireError;
use thiserror::Error;

pub type DeployResult<T> = core::result::Result<T, DeployError>;

#[derive(Error, Debug)]
pub enum DeployError {
    #[error("{0}")]
    Error(String),

    #[error("{0}")]
    Generic(String),

    #[error("{0}")]
    ExitStatus(#[from] ExitStatusError),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Std(#[from] Box<dyn Error>),

    #[error("{0}")]
    Inquire(#[from] InquireError),

    #[error("{0}")]
    Bip32(#[from] cosmrs::bip32::Error),

    #[error("{0}")]
    Serde(#[from] serde_json::Error),

    

    
    
    // #[error("{0}")]
    // OverflowError(#[from] OverflowError),

    // #[error("{0}")]
    // Common(#[from] CommonError),

    // #[error("{0}")]
    // Auth(#[from] NeptuneAuthorizationError),

    #[error("Unsupported shell, must use bash or zsh")]
    UnsupportedShell {},

    #[error("Chain already exists")]
    ChainAlreadyExists {},

    #[error("Env already exists")]
    EnvAlreadyExists {},

    #[error("Invalid directory")]
    InvalidDir {},
}

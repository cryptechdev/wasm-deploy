use std::{
    fmt::{Debug, Display},
    path::PathBuf,
    str::FromStr,
};

use crate::error::DeployError;
use serde::Serialize;
use strum::{IntoEnumIterator, ParseError};

pub trait Msg: Debug + Send + Sync + erased_serde::Serialize {}

impl<T> Msg for T where T: Debug + Serialize + Send + Sync {}

impl Serialize for dyn Msg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        erased_serde::serialize(self, serializer)
    }
}

/// This trait represents a contract that can be deployed.
pub trait Contract:
    Send + Sync + Debug + Display + FromStr<Err = ParseError> + IntoEnumIterator + 'static
{
    /// This is the name of the contract and represents
    /// how it will appear in the cli.
    fn name(&self) -> String;

    /// This is the address of the contract admin.
    /// It is required when instantiating.
    fn admin(&self) -> String;

    /// This method allows executing a contract.
    /// interactive-parse should be used to generate the msg.
    fn execute(&self) -> anyhow::Result<Box<dyn Msg>> {
        Err(DeployError::TraitNotImplemented.into())
    }

    /// This method allows querying a contract.
    /// interactive-parse should be used to generate the msg.
    fn query(&self) -> anyhow::Result<Box<dyn Msg>> {
        Err(DeployError::TraitNotImplemented.into())
    }

    /// This method allows sending a cw20 token with an attached message to a contract.
    /// interactive-parse should be used to generate the msg.
    fn cw20_send(&self) -> anyhow::Result<Box<dyn Msg>> {
        Err(DeployError::TraitNotImplemented.into())
    }

    /// This method gets the preprogrammed instantiate msg for the contract.
    fn instantiate_msg(&self) -> Option<Box<dyn Msg>>;

    /// This method will instantiate an external contract via code_id alongside a local contract.
    fn external_instantiate_msgs(&self) -> Vec<ExternalInstantiate<Box<dyn Msg>>> {
        vec![]
    }

    /// This method gets the preprogrammed migrate msg for the contract.
    fn migrate_msg(&self) -> Option<Box<dyn Msg>> {
        None
    }

    /// This method gets the preprogrammed set config msg for the contract.
    fn set_config_msg(&self) -> Option<Box<dyn Msg>> {
        None
    }

    /// This method gets the preprogrammed set up for the contract.
    fn set_up_msgs(&self) -> Vec<Box<dyn Msg>> {
        vec![]
    }

    /// This method allows for customizing the path to the contract.
    /// This should be the path relative to the project root.
    fn path(&self) -> PathBuf {
        PathBuf::from(format!("contracts/{}", self.name()))
    }
}

#[derive(Debug, Clone)]
pub struct ExternalInstantiate<T> {
    pub msg: T,
    pub code_id: u64,
    pub name: String,
}

impl<T> From<ExternalInstantiate<T>> for ExternalInstantiate<Box<dyn Msg>>
where
    T: Msg + Clone + 'static,
{
    fn from(msg: ExternalInstantiate<T>) -> Self {
        ExternalInstantiate {
            msg: Box::new(msg.msg),
            code_id: msg.code_id,
            name: msg.name,
        }
    }
}

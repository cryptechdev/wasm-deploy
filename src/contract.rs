use std::{fmt::{Debug, Display}};
use clap::{Parser, Subcommand};
use strum::IntoEnumIterator;

use crate::{wasm_cli::{wasm_cli_instantiate, wasm_cli_execute, wasm_cli_migrate, wasm_cli_query, wasm_cli_instantiate_with_code_id, wasm_cli_execute_silent}, error::DeployError};

pub trait Contract: Send + Sync + Debug + From<String> + IntoEnumIterator + Display + Clone + 'static {
    fn name(&self)                      -> String;
    fn admin(&self)                     -> String;
    fn instantiate_msg(&self)           -> Result<String, DeployError>;
    fn external_instantiate_msgs(&self) -> Result<Vec<ExternalInstantiate>, DeployError>;
    fn base_config_msg(&self)           -> Result<String, DeployError>;
    fn execute_msg(&self)               -> Result<String, DeployError>;
    fn query_msg(&self)                 -> Result<String, DeployError>;
    fn set_up_msgs(&self)               -> Result<Vec<String>, DeployError>;
}

pub fn execute_instantiate(contract: &impl Contract) -> Result<(), DeployError> {
    wasm_cli_instantiate(&contract.admin(), &contract.name(), &contract.instantiate_msg()?)?;
    for external in contract.external_instantiate_msgs()? {
        wasm_cli_instantiate_with_code_id(&contract.admin(), &external.name, external.code_id, &external.msg)?;
    }
    Ok(())
}
pub fn execute_migrate(contract: &impl Contract) -> Result<(), DeployError> {
    wasm_cli_migrate(&contract.name(), &contract.instantiate_msg()?)
}

pub fn execute_set_config(contract: &impl Contract) -> Result<(), DeployError> {
    wasm_cli_execute_silent(&contract.name(), &contract.base_config_msg()?)
}

pub fn execute_set_up(contract: &impl Contract) -> Result<(), DeployError> {
    for msg in contract.set_up_msgs()? {
        wasm_cli_execute_silent(&contract.name(), &msg)?;
    }
    Ok(())   
}

pub trait Execute: Parser + Subcommand + Display {
    fn execute_msg(&self) -> Result<String, DeployError>;
}

pub fn execute(contract: &impl Execute) -> Result<(), DeployError> {
    wasm_cli_execute(&contract.to_string(), &contract.execute_msg()?)
}

pub trait Query: Parser + Subcommand + Display {
    fn query_msg(&self) -> Result<String, DeployError>;
}

pub fn query(contract: &impl Query) -> Result<(), DeployError> {
    wasm_cli_query(&contract.to_string(), &contract.query_msg()?)
}

#[derive(Clone)]
pub struct ExternalInstantiate {
    pub msg: String,
    pub code_id: u64,
    pub name: String
}
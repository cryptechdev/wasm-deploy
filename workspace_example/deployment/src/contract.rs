use std::str::FromStr;

use interactive_parse::traits::InteractiveParseObj;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumIter, EnumString};
use wasm_deploy::{
    contract::{Contract, Cw20Hook, Execute, Query},
    error::{DeployError, DeployResult},
};

use crate::defaults::{ADMIN, CW20_INSTANTIATE, CW20_MINT};

#[derive(Clone, Debug, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
/// This is where you define the list of all contracts you want wasm-depoy to know about
pub enum Contracts {
    Cw20Base,
}

impl From<String> for Contracts {
    fn from(value: String) -> Self { Contracts::from_str(value.as_str()).expect("Error parsing contracts") }
}

impl Contract for Contracts {
    type Cw20HookMsg = Cw20HookCommand;
    type ExecuteMsg = ExecuteCommand;
    type QueryMsg = QueryCommand;

    fn name(&self) -> String { self.to_string() }

    fn admin(&self) -> String { ADMIN.to_string() }

    fn instantiate_msg(&self) -> DeployResult<Value> {
        match self {
            Contracts::Cw20Base { .. } => Ok(serde_json::to_value(CW20_INSTANTIATE.to_owned())?),
        }
    }

    fn config_msg(&self) -> DeployResult<Option<Value>> {
        match self {
            Contracts::Cw20Base { .. } => Ok(None),
        }
    }

    fn set_up_msgs(&self) -> Result<Vec<Value>, DeployError> {
        match self {
            Contracts::Cw20Base => Ok(CW20_MINT.iter().map(|x| serde_json::to_value(x).unwrap()).collect()),
        }
    }

    fn external_instantiate_msgs(&self) -> Result<Vec<wasm_deploy::contract::ExternalInstantiate>, DeployError> {
        match self {
            Contracts::Cw20Base => Ok(vec![]),
        }
    }
}

// Unfortunately ExecuteCommand and QueryCommand must
// be separated out to get a proper tx and q subcommand
#[derive(Clone, Serialize, Deserialize, Debug, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ExecuteCommand {
    /// Executes the price oracle contract
    Cw20Base { execute: cw20_base::msg::ExecuteMsg },
}

impl Execute for ExecuteCommand {
    fn execute_msg(&self) -> DeployResult<Value> {
        match self {
            ExecuteCommand::Cw20Base { execute } => Ok(serde_json::to_value(execute)?),
        }
    }

    fn parse(contract: &impl Contract) -> DeployResult<Self> {
        match contract.name().as_str() {
            "cw20_base" => Ok(Self::Cw20Base { execute: cw20_base::msg::ExecuteMsg::parse_to_obj()? }),
            _ => panic!("unknown contract"),
        }
    }
}

#[derive(Debug, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum QueryCommand {
    /// Queries the price oracle contract
    Cw20Base { query: cw20_base::msg::QueryMsg },
}

impl Query for QueryCommand {
    fn query_msg(&self) -> DeployResult<Value> {
        match self {
            QueryCommand::Cw20Base { query } => Ok(serde_json::to_value(query)?),
        }
    }

    fn parse(contract: &impl Contract) -> DeployResult<Self> {
        match contract.name().as_str() {
            "cw20_base" => Ok(Self::Cw20Base { query: cw20_base::msg::QueryMsg::parse_to_obj()? }),
            _ => panic!("unknown contract"),
        }
    }
}

/// This is to interacting with your contract if it implements the standard Cw20HookCommand
/// interface.
#[derive(Clone, Serialize, Deserialize, Debug, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Cw20HookCommand {
    /// This contract doesn't implement this interface
    Cw20Base { cw20_hook: () },
}

impl Cw20Hook for Cw20HookCommand {
    fn cw20_hook_msg(&self) -> DeployResult<Value> {
        match self {
            Cw20HookCommand::Cw20Base { .. } => Ok(serde_json::to_value(())?),
        }
    }

    fn parse(contract: &impl Contract) -> DeployResult<Self> {
        match contract.name().as_str() {
            "cw20_base" => Ok(Self::Cw20Base { cw20_hook: () }),
            _ => panic!("unknown contract"),
        }
    }
}

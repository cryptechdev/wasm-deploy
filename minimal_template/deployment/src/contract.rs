// This file defines your contract. It's mostly boiler plate.
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use interactive_parse::InteractiveParseObj;
use wasm_deploy::contract::{Contract, Msg};
use wasm_deploy::derive::contract;

use crate::defaults::{ADMIN, CW20_INSTANTIATE, CW20_MINT};

pub const ADMIN: &str = "{{admin}}";

/// This is where you define the list of all contracts you want wasm-deploy to know about
#[contract]
pub enum Contracts {
    // You should replace this with your contract name.
    MyContract,
    // You can add more contracts to this list
}

// Take a look at the Contract trait.
// There are a few default methods that you can override.
// Generally you'll want to match on the Contracts enum and handle the logic for each contract.
impl Contract for Contracts {
    // This is the address of the contract admin. It is required when instantiating.
    fn admin(&self) -> String {
        match self {
            Contracts::MyContract { .. } => ADMIN.to_string(),
        }
    }

    // This method allows executing a contract. interactive-parse should be used to generate the msg.
    fn execute(&self) -> anyhow::Result<Box<dyn Msg>> {
        match self {
            Contracts::MyContract { .. } => {
                // Insert your execute msg here.
                // Ok(Box::new(Cw20ExecuteMsg::parse_to_obj()?))
                todo!()
            }
        }
    }

    // This method allows querying a contract. interactive-parse should be used to generate the msg.
    fn query(&self) -> anyhow::Result<Box<dyn Msg>> {
        match self {
            Contracts::MyContract { .. } => {
                // Insert your query msg here.
                // Ok(Box::new(Cw20QueryMsg::parse_to_obj()?))},
                todo!()
        }
    }

    // This method gets the preprogrammed instantiate msg for the contract.
    fn instantiate_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::MyContract { .. } => {
                // Insert your instantiate msg here.
                // Some(Box::new(CW20_INSTANTIATE.to_owned()))},
                todo!()
        }
    }

    // This method gets the preprogrammed migrate msg for the contract.
    fn migrate_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::MyContract { .. } => {
                // Insert your instantiate msg here.
                // Some(Box::new(CW20_INSTANTIATE.to_owned()))},
                todo!()
        }
    }

    // Look at the workspace example for more complicated set up
}

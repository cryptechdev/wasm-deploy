// This file defines your contract. It's mostly boiler plate.
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use interactive_parse::traits::InteractiveParseObj;
use wasm_deploy::contract::{Contract, Msg};
use wasm_deploy::derive::contract;

use crate::defaults::{ADMIN, CW20_INSTANTIATE, CW20_MINT};

/// This is where you define the list of all contracts you want wasm-deploy to know about
#[contract]
pub enum Contracts {
    // Cw20Base is just an example.
    // You should replace it with your own contract.
    Cw20Base,
    // You can add more contracts to this list
}

// Take a look at the Contract trait.
// There are a few default methods that you can override.
// Generally you'll want to match on the Contracts enum and handle the logic for each contract.
impl Contract for Contracts {
    // This is the name of the contract and represents how it will appear in the cli.
    fn name(&self) -> String {
        match self {
            Contracts::Cw20Base { .. } => self.to_string(),
        }
    }

    // This is the address of the contract admin. It is required when instantiating.
    fn admin(&self) -> String {
        match self {
            Contracts::Cw20Base { .. } => ADMIN.to_string(),
        }
    }

    // This method allows executing a contract. interactive-parse should be used to generate the msg.
    fn execute(&self) -> anyhow::Result<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => Ok(Box::new(Cw20ExecuteMsg::parse_to_obj()?)),
        }
    }

    // This method allows querying a contract. interactive-parse should be used to generate the msg.
    fn query(&self) -> anyhow::Result<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => Ok(Box::new(Cw20QueryMsg::parse_to_obj()?)),
        }
    }

    // This method gets the preprogrammed instantiate msg for the contract.
    fn instantiate_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => Some(Box::new(CW20_INSTANTIATE.to_owned())),
        }
    }

    // This method gets the preprogrammed migrate msg for the contract.
    fn migrate_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => Some(Box::new(CW20_INSTANTIATE.to_owned())),
        }
    }

    // This method gets the preprogrammed set up msgs for the contract.
    fn set_up_msgs(&self) -> Vec<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base => CW20_MINT
                .iter()
                .map(|x| Box::new(x.clone()) as Box<dyn Msg>)
                .collect(),
        }
    }
}

// This file defines your contract. It's mostly boiler plate.
use crate::defaults::{ADMIN, CW20_INSTANTIATE, CW20_MINT};
use wasm_deploy::contract::{Deploy, Msg};
use wasm_deploy::derive::contracts;

/// This is where you define the list of all contracts you want wasm-deploy to know about
/// This attribute macro will generate a bunch of code for you.
/// Simply create an enum with variants for each contract.
#[contracts]
pub enum Contracts {
    // Cw20Base is just an example.
    // You should replace it with your own contract.
    #[contract(
        // This field must be a string literal or constant
        admin = ADMIN,
        // These fields should be paths to the types
        instantiate = cw20_base::msg::InstantiateMsg,
        execute = cw20_base::msg::ExecuteMsg,
        query = cw20_base::msg::QueryMsg,
        // The snake case name is technically cw_20_base
        // but we will rename it to match the library name
        rename = "cw20_base"
        // cw20_send = ...             
        // migrate = ...                
        // bin_name = "cw20"             // | You should only need to change these
        // path = "contracts/cw20_base"  // | if you have a non-standard workspace layout.

    )]
    Cw20Base,
    // You can add more contracts to this list
}

// Take a look at the Contract trait.
// There are a few default methods that you can override.
// These are preprogrammed messages for the various stages of deployment.
// Generally you'll want to match on the Contracts enum and handle the logic for each contract.
impl Deploy for Contracts {
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

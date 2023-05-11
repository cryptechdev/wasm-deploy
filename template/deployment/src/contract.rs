// This file defines your contract. It's mostly boiler plate.
use wasm_deploy::contract::Deploy;
use wasm_deploy::derive::contracts;

pub const ADMIN: &str = "{{admin}}";

/// This is where you define the list of all contracts you want wasm-deploy to know about
/// This attribute macro will generate a bunch of code for you.
/// Simply create an enum with variants for each contract.
#[contracts]
pub enum Contracts {
    // Cw20Base is just an example.
    // You should replace it with your own contract.
    #[contract(
        admin = ADMIN,
        instantiate = InstantiateMsg,
        execute = ExecuteMsg,
        query = QueryMsg
        // cw20_send = ...             
        // migrate = ...                
        // rename = "cw20"               // | You should only need to change these
        // bin_name = "cw20"             // | three ff you have a non-standard workspace
        // path = "contracts/cw20_base"  // | layout.

    )]
    MyContract,
    // You can add more contracts to this list
}

// Take a look at the Deploy trait.
// There are a few default methods that you can override.
// These apply for have preprogrammed messages for the various stages of deployment.
// Generally you'll want to match on the Contracts enum and handle the logic for each contract.
// You'll also likely want to use lazy_static to create the messages you need.
impl Deploy for Contracts {
    // // This method gets the preprogrammed instantiate msg for the contract.
    // fn instantiate_msg(&self) -> Option<Box<dyn Msg>> {
    //     match self {
    //         Contracts::MyContract { .. } => Some(Box::new(INSTANTIATE.to_owned())),
    //     }
    // }

    // // This method gets the preprogrammed migrate msg for the contract.
    // fn migrate_msg(&self) -> Option<Box<dyn Msg>> {
    //     match self {
    //         Contracts::MyContract { .. } => Some(Box::new(INSTANTIATE.to_owned())),
    //     }
    // }

    // // This method gets the preprogrammed set up msgs for the contract.
    // fn set_up_msgs(&self) -> Vec<Box<dyn Msg>> {
    //     match self {
    //         Contracts::MyContract => CW20_MINT
    //             .iter()
    //             .map(|x| Box::new(x.clone()) as Box<dyn Msg>)
    //             .collect(),
    //     }
    // }
}

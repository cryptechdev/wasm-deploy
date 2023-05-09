// Use this file to define the various default message you want deploy to use
use cw20::MinterResponse;
use lazy_static::lazy_static;
use wasm_deploy::{
    contract::{Contract, ExternalInstantiate},
    utils::{get_addr, get_code_id},
};

use crate::contract::Contracts;

// Using lazy_static helps us create the messages that we need for the various deployment stages.
lazy_static! {
// Enter all your instantiate and set up messages here.
}

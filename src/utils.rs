use serde_json::Value;

use crate::{
    error::{DeployError, DeployResult},
    file::ContractInfo,
};

pub fn replace_strings(value: &mut Value, contracts: &Vec<ContractInfo>) -> DeployResult<()> {
    match value {
        Value::Null => {}
        Value::Bool(_) => {}
        Value::Number(_) => {}
        Value::String(string) => {
            if let Some((_, new)) = string.split_once('&') {
                if let Some(contract) = contracts.iter().find(|x| x.name == new) {
                    match &contract.addr {
                        Some(addr) => *string = addr.clone(),
                        None => return Err(DeployError::AddrNotFound),
                    }
                }
            }
        }
        Value::Array(array) => {
            for value in array {
                replace_strings(value, contracts)?;
            }
        }
        Value::Object(map) => {
            for (_, value) in map {
                replace_strings(value, contracts)?;
            }
        }
    }
    Ok(())
}
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{error::DeployError, file::ContractInfo};

pub fn replace_strings(value: &mut Value, contracts: &Vec<ContractInfo>) -> anyhow::Result<()> {
    match value {
        Value::String(string) => {
            if let Some((_, new)) = string.split_once('&') {
                if let Some(contract) = contracts.iter().find(|x| x.name == new) {
                    match &contract.addr {
                        Some(addr) => *string = addr.clone(),
                        None => {
                            return Err(DeployError::AddrNotFound {
                                name: contract.name.clone(),
                            }
                            .into())
                        }
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
        _ => {}
    }
    Ok(())
}

pub fn replace_strings_any<T: Serialize + DeserializeOwned + Clone>(
    object: &mut T,
    contracts: &Vec<ContractInfo>,
) -> anyhow::Result<()> {
    let mut value = serde_json::to_value(object.clone())?;
    replace_strings(&mut value, contracts)?;
    *object = serde_json::from_value(value)?;
    Ok(())
}

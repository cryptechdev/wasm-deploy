use crate::{
    config::{ChainInfo, ContractInfo, Env, WorkspaceSettings, CONFIG, WORKSPACE_SETTINGS},
    error::DeployError,
};
use colored::Colorize;
use futures::executor::block_on;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tendermint_rpc::endpoint::broadcast::tx_commit;

lazy_static! {
    pub static ref BIN_NAME: String = std::env::current_exe()
        .unwrap()
        .file_stem()
        .unwrap()
        .to_owned()
        .into_string()
        .unwrap();
}

pub fn replace_strings(value: &mut Value, contracts: &Vec<ContractInfo>) -> anyhow::Result<()> {
    match value {
        Value::String(string) => {
            if let Some((_, new)) = string.split_once('&') {
                if let Some(contract) = contracts.iter().find(|x| x.name == new) {
                    match &contract.addr {
                        Some(addr) => string.clone_from(addr),
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

/// TODO: perhaps do this differently
pub fn replace_strings_any<T: Serialize + DeserializeOwned + Clone>(
    object: &mut T,
    contracts: &Vec<ContractInfo>,
) -> anyhow::Result<()> {
    let mut value = serde_json::to_value(object.clone())?;
    replace_strings(&mut value, contracts)?;
    *object = serde_json::from_value(value)?;
    Ok(())
}

pub async fn get_settings() -> anyhow::Result<Arc<WorkspaceSettings>> {
    match WORKSPACE_SETTINGS.read().await.clone() {
        Some(settings) => Ok(settings),
        None => Err(DeployError::SettingsUninitialized.into()),
    }
}

pub fn get_code_id(contract_name: &str) -> anyhow::Result<u64> {
    let config = block_on(CONFIG.read());
    Ok(config
        .get_contract(contract_name)?
        .code_id
        .ok_or(DeployError::CodeIdNotFound)?)
}

pub fn get_addr(contract_name: &str) -> anyhow::Result<String> {
    let config = block_on(CONFIG.read());
    Ok(config
        .get_contract(contract_name)?
        .addr
        .clone()
        .ok_or(DeployError::AddrNotFound {
            name: contract_name.to_string(),
        })?)
}

pub fn get_env() -> anyhow::Result<Env> {
    let config = block_on(CONFIG.read());
    Ok(config.get_active_env()?.clone())
}

pub fn get_chain() -> anyhow::Result<ChainInfo> {
    let config = block_on(CONFIG.read());
    Ok(config.get_active_chain_info()?.clone())
}

pub fn get_wallet_addr() -> anyhow::Result<String> {
    block_on(async {
        let config = CONFIG.read().await;
        let key = config.get_active_key().await?;
        let chain_info = config.get_active_chain_info()?;
        Ok(key
            .to_addr(&chain_info.cfg.prefix, &chain_info.cfg.derivation_path)
            .await?
            .to_string())
    })
}

pub fn print_res(tx_commit: tx_commit::Response) {
    println!(
        "gas wanted: {}, gas used: {}",
        tx_commit.deliver_tx.gas_wanted.to_string().green(),
        tx_commit.deliver_tx.gas_used.to_string().green()
    );
    println!("tx hash: {}", tx_commit.hash.to_string().purple());
}

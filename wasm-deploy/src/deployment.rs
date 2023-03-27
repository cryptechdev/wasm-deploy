use std::str::FromStr;

use colored::Colorize;
use cosm_tome::{
    chain::{request::TxOptions, response::ChainTxResponse},
    clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
    modules::{
        auth::model::Address,
        cosmwasm::model::{ExecRequest, InstantiateRequest, MigrateRequest, StoreCodeRequest},
    },
};

use crate::{
    contract::Contract,
    error::{DeployError, DeployResult},
    file::{Config, ContractInfo},
    utils::replace_strings,
};

pub enum DeploymentStage {
    StoreCode,
    Instantiate,
    ExternalInstantiate,
    Migrate,
    SetConfig,
    SetUp,
}

pub async fn execute_deployment(
    contracts: &[impl Contract],
    // TODO: perhaps accept &[DeploymentStage]
    deployment_stage: DeploymentStage,
) -> DeployResult<()> {
    let mut config = Config::load()?;
    let chain_info = config.get_active_chain_info()?;
    let key = config.get_active_key().await?;

    // TODO: maybe impl http here, maybe not required
    let Some(rpc_endpoint) = chain_info.rpc_endpoint.clone() else {
        return Err(DeployError::MissingGRpc);
    };

    let client = TendermintRPC::new(&rpc_endpoint)?;
    let cosm_tome = CosmTome::new(chain_info, client);
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
    };

    let response: Option<ChainTxResponse> = match deployment_stage {
        DeploymentStage::StoreCode => {
            let mut reqs = vec![];
            for contract in contracts {
                println!("Storing code for {}", contract.name());
                let path = format!("./artifacts/{}.wasm", contract.name());
                let wasm_data = std::fs::read(path)?;
                reqs.push(StoreCodeRequest {
                    wasm_data,
                    instantiate_perms: None,
                });
            }
            let response = cosm_tome.wasm_store_batch(reqs, &key, &tx_options).await?;

            for (i, contract) in contracts.iter().enumerate() {
                match config.get_contract(&contract.to_string()) {
                    Ok(contract_info) => contract_info.code_id = Some(response.code_ids[i]),
                    Err(_) => {
                        config.add_contract_from(ContractInfo {
                            name: contract.name(),
                            addr: None,
                            code_id: Some(response.code_ids[i]),
                        })?;
                    }
                }
            }
            config.save()?;
            Some(response.res)
        }
        DeploymentStage::Instantiate => {
            let mut reqs = vec![];
            let tx_options = TxOptions {
                timeout_height: None,
                fee: None,
                memo: "wasm_deploy".into(),
            };
            for contract in contracts {
                if let Some(msg) = contract.instantiate_msg() {
                    println!("Instantiating {}", contract.name());
                    let mut value = serde_json::to_value(msg)?;
                    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
                    let contract_info = config.get_contract(&contract.to_string())?;
                    let code_id = contract_info.code_id.ok_or(DeployError::CodeIdNotFound)?;
                    reqs.push(InstantiateRequest {
                        code_id,
                        msg: value,
                        label: contract.name(),
                        admin: Some(Address::from_str(&contract.admin()).unwrap()),
                        funds: vec![],
                    });
                }
            }
            let response = cosm_tome
                .wasm_instantiate_batch(reqs, &key, &tx_options)
                .await?;
            for (index, contract) in contracts.iter().enumerate() {
                let contract_info = config.get_contract(&contract.to_string())?;
                contract_info.addr = Some(response.addresses[index].to_string());
            }
            config.save()?;
            Some(response.res)
        }
        DeploymentStage::ExternalInstantiate => {
            let mut reqs = vec![];
            let tx_options = TxOptions {
                timeout_height: None,
                fee: None,
                memo: "wasm_deploy".into(),
            };
            for contract in contracts {
                for external in contract.external_instantiate_msgs() {
                    println!("Instantiating {}", external.name);
                    let mut value = serde_json::to_value(external.msg)?;
                    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
                    reqs.push(InstantiateRequest {
                        code_id: external.code_id,
                        msg: value,
                        label: external.name.clone(),
                        admin: Some(Address::from_str(&contract.admin()).unwrap()),
                        funds: vec![],
                    });
                }
            }
            if reqs.is_empty() {
                None
            } else {
                let response = cosm_tome
                    .wasm_instantiate_batch(reqs, &key, &tx_options)
                    .await?;
                let mut index = 0;
                for contract in contracts {
                    for external in contract.external_instantiate_msgs() {
                        config.add_contract_from(ContractInfo {
                            name: external.name,
                            addr: Some(response.addresses[index].to_string()),
                            code_id: Some(external.code_id),
                        })?;
                        index += 1;
                    }
                }
                config.save()?;
                Some(response.res)
            }
        }
        DeploymentStage::SetConfig => {
            let mut reqs = vec![];
            for contract in contracts {
                if let Some(msg) = contract.set_config_msg() {
                    println!("Setting config for {}", contract.name());
                    let mut value = serde_json::to_value(msg)?;
                    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
                    let contract_addr =
                        config.get_contract_addr_mut(&contract.to_string())?.clone();
                    reqs.push(ExecRequest {
                        msg: value,
                        funds: vec![],
                        address: Address::from_str(&contract_addr).unwrap(),
                    });
                };
            }
            if reqs.is_empty() {
                None
            } else {
                let response = cosm_tome
                    .wasm_execute_batch(reqs, &key, &tx_options)
                    .await?;
                Some(response.res)
            }
        }
        DeploymentStage::SetUp => {
            let mut reqs = vec![];
            for contract in contracts {
                for (i, msg) in contract.set_up_msgs().into_iter().enumerate() {
                    if i == 0 {
                        println!("Executing Set Up for {}", contract.name());
                    }
                    let mut value = serde_json::to_value(msg)?;
                    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
                    let contract_addr =
                        config.get_contract_addr_mut(&contract.to_string())?.clone();
                    reqs.push(ExecRequest {
                        msg: value,
                        funds: vec![],
                        address: Address::from_str(&contract_addr).unwrap(),
                    });
                }
            }
            if reqs.is_empty() {
                None
            } else {
                let response = cosm_tome
                    .wasm_execute_batch(reqs, &key, &tx_options)
                    .await?;
                Some(response.res)
            }
        }
        DeploymentStage::Migrate => {
            let mut reqs = vec![];
            for contract in contracts {
                if let Some(msg) = contract.migrate_msg() {
                    println!("Migrating {}", contract.name());
                    let mut value = serde_json::to_value(msg)?;
                    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
                    let contract_info = config.get_contract(&contract.to_string())?;
                    let contract_addr =
                        contract_info
                            .addr
                            .clone()
                            .ok_or(DeployError::AddrNotFound {
                                name: contract_info.name.clone(),
                            })?;
                    let code_id = contract_info.code_id.ok_or(DeployError::CodeIdNotFound)?;
                    reqs.push(MigrateRequest {
                        msg: value,
                        address: Address::from_str(&contract_addr).unwrap(),
                        new_code_id: code_id,
                    });
                }
            }
            let response = cosm_tome
                .wasm_migrate_batch(reqs, &key, &tx_options)
                .await?;
            Some(response.res)
        }
    };
    if let Some(res) = response {
        println!(
            "gas wanted: {}, gas used: {}",
            res.gas_wanted.to_string().green(),
            res.gas_used.to_string().green()
        );
        println!("tx hash: {}", res.tx_hash.purple());
    }

    Ok(())
}

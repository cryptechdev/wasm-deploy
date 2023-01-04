use std::str::FromStr;

use colored::Colorize;
use cosm_tome::{
    chain::{request::TxOptions, response::ChainTxResponse},
    clients::{client::CosmTome, cosmos_grpc::CosmosgRPC},
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
    SetConfig,
    SetUp,
    Migrate,
}

pub async fn msg_contract(
    contracts: &[impl Contract],
    msg_type: DeploymentStage,
) -> DeployResult<()> {
    let mut config = Config::load()?;
    let chain_info = config.get_active_chain_info()?;
    let key = config.get_active_key().await?;

    // TODO: maybe impl http here, maybe not required
    let Some(grpc_endpoint) = chain_info.grpc_endpoint.clone() else {
        return Err(DeployError::MissingGRpc);
    };

    let client = CosmosgRPC::new(grpc_endpoint);
    let cosm_tome = CosmTome::new(chain_info, client);
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
    };

    let response: Option<ChainTxResponse> = match msg_type {
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
                println!("Instantiating {}", contract.name());
                let mut msg = contract.instantiate_msg()?;
                replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
                let contract_info = config.get_contract(&contract.to_string())?;
                let code_id = contract_info.code_id.ok_or(DeployError::CodeIdNotFound)?;
                reqs.push(InstantiateRequest {
                    code_id,
                    msg,
                    label: contract.name(),
                    admin: Some(Address::from_str(&contract.admin()).unwrap()),
                    funds: vec![],
                });
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
                for mut external in contract.external_instantiate_msgs()? {
                    println!("Instantiating {}", external.name);
                    replace_strings(&mut external.msg, &config.get_active_env()?.contracts)?;
                    reqs.push(InstantiateRequest {
                        code_id: external.code_id,
                        msg: external.msg,
                        label: external.name.clone(),
                        admin: Some(Address::from_str(&contract.admin()).unwrap()),
                        funds: vec![],
                    });
                }
            }
            let response = cosm_tome
                .wasm_instantiate_batch(reqs, &key, &tx_options)
                .await?;
            let mut index = 0;
            for contract in contracts {
                for external in contract.external_instantiate_msgs()? {
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
        DeploymentStage::SetConfig => {
            let mut reqs = vec![];
            for contract in contracts {
                println!("Setting config for {}", contract.name());
                let Some(mut msg) = contract.config_msg()? else { return Ok(()) };
                replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
                let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
                reqs.push(ExecRequest {
                    msg,
                    funds: vec![],
                    address: Address::from_str(&contract_addr).unwrap(),
                });
            }
            let response = cosm_tome
                .wasm_execute_batch(reqs, &key, &tx_options)
                .await?;
            Some(response.res)
        }
        DeploymentStage::SetUp => {
            let mut reqs = vec![];
            for contract in contracts {
                println!("Executing Set Up for {}", contract.name());
                for mut msg in contract.set_up_msgs()? {
                    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
                    let contract_addr =
                        config.get_contract_addr_mut(&contract.to_string())?.clone();
                    reqs.push(ExecRequest {
                        msg,
                        funds: vec![],
                        address: Address::from_str(&contract_addr).unwrap(),
                    });
                }
            }
            let response = cosm_tome
                .wasm_execute_batch(reqs, &key, &tx_options)
                .await?;
            Some(response.res)
        }
        DeploymentStage::Migrate => {
            let mut reqs = vec![];
            for contract in contracts {
                if let Some(mut msg) = contract.migrate_msg()? {
                    println!("Migrating {}", contract.name());
                    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
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
                        msg,
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

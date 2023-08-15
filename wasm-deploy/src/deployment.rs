use std::str::FromStr;

use colored::Colorize;
use colored_json::to_colored_json_auto;
use cosm_utils::{
    chain::request::TxOptions,
    modules::{
        auth::model::Address,
        cosmwasm::model::{ExecRequest, InstantiateRequest, MigrateRequest, StoreCodeRequest},
    },
    prelude::*,
};
use log::debug;
use tendermint_rpc::{endpoint::broadcast::tx_commit, HttpClient};

use crate::{
    config::{ContractInfo, WorkspaceSettings, CONFIG},
    contract::Deploy,
    error::DeployError,
    utils::print_res,
};

pub enum DeploymentStage {
    StoreCode,
    Instantiate { interactive: bool },
    ExternalInstantiate,
    Migrate { interactive: bool },
    SetConfig,
    SetUp,
}

pub async fn execute_deployment(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    dry_run: bool,
    deployment_stage: DeploymentStage,
) -> anyhow::Result<()> {
    let config = CONFIG.read().await;
    let chain_info = config.get_active_chain_info()?.clone();
    let key = config.get_active_key().await?;
    let rpc_endpoint = chain_info.rpc_endpoint.clone();
    drop(config);
    let client = HttpClient::get_persistent_compat(rpc_endpoint.as_str()).await?;

    let response: Option<tx_commit::Response> = match deployment_stage {
        DeploymentStage::StoreCode => {
            let mut reqs = vec![];
            for contract in contracts {
                println!(
                    "{} code for {}",
                    "     Storing".bright_green().bold(),
                    contract.name()
                );
                let path = settings
                    .artifacts_dir
                    .join(format!("{}.wasm.gz", contract.bin_name()));
                let wasm_data = std::fs::read(path)?;
                reqs.push(StoreCodeRequest {
                    wasm_data,
                    instantiate_perms: None,
                });
            }
            if dry_run {
                println!(
                    "{}",
                    to_colored_json_auto(&serde_json::to_value(reqs.into_iter().map(|m| m.wasm_data).collect::<Vec<_>>())?)?
                );
                None
            } else {
                let response = client
                    .wasm_store_batch_commit(&chain_info.cfg, reqs, &key, &TxOptions::default())
                    .await?;

                let mut config = CONFIG.write().await;
                for (i, contract) in contracts.iter().enumerate() {
                    match config.get_contract_mut(&contract.to_string()) {
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
                config.save(settings)?;
                Some(response.res)
            }
        }
        DeploymentStage::Instantiate { interactive } => {
            let mut reqs = vec![];
            let config = CONFIG.read().await;
            let msgs = contracts
                .iter()
                .map(|x| {
                    let msg = if interactive {
                        Some(x.instantiate()?)
                    } else {
                        x.instantiate_msg()
                    };
                    anyhow::Result::Ok((x, msg))
                })
                .collect::<Result<Vec<_>, anyhow::Error>>()?;
            let has_msg = msgs
                .iter()
                .filter_map(|x| x.1.as_ref().map(|_| x.0))
                .collect::<Vec<_>>();
            for (contract, msg) in msgs {
                if let Some(msg) = msg {
                    println!(
                        "{} {}",
                        "Instantiating".bright_green().bold(),
                        contract.name()
                    );
                    let contract_info = config.get_contract(&contract.to_string())?;
                    let code_id = contract_info.code_id.ok_or(DeployError::CodeIdNotFound)?;
                    reqs.push(InstantiateRequest {
                        code_id,
                        msg,
                        label: contract.name(),
                        admin: Some(Address::from_str(&contract.admin())?),
                        funds: vec![],
                    });
                }
            }
            if dry_run {
                println!(
                    "{}",
                    to_colored_json_auto(&serde_json::to_value(reqs.into_iter().map(|m| m.msg).collect::<Vec<_>>())?)?
                );
                None
            } else {
                debug!("reqs: {:?}", reqs);
                let response = client
                    .wasm_instantiate_batch_commit(&chain_info.cfg, reqs, &key, &TxOptions::default())
                    .await?;
                drop(config);
                let mut config = CONFIG.write().await;
                for (index, contract) in has_msg.into_iter().enumerate() {
                    let contract_info = config.get_contract_mut(&contract.to_string())?;
                    contract_info.addr = Some(response.addresses[index].to_string());
                }
                config.save(settings)?;
                Some(response.res)
            }
        }
        DeploymentStage::ExternalInstantiate => {
            let mut reqs = vec![];
            let config = CONFIG.read().await;
            for contract in contracts {
                for external in contract.external_instantiate_msgs() {
                    println!(
                        "{} {}",
                        "Instantiating".bright_green().bold(),
                        external.name
                    );
                    reqs.push(InstantiateRequest {
                        code_id: external.code_id,
                        msg: external.msg,
                        label: external.name.clone(),
                        admin: Some(Address::from_str(&contract.admin())?),
                        funds: vec![],
                    });
                }
            }
            drop(config);
            if reqs.is_empty() {
                None
            } else if dry_run {
                println!(
                    "{}",
                    to_colored_json_auto(&serde_json::to_value(reqs.into_iter().map(|m| m.msg).collect::<Vec<_>>())?)?
                );
                None
            } else {
                let response = client
                    .wasm_instantiate_batch_commit(
                        &chain_info.cfg,
                        reqs,
                        &key,
                        &TxOptions::default(),
                    )
                    .await?;
                let mut index = 0;
                for contract in contracts {
                    for external in contract.external_instantiate_msgs() {
                        let mut config = CONFIG.write().await;
                        config.add_contract_from(ContractInfo {
                            name: external.name,
                            addr: Some(response.addresses[index].to_string()),
                            code_id: Some(external.code_id),
                        })?;
                        index += 1;
                    }
                }
                let config = CONFIG.read().await;
                config.save(settings)?;
                Some(response.res)
            }
        }
        DeploymentStage::SetConfig => {
            let mut reqs = vec![];
            let config = CONFIG.read().await;
            for contract in contracts {
                if let Some(msg) = contract.set_config_msg() {
                    println!(
                        "{} set_config for {}",
                        "   Executing".bright_green().bold(),
                        contract.name()
                    );
                    let contract_addr = config.get_contract_addr(&contract.to_string())?.clone();
                    reqs.push(ExecRequest {
                        msg,
                        funds: vec![],
                        address: Address::from_str(&contract_addr)?,
                    });
                };
            }
            if reqs.is_empty() {
                None
            } else if dry_run {
                println!(
                    "{}",
                    to_colored_json_auto(&serde_json::to_value(reqs.into_iter().map(|m| m.msg).collect::<Vec<_>>())?)?
                );
                None
            } else {
                debug!("reqs: {:?}", reqs);
                let response = client
                    .wasm_execute_batch_commit(&chain_info.cfg, reqs, &key, &TxOptions::default())
                    .await?;
                Some(response)
            }
        }
        DeploymentStage::SetUp => {
            let mut reqs = vec![];
            let config = CONFIG.read().await;
            for contract in contracts {
                for (i, msg) in contract.set_up_msgs().into_iter().enumerate() {
                    if i == 0 {
                        println!(
                            "{} Set Up for {}",
                            "   Executing".bright_green().bold(),
                            contract.name()
                        );
                    }
                    let contract_addr = config.get_contract_addr(&contract.to_string())?.clone();
                    reqs.push(ExecRequest {
                        msg,
                        funds: vec![],
                        address: Address::from_str(&contract_addr)?,
                    });
                }
            }
            if reqs.is_empty() {
                None
            } else if dry_run {
                println!(
                    "{}",
                    to_colored_json_auto(&serde_json::to_value(reqs.into_iter().map(|m| m.msg).collect::<Vec<_>>())?)?
                );
                None
            } else {
                debug!("reqs: {:?}", reqs);
                let response = client
                    .wasm_execute_batch_commit(&chain_info.cfg, reqs, &key, &TxOptions::default())
                    .await?;
                Some(response)
            }
        }
        DeploymentStage::Migrate { interactive } => {
            let mut reqs = vec![];
            let config = CONFIG.read().await;
            for contract in contracts {
                let msg = if interactive {
                    Some(contract.migrate()?)
                } else {
                    contract.migrate_msg()
                };
                if let Some(msg) = msg {
                    println!(
                        "{} {}",
                        "   Migrating".bright_green().bold(),
                        contract.name()
                    );
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
                        address: Address::from_str(&contract_addr)?,
                        new_code_id: code_id,
                    });
                }
            }
            
            if dry_run {
                println!(
                    "{}",
                    to_colored_json_auto(&serde_json::to_value(reqs.into_iter().map(|m| m.msg).collect::<Vec<_>>())?)?
                );
                None
            } else {
                debug!("reqs: {:?}", reqs);
                let response = client
                    .wasm_migrate_batch_commit(&chain_info.cfg, reqs, &key, &TxOptions::default())
                    .await?;
                Some(response)
            }
        }
    };

    if let Some(res) = response {
        debug!("response: {:?}", res);
        print_res(res);
    }

    Ok(())
}

use crate::{
    contract::Contract, error::DeployError, file::Config, settings::WorkspaceSettings,
    utils::replace_strings,
};
use colored::Colorize;
use cosm_tome::{
    chain::{coin::Coin, request::TxOptions},
    clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
    modules::{auth::model::Address, cosmwasm::model::ExecRequest},
};
use interactive_parse::traits::InteractiveParseObj;
use serde::Serialize;
use std::str::FromStr;

pub async fn execute_contract(
    settings: &WorkspaceSettings,
    contract: &impl Contract,
) -> anyhow::Result<()> {
    println!("Executing");
    let mut config = Config::load(settings)?;
    let msg = contract.execute()?;
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let funds = Vec::<Coin>::parse_to_obj()?;
    execute(&mut config, contract_addr, msg, funds).await?;
    Ok(())
}

pub async fn execute(
    config: &mut Config,
    addr: impl AsRef<str>,
    msg: impl Serialize,
    funds: Vec<Coin>,
) -> anyhow::Result<()> {
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(
        &chain_info
            .rpc_endpoint
            .clone()
            .ok_or(DeployError::MissingRpc)?,
    )?;
    let cosm_tome = CosmTome::new(chain_info, client);
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
    };
    let req = ExecRequest {
        msg: value,
        funds,
        address: Address::from_str(addr.as_ref()).unwrap(),
    };
    let response = cosm_tome.wasm_execute(req, &key, &tx_options).await?;
    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());
    Ok(())
}

use crate::{
    config::{Config, CONFIG},
    contract::Deploy,
};
use colored::Colorize;
use colored_json::to_colored_json_auto;
use cosm_utils::{
    chain::{coin::Coin, request::TxOptions},
    modules::{auth::model::Address, cosmwasm::model::ExecRequest},
    prelude::*,
};
use interactive_parse::InteractiveParseObj;
use log::debug;
use serde::Serialize;
use std::{fmt::Debug, str::FromStr};
use tendermint_rpc::HttpClient;

pub async fn execute_contract(contract: &impl Deploy, dry_run: bool) -> anyhow::Result<()> {
    println!("Executing");
    let msg = contract.execute()?;
    if dry_run {
        println!("{}", to_colored_json_auto(&serde_json::to_value(msg)?)?);
        return Ok(());
    }
    let config = CONFIG.read().await;
    let contract_addr = config.get_contract_addr(&contract.to_string())?.clone();
    let funds = Vec::<Coin>::parse_to_obj()?;
    execute(&config, contract_addr, msg, funds).await?;

    Ok(())
}

pub async fn execute(
    config: &Config,
    addr: impl AsRef<str>,
    msg: impl Serialize + Send + Debug,
    funds: Vec<Coin>,
) -> anyhow::Result<()> {
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?.clone();
    let client = HttpClient::get_persistent_compat(chain_info.rpc_endpoint.as_str()).await?;
    let req = ExecRequest {
        msg,
        funds,
        address: Address::from_str(addr.as_ref())?,
    };
    debug!("req: {:?}", req);
    let response = client
        .wasm_execute_commit(&chain_info.cfg, req, &key, &TxOptions::default())
        .await?;
    println!(
        "gas wanted: {}, gas used: {}",
        response.deliver_tx.gas_wanted.to_string().green(),
        response.deliver_tx.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.hash.to_string().purple());
    Ok(())
}

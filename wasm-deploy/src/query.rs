use std::{fmt::Debug, str::FromStr};

use colored_json::to_colored_json_auto;
use cosm_utils::{modules::auth::model::Address, prelude::*};
use cw20::Cw20QueryMsg;
use inquire::Text;
use interactive_parse::InteractiveParseObj;
use log::debug;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tendermint_rpc::HttpClient;

use crate::{
    config::{Config, CONFIG},
    contract::Deploy,
    utils::replace_strings_any,
};

pub async fn query_contract(contract: &impl Deploy, dry_run: bool) -> anyhow::Result<Value> {
    println!("Querying");
    let msg = contract.query()?;
    if dry_run {
        println!("{}", to_colored_json_auto(&serde_json::to_value(msg)?)?);
        Ok(Value::default())
    } else {
        let config = CONFIG.read().await;
        let addr = config.get_contract_addr(&contract.to_string())?.clone();
        let value = query(&config, addr, msg).await?;
        let color = to_colored_json_auto(&value)?;
        println!("{color}");
        Ok(value)
    }
}

pub async fn query(
    config: &Config,
    mut addr: impl AsRef<str> + Serialize + DeserializeOwned + Clone,
    msg: impl Serialize + Sync + Debug,
) -> anyhow::Result<Value> {
    replace_strings_any(&mut addr, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?.clone();
    let client = HttpClient::get_persistent_compat(chain_info.rpc_endpoint.as_str()).await?;
    debug!("msg: {:?}", msg);
    let response = client
        .wasm_query(Address::from_str(addr.as_ref())?, &msg, None)
        .await?;
    debug!("response: {:?}", response);
    let string = String::from_utf8(response.value.data)?;
    Ok(serde_json::from_str::<Value>(string.as_str())?)
}

pub async fn cw20_query(dry_run: bool) -> anyhow::Result<Value> {
    println!("Querying cw20");
    let msg = Cw20QueryMsg::parse_to_obj()?;
    if dry_run {
        println!("{}", to_colored_json_auto(&serde_json::to_value(msg)?)?);
        Ok(Value::default())
    } else {
        let config = CONFIG.read().await;
        let addr = Text::new("Cw20 Contract Address?")
            .with_help_message("string")
            .prompt()?;
        let value = query(&config, addr, msg).await?;
        let color = to_colored_json_auto(&value)?;
        println!("{color}");
        Ok(value)
    }
}

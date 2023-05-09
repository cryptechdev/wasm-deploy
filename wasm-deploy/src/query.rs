use std::str::FromStr;

use colored_json::to_colored_json_auto;
use cosm_utils::{modules::auth::model::Address, prelude::Cosmwasm};
use cw20::Cw20QueryMsg;
use inquire::Text;
use interactive_parse::InteractiveParseObj;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tendermint_rpc::HttpClient;

use crate::{
    contract::Contract,
    file::{Config, CONFIG},
    utils::replace_strings_any,
};

pub async fn query_contract(contract: &impl Contract) -> anyhow::Result<Value> {
    println!("Querying");
    let config = CONFIG.read().await;
    let msg = contract.query()?;
    let addr = config.get_contract_addr(&contract.to_string())?.clone();
    let value = query(&config, addr, msg).await?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    Ok(value)
}

pub async fn query(
    config: &Config,
    mut addr: impl AsRef<str> + Serialize + DeserializeOwned + Clone,
    msg: impl Serialize + Sync,
) -> anyhow::Result<Value> {
    replace_strings_any(&mut addr, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?.clone();
    let client = HttpClient::new(chain_info.rpc_endpoint.as_str())?;
    let response = client
        .wasm_query(Address::from_str(addr.as_ref())?, &msg)
        .await?;
    let string = String::from_utf8(response.data)?;
    Ok(serde_json::from_str::<Value>(string.as_str())?)
}

pub async fn cw20_query() -> anyhow::Result<Value> {
    println!("Querying cw20");
    let config = CONFIG.read().await;
    let addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let msg = Cw20QueryMsg::parse_to_obj()?;
    let value = query(&config, addr, msg).await?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    Ok(value)
}

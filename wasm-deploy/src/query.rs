use std::str::FromStr;

use colored_json::to_colored_json_auto;
use cosm_tome::{
    clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
    modules::auth::model::Address,
};
use cw20::Cw20QueryMsg;
use inquire::Text;
use interactive_parse::traits::InteractiveParseObj;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    contract::Contract,
    error::DeployError,
    file::Config,
    utils::{replace_strings, replace_strings_any},
};

pub async fn query_contract(contract: &impl Contract) -> Result<Value, DeployError> {
    println!("Querying");
    let mut config = Config::load()?;
    let msg = contract.query()?;
    let addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let value = query(&mut config, addr, msg).await?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    Ok(value)
}

pub async fn query(
    config: &mut Config,
    mut addr: impl AsRef<str> + Serialize + DeserializeOwned + Clone,
    msg: impl Serialize,
) -> Result<Value, DeployError> {
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    replace_strings_any(&mut addr, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(
        &chain_info
            .grpc_endpoint
            .clone()
            .ok_or(DeployError::MissingGRpc)?,
    )?;
    let cosm_tome = CosmTome::new(chain_info, client);
    let response = cosm_tome
        .wasm_query(Address::from_str(addr.as_ref()).unwrap(), &value)
        .await?;
    let string = String::from_utf8(response.res.data.unwrap()).unwrap();
    Ok(serde_json::from_str::<Value>(string.as_str()).unwrap())
}

pub async fn cw20_query() -> Result<Value, DeployError> {
    println!("Querying cw20");
    let mut config = Config::load()?;
    let addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let msg = Cw20QueryMsg::parse_to_obj()?;
    let value = query(&mut config, addr, msg).await?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    Ok(value)
}

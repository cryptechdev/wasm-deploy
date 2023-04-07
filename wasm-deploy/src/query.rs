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
    msg: impl Serialize,
) -> anyhow::Result<Value> {
    replace_strings_any(&mut addr, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_config()?.clone();
    let client = TendermintRPC::new(
        &chain_info
            .rpc_endpoint
            .clone()
            .ok_or(DeployError::MissingRpc)?,
    )?;
    let cosm_tome = CosmTome::new(chain_info, client);
    let response = cosm_tome
        .wasm_query(Address::from_str(addr.as_ref())?, &msg)
        .await?;
    let string = String::from_utf8(response.res.data.ok_or(DeployError::EmptyResponse)?)?;
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

#[cfg(test)]
mod test {
    use cosm_tome::{
        clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
        config::cfg::ChainConfig,
    };

    #[tokio::test]
    async fn test_query() {
        let url = "https://archive-rpc.noria.nextnet.zone:443";
        let client = TendermintRPC::new(url).expect("Failed to create client");
        let chain_cfg = ChainConfig {
            denom: "".to_string(),
            prefix: "".to_string(),
            chain_id: "".to_string(),
            derivation_path: "".to_string(),
            rpc_endpoint: Some("".to_string()),
            grpc_endpoint: None,
            gas_price: 0.025,
            gas_adjustment: 1.2,
        };
        let cosm_tome = CosmTome::new(chain_cfg, client);

        let res = cosm_tome.tendermint_query_latest_block().await.unwrap();

        println!("{:#?}", res);
    }
}

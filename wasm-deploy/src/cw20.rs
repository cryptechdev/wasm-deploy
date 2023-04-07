use std::str::FromStr;

use crate::{contract::Contract, error::DeployError, file::CONFIG};
use colored::Colorize;
use cosm_tome::{
    chain::{coin::Coin, request::TxOptions},
    clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
    modules::{
        auth::model::Address,
        cosmwasm::model::{ExecRequest, InstantiateRequest},
    },
};
use cw20::Cw20ExecuteMsg;
use inquire::{CustomType, Text};
use interactive_parse::traits::InteractiveParseObj;

pub async fn cw20_send(contract: &impl Contract) -> anyhow::Result<()> {
    println!("Executing cw20 send");
    let config = CONFIG.read().await;
    let key = config.get_active_key().await?;

    let hook_msg = contract.cw20_send()?;
    let contract_addr = config.get_contract_addr(&contract.to_string())?.clone();
    let cw20_contract_addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let amount = CustomType::<u64>::new("Amount of tokens to send?")
        .with_help_message("int")
        .prompt()?;
    let msg = Cw20ExecuteMsg::Send {
        contract: contract_addr,
        amount: amount.into(),
        msg: serde_json::to_vec(&hook_msg)?.into(),
    };
    let chain_info = config.get_active_chain_config()?.clone();
    let client = TendermintRPC::new(
        &chain_info
            .rpc_endpoint
            .clone()
            .ok_or(DeployError::MissingRpc)?,
    )?;
    let cosm_tome = CosmTome::new(chain_info, client);
    let funds = Vec::<Coin>::parse_to_obj()?;
    let req = ExecRequest {
        msg,
        funds,
        address: Address::from_str(&cw20_contract_addr)?,
    };
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
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

pub async fn cw20_execute() -> anyhow::Result<()> {
    println!("Executing cw20 transfer");
    let config = CONFIG.read().await;
    let key = config.get_active_key().await?;

    let cw20_contract_addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let msg = Cw20ExecuteMsg::parse_to_obj()?;
    let chain_info = config.get_active_chain_config()?.clone();
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
        msg,
        funds: vec![],
        address: Address::from_str(&cw20_contract_addr)?,
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

pub async fn cw20_instantiate() -> anyhow::Result<()> {
    println!("Executing cw20 instantiate");
    let config = CONFIG.read().await;
    let key = config.get_active_key().await?;

    let code_id: u64 = Text::new("Cw20 Code Id?")
        .with_help_message("int")
        .prompt()?
        .parse()?;

    let admin = Some(Address::from_str(
        Text::new("Admin Addr?")
            .with_help_message("string")
            .prompt()?
            .as_str(),
    )?);

    let msg = cw20_base::msg::InstantiateMsg::parse_to_obj()?;
    let chain_info = config.get_active_chain_config()?.clone();
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
    let req = InstantiateRequest {
        code_id,
        funds: vec![],
        msg,
        label: "cw20".into(),
        admin,
    };

    let response = cosm_tome.wasm_instantiate(req, &key, &tx_options).await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

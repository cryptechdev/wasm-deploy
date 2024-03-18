use std::str::FromStr;

use crate::{config::CONFIG, contract::Deploy};
use colored::Colorize;
use colored_json::to_colored_json_auto;
use cosm_utils::prelude::*;
use cosm_utils::{
    chain::{coin::Coin, request::TxOptions},
    modules::{
        auth::model::Address,
        cosmwasm::model::{ExecRequest, InstantiateRequest},
    },
};
use cw20::Cw20ExecuteMsg;
use inquire::{CustomType, Text};
use interactive_parse::InteractiveParseObj;
use tendermint_rpc::HttpClient;

pub async fn cw20_send(contract: &impl Deploy, dry_run: bool) -> anyhow::Result<()> {
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
    if dry_run {
        println!("{}", to_colored_json_auto(&serde_json::to_value(msg)?)?);
    } else {
        let chain_info = config.get_active_chain_info()?.clone();
        let client = HttpClient::get_persistent_compat(chain_info.rpc_endpoint.as_str()).await?;
        let funds = Vec::<Coin>::parse_to_obj()?;
        let req = ExecRequest {
            msg,
            funds,
            address: Address::from_str(&cw20_contract_addr)?,
        };

        let response = client
            .wasm_execute_commit(&chain_info.cfg, req, &key, &TxOptions::default())
            .await?;
        println!(
            "gas wanted: {}, gas used: {}",
            response.deliver_tx.gas_wanted.to_string().green(),
            response.deliver_tx.gas_used.to_string().green()
        );
        println!("tx hash: {}", response.hash.to_string().purple());
    }
    Ok(())
}

pub async fn cw20_execute(dry_run: bool) -> anyhow::Result<()> {
    println!("Executing cw20 transfer");
    let config = CONFIG.read().await;
    let key = config.get_active_key().await?;

    let cw20_contract_addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let msg = Cw20ExecuteMsg::parse_to_obj()?;
    let chain_info = config.get_active_chain_info()?.clone();
    let client = HttpClient::get_persistent_compat(chain_info.rpc_endpoint.as_str()).await?;
    let req = ExecRequest {
        msg,
        funds: vec![],
        address: Address::from_str(&cw20_contract_addr)?,
    };
    if dry_run {
        println!("{}", to_colored_json_auto(&serde_json::to_value(req.msg)?)?);
    } else {
        let response = client
            .wasm_execute_commit(&chain_info.cfg, req, &key, &TxOptions::default())
            .await?;

        println!(
            "gas wanted: {}, gas used: {}",
            response.deliver_tx.gas_wanted.to_string().green(),
            response.deliver_tx.gas_used.to_string().green()
        );
        println!("tx hash: {}", response.hash.to_string().purple());
    }

    Ok(())
}

pub async fn cw20_instantiate(dry_run: bool) -> anyhow::Result<()> {
    println!("Executing cw20 instantiate");
    let config = CONFIG.read().await;
    let key = config.get_active_key().await?;

    let msg = cw20_base::msg::InstantiateMsg::parse_to_obj()?;

    if dry_run {
        println!("{}", to_colored_json_auto(&serde_json::to_value(msg)?)?);
    } else {
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

        let chain_info = config.get_active_chain_info()?.clone();
        let client = HttpClient::get_persistent_compat(chain_info.rpc_endpoint.as_str()).await?;
        let req = InstantiateRequest {
            code_id,
            funds: vec![],
            msg,
            label: "cw20".into(),
            admin,
        };

        let response = client
            .wasm_instantiate_commit(&chain_info.cfg, req, &key, &TxOptions::default())
            .await?;

        println!(
            "gas wanted: {}, gas used: {}",
            response.res.deliver_tx.gas_wanted.to_string().green(),
            response.res.deliver_tx.gas_used.to_string().green()
        );
        println!("tx hash: {}", response.res.hash.to_string().purple());
    }
    Ok(())
}

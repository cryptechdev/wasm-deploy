use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use colored::Colorize;
use colored_json::to_colored_json_auto;
use cosm_tome::{
    chain::{coin::Coin, request::TxOptions},
    clients::{client::CosmTome, cosmos_grpc::CosmosgRPC},
    modules::{auth::model::Address, cosmwasm::model::ExecRequest},
};
use cw20::Cw20ExecuteMsg;
use inquire::{CustomType, Text};
use interactive_parse::traits::InteractiveParseObj;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::{
    error::{DeployError, DeployResult},
    file::Config,
    utils::replace_strings,
};

pub trait Contract: Send + Sync + Debug + From<String> + IntoEnumIterator + Display + Clone + 'static {
    type ExecuteMsg: Execute;
    type QueryMsg: Query;
    type Cw20HookMsg: Cw20Hook;

    fn name(&self) -> String;
    fn admin(&self) -> String;
    fn instantiate_msg(&self) -> Result<Value, DeployError>;
    fn migrate_msg(&self) -> Result<Option<Value>, DeployError>;
    fn external_instantiate_msgs(&self) -> Result<Vec<ExternalInstantiate>, DeployError>;
    fn config_msg(&self) -> Result<Option<Value>, DeployError>;
    fn set_up_msgs(&self) -> Result<Vec<Value>, DeployError>;
}

pub trait Execute: Serialize + DeserializeOwned + Display + Debug {
    fn execute_msg(&self) -> Result<Value, DeployError>;
    fn parse(contract: &impl Contract) -> DeployResult<Self>;
}

pub async fn execute(contract: &impl Execute) -> Result<(), DeployError> {
    println!("Executing");
    let mut config = Config::load()?;
    let mut msg = contract.execute_msg()?;
    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(chain_info.grpc_endpoint.clone().ok_or(DeployError::MissingGRpc)?);
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let funds = Vec::<Coin>::parse_to_obj()?;
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let req = ExecRequest { msg, funds, address: Address::from_str(&contract_addr).unwrap() };
    let response = cosm_tome.wasm_execute(req, &key, &tx_options).await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

pub trait Query: Serialize + DeserializeOwned + Display + Debug {
    fn query_msg(&self) -> Result<Value, DeployError>;
    fn parse(contract: &impl Contract) -> DeployResult<Self>;
}

pub async fn query(contract: &impl Query) -> Result<(), DeployError> {
    println!("Querying");
    let mut config = Config::load()?;
    let mut msg = contract.query_msg()?;
    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
    let addr = config.get_contract_addr_mut(&contract.to_string())?;
    let client = CosmosgRPC::new(chain_info.grpc_endpoint.clone().ok_or(DeployError::MissingGRpc)?);
    let cosm_tome = CosmTome::new(chain_info, client);
    let response = cosm_tome.wasm_query(Address::from_str(addr).unwrap(), &msg).await?;

    let string = String::from_utf8(response.res.data.unwrap()).unwrap();
    let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
    let color = to_colored_json_auto(&value)?;
    println!("{color}");

    Ok(())
}

pub trait Cw20Hook: Serialize + DeserializeOwned + Display + Debug {
    fn cw20_hook_msg(&self) -> Result<Value, DeployError>;
    fn parse(contract: &impl Contract) -> DeployResult<Self>;
}

pub async fn cw20_send(contract: &impl Cw20Hook) -> Result<(), DeployError> {
    println!("Executing cw20 send");
    let mut config = Config::load()?;
    let key = config.get_active_key().await?;

    let hook_msg = contract.cw20_hook_msg()?;
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let cw20_contract_addr = Text::new("Cw20 Contract Address?").with_help_message("string").prompt()?;
    let amount = CustomType::<u64>::new("Amount of tokens to send?").with_help_message("int").prompt()?;
    let msg = Cw20ExecuteMsg::Send {
        contract: contract_addr,
        amount: amount.into(),
        msg: serde_json::to_vec(&hook_msg)?.into(),
    };
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(chain_info.grpc_endpoint.clone().ok_or(DeployError::MissingGRpc)?);
    let cosm_tome = CosmTome::new(chain_info, client);
    let funds = Vec::<Coin>::parse_to_obj()?;
    let req = ExecRequest { msg, funds, address: Address::from_str(&cw20_contract_addr).unwrap() };
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let response = cosm_tome.wasm_execute(req, &key, &tx_options).await?;
    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

pub async fn cw20_transfer() -> Result<(), DeployError> {
    println!("Executing cw20 transfer");
    let mut config = Config::load()?;
    let key = config.get_active_key().await?;

    let cw20_contract_addr = Text::new("Cw20 Contract Address?").with_help_message("string").prompt()?;
    let msg = Cw20ExecuteMsg::parse_to_obj()?;
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(chain_info.grpc_endpoint.clone().ok_or(DeployError::MissingGRpc)?);
    let cosm_tome = CosmTome::new(chain_info, client);
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let req = ExecRequest { msg, funds: vec![], address: Address::from_str(&cw20_contract_addr).unwrap() };
    let response = cosm_tome.wasm_execute(req, &key, &tx_options).await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

#[derive(Clone, Debug)]
pub struct ExternalInstantiate {
    pub msg: Value,
    pub code_id: u64,
    pub name: String,
}

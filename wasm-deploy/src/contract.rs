use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use crate::{
    error::{DeployError, DeployResult},
    file::Config,
    utils::replace_strings,
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
use serde::Serialize;
use serde_json::Value;
use strum::{IntoEnumIterator, ParseError};

pub trait Msg: Debug + Send + Sync + erased_serde::Serialize {}

impl<T> Msg for T where T: Debug + Serialize + Send + Sync {}

impl Serialize for dyn Msg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        erased_serde::serialize(self, serializer)
    }
}

pub trait Contract:
    Send + Sync + Debug + Display + FromStr<Err = ParseError> + IntoEnumIterator + 'static
{
    fn name(&self) -> String;
    fn admin(&self) -> String;

    fn execute(&self) -> DeployResult<Box<dyn Msg>>;
    fn query(&self) -> DeployResult<Box<dyn Msg>>;
    fn cw20_send(&self) -> DeployResult<Box<dyn Msg>>;

    fn instantiate_msg(&self) -> Option<Box<dyn Msg>>;
    fn external_instantiate_msgs(&self) -> Vec<ExternalInstantiate<Box<dyn Msg>>>;
    fn migrate_msg(&self) -> Option<Box<dyn Msg>>;
    fn set_config_msg(&self) -> Option<Box<dyn Msg>>;
    // TODO: Ideally these could be any generic request type
    fn set_up_msgs(&self) -> Vec<Box<dyn Msg>>;
}

#[derive(Debug, Clone)]
pub struct ExternalInstantiate<T> {
    pub msg: T,
    pub code_id: u64,
    pub name: String,
}

impl<T> From<ExternalInstantiate<T>> for ExternalInstantiate<Box<dyn Msg>>
where
    T: Msg + Clone + 'static,
{
    fn from(msg: ExternalInstantiate<T>) -> Self {
        ExternalInstantiate {
            msg: Box::new(msg.msg),
            code_id: msg.code_id,
            name: msg.name,
        }
    }
}

pub async fn execute(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Executing");
    let mut config = Config::load()?;
    let msg = contract.execute()?;
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(
        chain_info
            .grpc_endpoint
            .clone()
            .ok_or(DeployError::MissingGRpc)?,
    );
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let funds = Vec::<Coin>::parse_to_obj()?;
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
    };
    let req = ExecRequest {
        msg: value,
        funds,
        address: Address::from_str(&contract_addr).unwrap(),
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
    addr: impl AsRef<str>,
    msg: impl Serialize,
) -> Result<Value, DeployError> {
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(
        chain_info
            .grpc_endpoint
            .clone()
            .ok_or(DeployError::MissingGRpc)?,
    );
    let cosm_tome = CosmTome::new(chain_info, client);
    let response = cosm_tome
        .wasm_query(Address::from_str(addr.as_ref()).unwrap(), &value)
        .await?;
    let string = String::from_utf8(response.res.data.unwrap()).unwrap();
    Ok(serde_json::from_str::<Value>(string.as_str()).unwrap())
}

pub async fn cw20_send(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Executing cw20 send");
    let mut config = Config::load()?;
    let key = config.get_active_key().await?;

    let hook_msg = contract.cw20_send()?;
    let mut value = serde_json::to_value(hook_msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let cw20_contract_addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let amount = CustomType::<u64>::new("Amount of tokens to send?")
        .with_help_message("int")
        .prompt()?;
    let msg = Cw20ExecuteMsg::Send {
        contract: contract_addr,
        amount: amount.into(),
        msg: serde_json::to_vec(&value)?.into(),
    };
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(
        chain_info
            .grpc_endpoint
            .clone()
            .ok_or(DeployError::MissingGRpc)?,
    );
    let cosm_tome = CosmTome::new(chain_info, client);
    let funds = Vec::<Coin>::parse_to_obj()?;
    let req = ExecRequest {
        msg,
        funds,
        address: Address::from_str(&cw20_contract_addr).unwrap(),
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

pub async fn cw20_transfer() -> Result<(), DeployError> {
    println!("Executing cw20 transfer");
    let mut config = Config::load()?;
    let key = config.get_active_key().await?;

    let cw20_contract_addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let msg = Cw20ExecuteMsg::parse_to_obj()?;
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(
        chain_info
            .grpc_endpoint
            .clone()
            .ok_or(DeployError::MissingGRpc)?,
    );
    let cosm_tome = CosmTome::new(chain_info, client);
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
    };
    let req = ExecRequest {
        msg: value,
        funds: vec![],
        address: Address::from_str(&cw20_contract_addr).unwrap(),
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

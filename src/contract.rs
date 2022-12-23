use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use colored::Colorize;
use colored_json::to_colored_json_auto;
use cosm_tome::{
    chain::{coin::Coin, request::TxOptions},
    clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
    modules::{
        auth::model::Address,
        cosmwasm::model::{ExecRequest, InstantiateRequest, MigrateRequest, StoreCodeRequest},
    },
};
use cw20::Cw20ExecuteMsg;
use inquire::{CustomType, Text};
use interactive_parse::traits::InteractiveParseObj;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::{
    error::{DeployError, DeployResult},
    file::{Config, ContractInfo},
};

pub trait Contract: Send + Sync + Debug + From<String> + IntoEnumIterator + Display + Clone + 'static {
    type ExecuteMsg: Execute;
    type QueryMsg: Query;
    type Cw20HookMsg: Cw20Hook;

    fn name(&self) -> String;
    fn admin(&self) -> String;
    fn instantiate_msg(&self) -> Result<Value, DeployError>;
    fn external_instantiate_msgs(&self) -> Result<Vec<ExternalInstantiate>, DeployError>;
    fn base_config_msg(&self) -> Result<Option<Value>, DeployError>;
    fn set_up_msgs(&self) -> Result<Vec<Value>, DeployError>;
}

pub async fn execute_store(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Storing code for {}", contract.name());
    let mut config = Config::load()?;
    let chain_info = config.get_active_chain_info()?;
    let key = config.get_active_key().await?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
    let cosm_tome = CosmTome::new(chain_info, client);
    let path = format!("./artifacts/{}.wasm", contract.name());
    let wasm_data = std::fs::read(path)?;
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let req = StoreCodeRequest { wasm_data, instantiate_perms: None };
    let response = cosm_tome.wasm_store(req, &key, &tx_options).await?;

    match config.get_contract(&contract.to_string()) {
        Ok(contract_info) => contract_info.code_id = Some(response.code_id),
        Err(_) => {
            config.add_contract_from(ContractInfo {
                name:    contract.name(),
                addr:    None,
                code_id: Some(response.code_id),
            })?;
        }
    }

    config.save()?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

pub async fn execute_instantiate(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Instantiating {}", contract.name());
    let mut config = Config::load()?;
    let mut msg = contract.instantiate_msg()?;
    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_info = config.get_contract(&contract.to_string())?;
    let code_id = contract_info.code_id.ok_or(DeployError::CodeIdNotFound)?;
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let req = InstantiateRequest {
        code_id,
        msg,
        label: String::new(),
        admin: Some(Address::from_str(&contract.admin()).unwrap()),
        funds: vec![],
    };

    let response = cosm_tome.wasm_instantiate(req, &key, &tx_options).await?;

    contract_info.addr = Some(response.address.to_string());
    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    for mut external in contract.external_instantiate_msgs()? {
        println!("Instantiating {}", external.name);
        replace_strings(&mut external.msg, &config.get_active_env()?.contracts)?;
        let req = InstantiateRequest {
            code_id: external.code_id,
            msg:     external.msg,
            label:   String::new(),
            admin:   Some(Address::from_str(&contract.admin()).unwrap()),
            funds:   vec![],
        };

        let response = cosm_tome.wasm_instantiate(req, &key, &tx_options).await?;

        config.add_contract_from(ContractInfo {
            name:    external.name,
            addr:    Some(response.address.to_string()),
            code_id: Some(external.code_id),
        })?;

        println!(
            "gas wanted: {}, gas used: {}",
            response.res.gas_wanted.to_string().green(),
            response.res.gas_used.to_string().green()
        );
        println!("tx hash: {}", response.res.tx_hash.purple());
    }
    config.save()?;
    Ok(())
}

// assumes store code has already been called
pub async fn execute_migrate(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Migrating {}", contract.name());
    let mut config = Config::load()?;
    let mut msg = contract.instantiate_msg()?;
    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_info = config.get_contract(&contract.to_string())?;
    let contract_addr = contract_info.addr.clone().ok_or(DeployError::AddrNotFound)?;
    let code_id = contract_info.code_id.ok_or(DeployError::CodeIdNotFound)?;
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let req = MigrateRequest { msg, address: Address::from_str(&contract_addr).unwrap(), new_code_id: code_id };
    let response = cosm_tome.wasm_migrate(req, &key, &tx_options).await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

/// TODO: remove duplicate code here
pub async fn execute_set_config(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Setting config for {}", contract.name());
    let mut config = Config::load()?;
    let Some(mut msg) = contract.base_config_msg()? else { return Ok(()) };
    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let req = ExecRequest { msg, funds: vec![], address: Address::from_str(&contract_addr).unwrap() };

    let response = cosm_tome.wasm_execute(req, &key, &tx_options).await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.res.tx_hash.purple());

    Ok(())
}

pub async fn execute_set_up(contract: &impl Contract) -> Result<(), DeployError> {
    println!("Executing set up for {}", contract.name());
    let mut config = Config::load()?;
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let tx_options = TxOptions { timeout_height: None, fee: None, memo: "wasm_deploy".into() };
    let key = config.get_active_key().await?;

    for mut msg in contract.set_up_msgs()? {
        replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
        let req = ExecRequest { msg, funds: vec![], address: Address::from_str(&contract_addr).unwrap() };

        let response = cosm_tome.wasm_execute(req, &key, &tx_options).await?;

        println!(
            "gas wanted: {}, gas used: {}",
            response.res.gas_wanted.to_string().green(),
            response.res.gas_used.to_string().green()
        );
        println!("tx hash: {}", response.res.tx_hash.purple());
    }
    Ok(())
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
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
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
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
    let cosm_tome = CosmTome::new(chain_info, client);
    let response = cosm_tome.wasm_query(Address::from_str(addr).unwrap(), &msg).await?;

    let string = String::from_utf8(response.res.data.unwrap()).unwrap();
    let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
    let color = to_colored_json_auto(&value)?;
    println!("{}", color);

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
        amount:   amount.into(),
        msg:      serde_json::to_vec(&hook_msg)?.into(),
    };
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
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
    let recipient = Text::new("Recipient?").with_help_message("string").prompt()?;
    let amount = CustomType::<u64>::new("Amount of tokens to send?").with_help_message("int").prompt()?;
    let msg = Cw20ExecuteMsg::Transfer { recipient, amount: amount.into() };
    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(&chain_info.rpc_endpoint.clone().unwrap()).unwrap();
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
    pub msg:     Value,
    pub code_id: u64,
    pub name:    String,
}

fn replace_strings(value: &mut Value, contracts: &Vec<ContractInfo>) -> DeployResult<()> {
    match value {
        Value::Null => {}
        Value::Bool(_) => {}
        Value::Number(_) => {}
        Value::String(string) => {
            if let Some((_, new)) = string.split_once('&') {
                if let Some(contract) = contracts.iter().find(|x| x.name == new) {
                    match &contract.addr {
                        Some(addr) => *string = addr.clone(),
                        None => return Err(DeployError::AddrNotFound),
                    }
                }
            }
        }
        Value::Array(array) => {
            for value in array {
                replace_strings(value, contracts)?;
            }
        }
        Value::Object(map) => {
            for (_, value) in map {
                replace_strings(value, contracts)?;
            }
        }
    }
    Ok(())
}

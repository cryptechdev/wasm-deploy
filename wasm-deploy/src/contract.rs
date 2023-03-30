use std::{
    fmt::{Debug, Display},
    path::PathBuf,
    str::FromStr,
};

use crate::{
    error::DeployError, file::Config, settings::WorkspaceSettings, utils::replace_strings,
};
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
use serde::Serialize;
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
    /// This is the name of the contract and represents
    /// how it will appear in the cli.
    fn name(&self) -> String;

    /// This is the address of the contract admin.
    /// It is required when instantiating.
    fn admin(&self) -> String;

    /// This method allows executing a contract.
    /// interactive-parse should be used to generate the msg.
    fn execute(&self) -> anyhow::Result<Box<dyn Msg>> {
        Err(DeployError::TraitNotImplemented.into())
    }

    /// This method allows querying a contract.
    /// interactive-parse should be used to generate the msg.
    fn query(&self) -> anyhow::Result<Box<dyn Msg>> {
        Err(DeployError::TraitNotImplemented.into())
    }

    /// This method allows sending a cw20 token with an attached message to a contract.
    /// interactive-parse should be used to generate the msg.
    fn cw20_send(&self) -> anyhow::Result<Box<dyn Msg>> {
        Err(DeployError::TraitNotImplemented.into())
    }

    /// This method gets the preprogrammed instantiate msg for the contract.
    fn instantiate_msg(&self) -> Option<Box<dyn Msg>>;

    /// This method will instantiate an external contract via code_id alongside a local contract.
    fn external_instantiate_msgs(&self) -> Vec<ExternalInstantiate<Box<dyn Msg>>> {
        vec![]
    }

    /// This method gets the preprogrammed migrate msg for the contract.
    fn migrate_msg(&self) -> Option<Box<dyn Msg>> {
        None
    }

    /// This method gets the preprogrammed set config msg for the contract.
    fn set_config_msg(&self) -> Option<Box<dyn Msg>> {
        None
    }

    /// This method gets the preprogrammed set up for the contract.
    fn set_up_msgs(&self) -> Vec<Box<dyn Msg>> {
        vec![]
    }

    /// This method allows for customizing the path to the contract.
    /// This should be the path relative to the project root.
    fn path(&self) -> PathBuf {
        PathBuf::from(format!("contracts/{}", self.name()))
    }
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

pub async fn cw20_send(
    settings: &WorkspaceSettings,
    contract: &impl Contract,
) -> anyhow::Result<()> {
    println!("Executing cw20 send");
    let mut config = Config::load(settings)?;
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

pub async fn cw20_execute(settings: &WorkspaceSettings) -> anyhow::Result<()> {
    println!("Executing cw20 transfer");
    let mut config = Config::load(settings)?;
    let key = config.get_active_key().await?;

    let cw20_contract_addr = Text::new("Cw20 Contract Address?")
        .with_help_message("string")
        .prompt()?;
    let msg = Cw20ExecuteMsg::parse_to_obj()?;
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
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

pub async fn cw20_instantiate(settings: &WorkspaceSettings) -> anyhow::Result<()> {
    println!("Executing cw20 instantiate");
    let mut config = Config::load(settings)?;
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
    let mut msg = serde_json::to_value(msg)?;
    replace_strings(&mut msg, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
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

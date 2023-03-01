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
use serde_json::Value;
use strum::IntoEnumIterator;

// pub trait Request: Debug + Send + Sync {
//     fn to_any(&self, sender_addr: Address) -> DeployResult<Any>;
// }

// impl<S> Request for ExecRequest<S>
// where
//     S: Serialize + Clone + Debug + Send + Sync,
// {
//     fn to_any(&self, sender_addr: Address) -> DeployResult<Any> {
//         let proto = self.clone().to_proto(sender_addr)?;
//         let any = proto.to_any()?;
//         Ok(any)
//     }
// }

// pub trait MyMsg: MySerial + JsonSchema + Display {}

// #[derive(Clone, Debug)]
// pub struct ContractStruct<'a, E: MyMsg, Q: MyMsg, C: MyMsg> {
//     pub name: String,
//     pub admin: String,
//     pub instantiate_msg: Option<&'a dyn MySerial>,
//     pub external_instantiate_msgs: Vec<ExternalInstantiate<'a>>,
//     pub migrate_msg: Option<&'a dyn MySerial>,
//     pub set_config_msg: Option<&'a dyn MySerial>,
//     pub set_up_msgs: Vec<&'a dyn MySerial>,
//     pub execute_type: std::marker::PhantomData<E>,
//     pub query_type: std::marker::PhantomData<Q>,
//     pub cw20_hook_type: std::marker::PhantomData<C>,
// }

// impl<'a, E: MyMsg, Q: MyMsg, C: MyMsg> Display for ContractStruct<'a, E, Q, C> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         todo!()
//     }
// }

// impl<'a, E: MyMsg, Q: MyMsg, C: MyMsg> From<String> for ContractStruct<'a, E, Q, C> {
//     fn from(value: String) -> Self {
//         todo!()
//     }
// }

// impl<'a, E: MyMsg, Q: MyMsg, C: MyMsg> Contract for ContractStruct<'a, E, Q, C> {
//     type ExecuteMsg = E;

//     type QueryMsg = Q;

//     type Cw20HookMsg = C;

//     fn name(&self) -> String {
//         self.name.clone()
//     }

//     fn admin(&self) -> String {
//         self.admin.clone()
//     }

//     fn instantiate_msg(&self) -> Option<&dyn MySerial> {
//         self.instantiate_msg
//     }

//     fn external_instantiate_msgs(&self) -> Vec<ExternalInstantiate> {
//         self.external_instantiate_msgs
//     }

//     fn migrate_msg(&self) -> Option<&dyn MySerial> {
//         self.migrate_msg
//     }

//     fn set_config_msg(&self) -> Option<&dyn MySerial> {
//         self.set_config_msg
//     }

//     fn set_up_msgs(&self) -> Vec<&dyn MySerial> {
//         self.set_up_msgs
//     }
// }

#[typetag::serde(tag = "type")]
pub trait MySerial: Debug + Send + Sync {}

pub trait Contract:
    Send + Sync + Debug + Display + From<String> + IntoEnumIterator + 'static
{
    fn name(&self) -> String;
    fn admin(&self) -> String;

    fn execute(&self) -> DeployResult<Box<dyn MySerial>>;
    fn query(&self) -> DeployResult<Box<dyn MySerial>>;
    fn cw20_send(&self) -> DeployResult<Box<dyn MySerial>>;

    fn instantiate_msg(&self) -> Option<&dyn MySerial>;
    fn external_instantiate_msgs(&self) -> Vec<ExternalInstantiate>;
    fn migrate_msg(&self) -> Option<&dyn MySerial>;
    fn set_config_msg(&self) -> Option<&dyn MySerial>;
    // TODO: Ideally these could be any generic request type
    fn set_up_msgs(&self) -> Vec<&dyn MySerial>;
}

// pub trait Contract:
//     Send + Sync + Debug + From<String> + IntoEnumIterator + Display + Clone + 'static
// {
//     type ExecuteMsg: Execute;
//     type QueryMsg: Query;
//     type Cw20HookMsg: Cw20Hook;

//     fn name(&self) -> String;
//     fn admin(&self) -> String;
//     fn instantiate_msg(&self) -> Result<Value, DeployError>;
//     fn migrate_msg(&self) -> Result<Option<Value>, DeployError>;
//     fn external_instantiate_msgs(&self) -> Result<Vec<ExternalInstantiate>, DeployError>;
//     fn config_msg(&self) -> Result<Option<Value>, DeployError>;
//     fn set_up_msgs(&self) -> Result<Vec<Value>, DeployError>;
// }

// pub trait Execute: Serialize + DeserializeOwned + Display + Debug {
//     fn execute_msg(&self) -> Result<Value, DeployError>;
//     fn parse(contract: &impl Contract) -> DeployResult<Self>;
// }

pub async fn execute<C: Contract>(contract: &C) -> Result<(), DeployError> {
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

// pub trait Query: Serialize + DeserializeOwned + Display + Debug {
//     fn query_msg(&self) -> Result<Value, DeployError>;
//     fn parse(contract: &impl Contract) -> DeployResult<Self>;
// }

pub async fn query<C: Contract>(contract: &C) -> Result<Value, DeployError> {
    println!("Querying");
    let mut config = Config::load()?;
    let msg = contract.query()?;
    let mut value = serde_json::to_value(msg)?;
    replace_strings(&mut value, &config.get_active_env()?.contracts)?;
    let chain_info = config.get_active_chain_info()?;
    let addr = config.get_contract_addr_mut(&contract.to_string())?;
    let client = CosmosgRPC::new(
        chain_info
            .grpc_endpoint
            .clone()
            .ok_or(DeployError::MissingGRpc)?,
    );
    let cosm_tome = CosmTome::new(chain_info, client);
    let response = cosm_tome
        .wasm_query(Address::from_str(addr).unwrap(), &value)
        .await?;

    let string = String::from_utf8(response.res.data.unwrap()).unwrap();
    let value: serde_json::Value = serde_json::from_str(string.as_str()).unwrap();
    let color = to_colored_json_auto(&value)?;
    println!("{color}");

    Ok(value)
}

// pub trait Cw20Hook: Serialize + DeserializeOwned + Display + Debug {
//     fn cw20_hook_msg(&self) -> Result<Value, DeployError>;
//     fn parse(contract: &impl Contract) -> DeployResult<Self>;
// }

pub async fn cw20_send<C: Contract>(contract: &C) -> Result<(), DeployError> {
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
        msg,
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

#[derive(Clone, Debug)]
pub struct ExternalInstantiate<'a> {
    pub msg: &'a dyn MySerial,
    pub code_id: u64,
    pub name: String,
}

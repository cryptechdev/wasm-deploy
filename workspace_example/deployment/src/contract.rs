// This file defines your contract. It's mostly boiler plate.
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use interactive_parse::traits::InteractiveParseObj;
use strum_macros::{Display, EnumIter, EnumString};
use wasm_deploy::{
    contract::{Contract, ExternalInstantiate, Msg},
    error::{DeployError, DeployResult},
};

use crate::defaults::{ADMIN, CW20_INSTANTIATE, CW20_MINT};

#[derive(Clone, Debug, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
/// This is where you define the list of all contracts you want wasm-depoy to know about
pub enum Contracts {
    Cw20Base,
    // You can add more contracts to this list
}

impl Contract for Contracts {
    fn name(&self) -> String {
        self.to_string()
    }

    fn admin(&self) -> String {
        ADMIN.to_string()
    }

    fn execute(&self) -> DeployResult<Box<dyn Msg>> {
        Ok(Box::new(Cw20ExecuteMsg::parse_to_obj()?))
    }

    fn query(&self) -> DeployResult<Box<dyn Msg>> {
        Ok(Box::new(Cw20QueryMsg::parse_to_obj()?))
    }

    fn cw20_send(&self) -> DeployResult<Box<dyn Msg>> {
        Err(DeployError::Generic("Not implemented".to_string()))
    }

    fn instantiate_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => Some(Box::new(CW20_INSTANTIATE.to_owned())),
        }
    }

    fn external_instantiate_msgs(&self) -> Vec<ExternalInstantiate<Box<dyn Msg>>> {
        match self {
            Contracts::Cw20Base => vec![],
        }
    }

    fn migrate_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => Some(Box::new(CW20_INSTANTIATE.to_owned())),
        }
    }

    fn set_config_msg(&self) -> Option<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base { .. } => None,
        }
    }

    fn set_up_msgs(&self) -> Vec<Box<dyn Msg>> {
        match self {
            Contracts::Cw20Base => CW20_MINT
                .iter()
                .map(|x| Box::new(x.clone()) as Box<dyn Msg>)
                .collect(),
        }
    }
}

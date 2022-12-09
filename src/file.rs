#[cfg(feature = "ledger")]
use std::rc::Rc;
use std::{
    fmt::Display,
    fs::{create_dir_all, OpenOptions},
    io::prelude::*,
    path::PathBuf,
};

use cosmrs::{
    rpc::{Client, HttpClient},
    tendermint::chain::Id,
};
use inquire::{Confirm, CustomType, Select, Text};
use interactive_parse::traits::InteractiveParseObj;
use lazy_static::lazy_static;
#[cfg(feature = "ledger")]
use ledger_utility::Connection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::VariantNames;

#[cfg(feature = "ledger")]
use crate::ledger::get_ledger_info;
use crate::{
    error::DeployError,
    key::{Key, KeyringParams, UserKey},
};

lazy_static! {
    pub static ref CONFIG_PATH: PathBuf = PathBuf::from("deployment/.wasm-deploy/config.json");
    pub static ref BUILD_DIR: PathBuf = PathBuf::from("target/debug/");
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Env {
    pub is_active: bool,
    pub env_id:    String,
    pub chain_id:  Id,
    pub contracts: Vec<ContractInfo>,
    pub key_name:  String,
}

impl Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.env_id.fmt(f) }
}
#[derive(Clone, Debug, JsonSchema, PartialEq, Serialize, Deserialize)]
pub struct ChainInfo {
    /// uatom
    pub denom:           String,
    pub chain_id:        String,
    pub rpc_endpoint:    String,
    pub grpc_endpoint:   String,
    /// "0.025"
    pub gas_price:       f64,
    /// "1.2"
    pub gas_adjustment:  f64,
    /// "cosmos"
    pub prefix:          String,
    /// "m/44'/118'/0'/0/0"
    #[serde(default = "default_derivation_path")]
    pub derivation_path: String,
}

fn default_derivation_path() -> String { "m/44'/118'/0'/0/0".to_string() }

impl Display for ChainInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.chain_id.fmt(f) }
}

#[derive(Clone, Debug, JsonSchema, PartialEq, Serialize, Deserialize)]
pub struct ContractInfo {
    pub name:    String,
    pub addr:    Option<String>,
    pub code_id: Option<u64>,
}

impl Display for ContractInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.name.fmt(f) }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub shell_completion_dir: Option<PathBuf>,
    pub chains:               Vec<ChainInfo>,
    pub envs:                 Vec<Env>,
    pub keys:                 Vec<UserKey>,
}

impl Config {
    pub fn init() -> Result<Config, DeployError> {
        create_dir_all(CONFIG_PATH.parent().expect("Invalid CONFIG_PATH")).unwrap();
        let config = Config::default();
        Ok(config)
    }

    pub fn load() -> Result<Config, DeployError> {
        let config = match std::fs::read(CONFIG_PATH.as_path()) {
            Ok(serialized) => serde_json::from_slice(&serialized)?,
            Err(_) => return Err(DeployError::ConfigNotFound {}),
        };

        Ok(config)
    }

    pub fn save(&self) -> Result<(), DeployError> {
        let mut file = OpenOptions::new().truncate(true).write(true).create(true).open(CONFIG_PATH.as_path())?;
        let serialized = serde_json::to_vec_pretty(self)?;
        file.write_all(&serialized)?;
        Ok(())
    }

    pub fn get_active_env_mut(&mut self) -> Result<&mut Env, DeployError> {
        match self.envs.iter().position(|x| x.is_active) {
            Some(index) => Ok(self.envs.get_mut(index).unwrap()),
            None => {
                println!("No env found, creating one");
                Ok(self.add_env()?)
            }
        }
    }

    pub fn get_active_env(&self) -> Result<&Env, DeployError> {
        match self.envs.iter().position(|x| x.is_active) {
            Some(index) => Ok(self.envs.get(index).unwrap()),
            None => Err(DeployError::EnvNotFound),
        }
    }

    pub(crate) fn get_active_chain_info(&mut self) -> Result<ChainInfo, DeployError> {
        let chains = self.chains.clone();
        let env = self.get_active_env_mut()?;
        match chains.iter().find(|x| x.chain_id == env.chain_id.to_string()) {
            Some(chain_info) => Ok(chain_info.clone()),
            None => self.add_chain(),
        }
    }

    #[allow(unused_mut)]
    pub(crate) async fn get_active_key(&mut self) -> Result<UserKey, DeployError> {
        let active_key_name = self.get_active_env()?.key_name.clone();
        let key = self
            .keys
            .iter_mut()
            .find(|x| x.name == active_key_name)
            .ok_or(DeployError::KeyNotFound { key_name: active_key_name })?;
        let mut key = key.clone();
        #[cfg(feature = "ledger")]
        if let Key::Ledger { connection, .. } = &mut key.key {
            if connection.is_none() {
                *connection = Some(Rc::new(Connection::new().await));
            }
        }
        Ok(key)
    }

    pub(crate) fn _get_active_chain_id(&mut self) -> Result<Id, DeployError> {
        Ok(self.get_active_chain_info()?.chain_id.try_into().unwrap())
    }

    pub(crate) fn _get_client(&mut self) -> Result<impl Client, DeployError> {
        let url = self.get_active_chain_info()?.rpc_endpoint;
        Ok(HttpClient::new(url.as_str()).unwrap())
    }

    pub(crate) fn add_chain_from(&mut self, chain_info: ChainInfo) -> Result<ChainInfo, DeployError> {
        match self.chains.iter().any(|x| x.chain_id == chain_info.chain_id) {
            true => Err(DeployError::ChainAlreadyExists),
            false => {
                self.chains.push(chain_info.clone());
                Ok(chain_info)
            }
        }
    }

    pub(crate) fn add_chain(&mut self) -> Result<ChainInfo, DeployError> {
        let chain_info = ChainInfo::parse_to_obj()?;
        self.add_chain_from(chain_info.clone())?;
        Ok(chain_info)
    }

    /// Adds or replaces a contract
    pub(crate) fn add_contract_from(&mut self, new_contract: ContractInfo) -> Result<ContractInfo, DeployError> {
        let env = self.get_active_env_mut()?;
        match env.contracts.iter_mut().find(|x| x.name == new_contract.name) {
            Some(contract) => *contract = new_contract.clone(),
            None => env.contracts.push(new_contract.clone()),
        }
        Ok(new_contract)
    }

    pub(crate) fn add_contract(&mut self) -> Result<ContractInfo, DeployError> {
        let contract = ContractInfo::parse_to_obj()?;
        self.add_contract_from(contract.clone())?;
        Ok(contract)
    }

    pub(crate) fn get_contract_addr_mut(&mut self, name: &String) -> Result<&String, DeployError> {
        let contract = self.get_contract(name)?;
        match &contract.addr {
            Some(addr) => Ok(addr),
            None => Err(DeployError::NoAddr),
        }
    }

    pub(crate) fn _get_code_id(&mut self, name: &String) -> Result<&mut u64, DeployError> {
        let contract = self.get_contract(name)?;
        match &mut contract.code_id {
            Some(code_id) => Ok(code_id),
            None => Err(DeployError::CodeIdNotFound),
        }
    }

    pub(crate) fn get_contract(&mut self, name: &String) -> Result<&mut ContractInfo, DeployError> {
        let env = self.get_active_env_mut()?;
        env.contracts.iter_mut().find(|x| &x.name == name).ok_or(DeployError::ContractNotFound)
    }

    pub(crate) fn add_key_from(&mut self, key: UserKey) -> Result<UserKey, DeployError> {
        if self.keys.iter().any(|x| x.name == key.name) {
            return Err(DeployError::KeyAlreadyExists);
        }
        self.keys.push(key.clone());
        Ok(key)
    }

    pub(crate) async fn add_key(&mut self) -> Result<UserKey, DeployError> {
        let key_type = Select::new("Select Key Type", Key::VARIANTS.to_vec()).prompt()?;
        let key = match key_type {
            "Keyring" => {
                let params = KeyringParams::parse_to_obj()?;
                let entry = keyring::Entry::new(&params.service, &params.user_name);
                let password = inquire::Text::new("Mnemonic?").prompt()?;
                entry.set_password(password.as_str())?;
                Key::Keyring { params }
            }
            "Mnemonic" => Key::Mnemonic { phrase: Text::new("Enter Mnemonic").prompt()? },
            #[cfg(feature = "ledger")]
            "Ledger" => {
                let chain_info = self.get_active_chain_info()?;
                let connection = Connection::new().await;
                let info = get_ledger_info(&connection, chain_info).await?;
                Key::Ledger { info, connection: None }
            }
            _ => panic!("should not happen"),
        };
        let name = Text::new("Key Name?").prompt()?;
        self.add_key_from(UserKey { name, key })
    }

    pub(crate) fn add_env(&mut self) -> Result<&mut Env, DeployError> {
        println!("Creating new deployment environment");
        let env_id = inquire::Text::new("Environment label?")
            .with_help_message("\"dev\", \"prod\", \"other\"")
            .prompt()?;
        if self.envs.iter().any(|x| x.env_id == env_id) {
            return Err(DeployError::EnvAlreadyExists);
        }
        let chain_id = inquire::Select::new("Chain?", self.chains.clone())
            .with_help_message("\"dev\", \"prod\", \"other\"")
            .prompt()?
            .chain_id;
        let key_name = inquire::Select::new("Key name?", self.keys.clone())
            .with_help_message("\"dev\", \"prod\", \"other\"")
            .prompt()?
            .name;
        let env = Env { is_active: true, key_name, env_id, chain_id: chain_id.try_into().unwrap(), contracts: vec![] };
        self.envs.push(env);
        if self.envs.len() > 1 {
            self.change_env()?
        }
        Ok(self.envs.last_mut().unwrap())
    }

    pub(crate) fn change_env(&mut self) -> Result<(), DeployError> {
        let env = Select::new("Select env to activate", self.envs.clone()).prompt()?;
        self.envs.iter_mut().for_each(|x| x.is_active = *x == env);
        Ok(())
    }
}

// TODO: move this into impl block
pub fn get_shell_completion_dir() -> Result<Option<PathBuf>, DeployError> {
    let mut config = Config::load()?;
    match config.shell_completion_dir {
        Some(shell_completion_path) => Ok(Some(shell_completion_path)),
        None => {
            let ans = Confirm::new("Shell completion directory not found.\nWould you like to add one?")
                .with_default(true)
                .prompt()?;
            match ans {
                true => {
                    let string = CustomType::<String>::new("Enter you shell completion script directory.").prompt()?;
                    let path = PathBuf::from(string);
                    match path.is_dir() {
                        true => {
                            config.shell_completion_dir = Some(path.clone());
                            config.save()?;
                            Ok(Some(path))
                        }
                        false => Err(DeployError::InvalidDir),
                    }
                }
                false => Ok(None),
            }
        }
    }
}

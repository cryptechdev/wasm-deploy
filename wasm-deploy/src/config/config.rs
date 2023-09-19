use crate::config::chain::{ChainInfo, Chains};
use crate::config::{WorkspaceSettings, WORKSPACE_SETTINGS};
use crate::error::DeployError;
#[cfg(feature = "ledger")]
use crate::ledger::get_ledger_info;
use cosm_utils::prelude::*;
use cosm_utils::{
    config::cfg::ChainConfig,
    signing_key::key::{Key, KeyringParams, UserKey},
};
use futures::executor::block_on;
use ibc_chain_registry::{chain::ChainData, constants::ALL_CHAINS, fetchable::Fetchable};
use inquire::{Confirm, CustomType, Select, Text};
use interactive_parse::InteractiveParseObj;
use lazy_static::lazy_static;
#[cfg(feature = "ledger")]
use ledger_utility::Connection;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ledger")]
use std::rc::Rc;
use std::{
    fs::{create_dir_all, OpenOptions},
    io::prelude::*,
    path::PathBuf,
    sync::Arc,
};
use tendermint_rpc::HttpClient;
use tokio::sync::RwLock;

use super::{ContractInfo, Env, UserSettings};

lazy_static! {
    pub static ref CONFIG: Arc<RwLock<Config>> = {
        match block_on(WORKSPACE_SETTINGS.read()).as_ref() {
            Some(settings) => Arc::new(RwLock::new(Config::load(settings).unwrap())),
            None => panic!("WORKSPACE_SETTINGS not set"),
        }
    };
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub settings: UserSettings,
    pub shell_completion_dir: Option<PathBuf>,
    pub chains: Chains,
    pub envs: Vec<Env>,
    pub keys: Vec<UserKey>,
}

impl Config {
    pub fn init(settings: &WorkspaceSettings) -> anyhow::Result<Config> {
        create_dir_all(settings.config_path.parent().expect("Invalid CONFIG_PATH"))?;
        let config = Config::default();
        Ok(config)
    }

    pub fn load(settings: &WorkspaceSettings) -> anyhow::Result<Config> {
        let config = match std::fs::read(settings.config_path.clone()) {
            Ok(serialized) => serde_json::from_slice(&serialized)?,
            Err(_) => return Err(DeployError::ConfigNotFound {}.into()),
        };

        Ok(config)
    }

    pub fn save(&self, settings: &WorkspaceSettings) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .open(settings.config_path.clone())?;
        let serialized = serde_json::to_vec_pretty(self)?;
        file.write_all(&serialized)?;
        Ok(())
    }

    pub fn get_active_env(&self) -> Result<&Env, DeployError> {
        match self.envs.iter().position(|x| x.is_active) {
            Some(index) => Ok(self.envs.get(index).ok_or(DeployError::EnvNotFound)?),
            None => Err(DeployError::EnvNotFound),
        }
    }

    pub fn get_active_env_mut(&mut self) -> anyhow::Result<&mut Env> {
        match self.envs.iter().position(|x| x.is_active) {
            Some(index) => Ok(self.envs.get_mut(index).ok_or(DeployError::EnvNotFound)?),
            None => Err(DeployError::EnvNotFound.into()),
        }
    }

    pub fn get_active_chain_info(&self) -> anyhow::Result<&ChainInfo> {
        let env = self.get_active_env()?;
        match self.chains.get(&env.chain_label) {
            Some(chain_info) => Ok(chain_info),
            None => Err(DeployError::ChainConfigNotFound.into()),
        }
    }

    #[allow(unused_mut)]
    pub async fn get_active_key(&self) -> anyhow::Result<UserKey> {
        let active_key_name = self.get_active_env()?.key_name.clone();
        let key = self.keys.iter().find(|x| x.name == active_key_name).ok_or(
            DeployError::KeyNotFound {
                key_name: active_key_name,
            },
        )?;
        let mut key = key.clone();
        #[cfg(feature = "ledger")]
        if let Key::Ledger { connection, .. } = &mut key.key {
            if connection.is_none() {
                *connection = Some(Rc::new(Connection::new().await));
            }
        }
        Ok(key)
    }

    pub fn add_chain_from(
        &mut self,
        label: String,
        chain_info: ChainInfo,
    ) -> Result<ChainInfo, DeployError> {
        match self.chains.contains_key(&label) {
            true => Err(DeployError::ChainAlreadyExists),
            false => {
                self.chains.insert(label, chain_info.clone());
                Ok(chain_info)
            }
        }
    }

    pub async fn add_chain(&mut self) -> anyhow::Result<(String, ChainInfo)> {
        let res = Select::new(
            "How would you like to input your chain information?",
            vec![
                "Add chain manually",
                "Add chain from cosmos chain registry (mainnets only)",
            ],
        )
        .prompt()?;
        let chain_info = match res {
            "Add chain manually" => ChainInfo::parse_to_obj()?,
            "Add chain from cosmos chain registry (mainnets only)" => {
                let chain_name = Select::new("Select chain", ALL_CHAINS.to_vec())
                    .prompt()?
                    .to_string();
                let chain_data = ChainData::fetch(chain_name.clone(), None).await?;
                let fee_token = if chain_data.fees.fee_tokens.len() == 1 {
                    chain_data.fees.fee_tokens[0].clone()
                } else {
                    let message = "Select fee token";
                    let options = chain_data
                        .fees
                        .fee_tokens
                        .iter()
                        .map(|x| x.denom.clone())
                        .collect();
                    let token_denom = Select::new(message, options).prompt()?;
                    chain_data
                        .fees
                        .fee_tokens
                        .iter()
                        .find(|x| x.denom == token_denom)
                        .unwrap()
                        .clone()
                };
                let rpc_endpoint = if chain_data.apis.rpc.len() == 1 {
                    chain_data.apis.rpc[0].clone().address
                } else {
                    let message = "Select RPC endpoint";
                    let options = chain_data
                        .apis
                        .rpc
                        .iter()
                        .map(|x| x.address.clone())
                        .collect();
                    Select::new(message, options).prompt()?
                };
                let derivation_path = format!("m/44'/{}'/0'/0/0", chain_data.slip44);
                let cfg = ChainConfig {
                    denom: fee_token.denom,
                    prefix: chain_data.bech32_prefix,
                    chain_id: chain_data.chain_id.to_string(),
                    derivation_path,
                    gas_price: fee_token.average_gas_price,
                    gas_adjustment: 1.3,
                };

                ChainInfo { cfg, rpc_endpoint }
            }
            _ => unreachable!(),
        };

        let label = Text::new("Enter a label for this chain")
            .with_default(&chain_info.cfg.chain_id)
            .prompt()?;

        self.add_chain_from(label.clone(), chain_info.clone())?;
        Ok((label, chain_info))
    }

    /// Adds or replaces a contract
    pub fn add_contract_from(
        &mut self,
        new_contract: ContractInfo,
    ) -> anyhow::Result<ContractInfo> {
        let env = self.get_active_env_mut()?;
        match env
            .contracts
            .iter_mut()
            .find(|x| x.name == new_contract.name)
        {
            Some(contract) => *contract = new_contract.clone(),
            None => env.contracts.push(new_contract.clone()),
        }
        Ok(new_contract)
    }

    pub fn add_contract(&mut self) -> anyhow::Result<ContractInfo> {
        let contract = ContractInfo::parse_to_obj()?;
        self.add_contract_from(contract.clone())?;
        Ok(contract)
    }

    pub fn get_contract_addr(&self, name: &str) -> anyhow::Result<&String> {
        let contract = self.get_contract(name)?;
        match &contract.addr {
            Some(addr) => Ok(addr),
            None => Err(DeployError::AddrNotFound {
                name: name.to_string(),
            }
            .into()),
        }
    }

    pub fn get_contract(&self, name: &str) -> anyhow::Result<&ContractInfo> {
        let env = self.get_active_env()?;
        env.contracts.iter().find(|x| x.name == name).ok_or(
            DeployError::ContractNotFound {
                contract_name: name.to_string(),
            }
            .into(),
        )
    }

    pub fn get_contract_mut(&mut self, name: &str) -> anyhow::Result<&mut ContractInfo> {
        let env = self.get_active_env_mut()?;
        env.contracts.iter_mut().find(|x| x.name == name).ok_or(
            DeployError::ContractNotFound {
                contract_name: name.to_string(),
            }
            .into(),
        )
    }

    pub fn add_key_from(&mut self, key: UserKey) -> Result<UserKey, DeployError> {
        if self.keys.iter().any(|x| x.name == key.name) {
            return Err(DeployError::KeyAlreadyExists);
        }
        self.keys.push(key.clone());
        Ok(key)
    }

    pub async fn add_key(&mut self) -> anyhow::Result<UserKey> {
        let key_type = Select::new("Select Key Type", vec!["Keyring", "Mnemonic"]).prompt()?;
        let key = match key_type {
            "Keyring" => {
                let params = KeyringParams::parse_to_obj()?;
                let service = Text::new("service?")
                    .with_help_message("Describe this key")
                    .prompt()
                    .unwrap();
                let entry = keyring::Entry::new(&service, &params.user)?;
                let password = inquire::Text::new("Mnemonic?").prompt()?;
                entry.set_password(password.as_str())?;
                Key::Keyring(params)
            }
            "Mnemonic" => Key::Mnemonic(Text::new("Enter Mnemonic").prompt()?),
            #[cfg(feature = "ledger")]
            "Ledger" => {
                let chain_info = self.get_active_chain_info()?;
                let connection = Connection::new().await;
                let info = get_ledger_info(&connection, chain_info).await?;
                Key::Ledger {
                    info,
                    connection: None,
                }
            }
            _ => panic!("should not happen"),
        };
        let name = Text::new("Key Name?").prompt()?;
        Ok(self.add_key_from(UserKey { name, key })?)
    }

    pub fn add_env(&mut self) -> anyhow::Result<&mut Env> {
        println!("Creating new deployment environment");
        let env_id = inquire::Text::new("Environment label?")
            .with_help_message("\"dev\", \"prod\", \"other\"")
            .prompt()?;
        if self.envs.iter().any(|x| x.env_id == env_id) {
            return Err(DeployError::EnvAlreadyExists.into());
        }
        let chain_label = inquire::Select::new(
            "Select which chain to activate",
            self.chains.keys().collect(),
        )
        .with_help_message("\"dev\", \"prod\", \"other\"")
        .prompt()?
        .clone();
        let key_name = inquire::Select::new(
            "Select key",
            self.keys.iter().map(|x| x.name.clone()).collect::<Vec<_>>(),
        )
        .with_help_message("\"my_key\"")
        .prompt()?;
        let env = Env {
            is_active: true,
            key_name,
            env_id,
            chain_label,
            contracts: vec![],
        };
        self.envs.push(env);
        if self.envs.len() > 1 {
            self.change_env()?
        }
        Ok(self.envs.last_mut().unwrap())
    }

    pub fn change_env(&mut self) -> anyhow::Result<()> {
        let env = Select::new("Select env to activate", self.envs.clone()).prompt()?;
        self.envs.iter_mut().for_each(|x| x.is_active = *x == env);
        Ok(())
    }

    pub async fn get_rpc_client(&mut self) -> anyhow::Result<HttpClient> {
        let chain_info = self.get_active_chain_info()?;
        let client = HttpClient::get_persistent_compat(chain_info.rpc_endpoint.as_str()).await?;
        Ok(client)
    }

    pub fn get_shell_completion_dir(&self) -> Option<&PathBuf> {
        self.shell_completion_dir.as_ref()
    }

    pub fn set_shell_completion_dir(
        &mut self,
        settings: &WorkspaceSettings,
    ) -> anyhow::Result<Option<&PathBuf>> {
        let ans = Confirm::new("Shell completion directory not found.\nWould you like to add one?")
            .with_default(true)
            .prompt()?;
        match ans {
            true => {
                let string =
                    CustomType::<String>::new("Enter you shell completion script directory.")
                        .prompt()?;
                let path = PathBuf::from(string);
                match path.is_dir() {
                    true => {
                        self.shell_completion_dir = Some(path.clone());
                        self.save(settings)?;
                        Ok(self.shell_completion_dir.as_ref())
                    }
                    false => Err(DeployError::InvalidDir.into()),
                }
            }
            false => Ok(None),
        }
    }
}

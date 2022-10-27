use std::error::Error;
use std::fmt::Display;
use std::path::{PathBuf};
use clap::Parser;
use cosmrs::bip32::{self, Language};
use cosmrs::crypto::secp256k1::{SigningKey};
use cosmrs::rpc::{Client, HttpClient};
use cosmrs::tendermint::chain::Id;
use inquire::{Confirm, CustomType};
use serde::{Serialize, Deserialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::prelude::*;
use lazy_static::lazy_static;
use clap_interactive::InteractiveParse;

use crate::error::DeployError;

lazy_static!{
    pub static ref CONFIG_PATH: PathBuf = PathBuf::from("deployment/.wasm-deploy/config.json");
    pub static ref BUILD_DIR: PathBuf = PathBuf::from("target/debug/");
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub shell_completion_dir: Option<PathBuf>,
    pub chains: Vec<ChainInfo>,
    pub envs: Vec<Env>,
}

impl Config {

    pub fn init() -> Result<Config, DeployError> {
        create_dir_all(CONFIG_PATH.parent().expect("Invalid CONFIG_PATH")).unwrap();
        let config = Config::default();
        config.save()?;
        Ok(config)
    }

    pub fn load() -> Result<Config, DeployError> {
        create_dir_all(CONFIG_PATH.parent().expect("Invalid CONFIG_PATH")).unwrap();
        let config = 
        match std::fs::read(CONFIG_PATH.as_path()) {
            Ok(serialized) => {
                serde_json::from_slice(&serialized)?
            },
            Err(_) => {
                Config::default()
            },
        };

        Ok(config)
    }

    pub fn save(&self) -> Result<(), DeployError> {
        create_dir_all(CONFIG_PATH.parent().expect("Invalid CONFIG_PATH")).unwrap();
        let mut file = OpenOptions::new().write(true).create(true).open(CONFIG_PATH.as_path())?;
        let serialized = serde_json::to_vec(self)?;
        file.write_all(&serialized)?;
        Ok(())
    }

    pub(crate) fn get_active_env(&mut self) -> Result<Env, DeployError> {
        match self.envs.iter().find(|x| x.is_active == true ) {
            Some(env) => Ok(env.clone()),
            None => {
                println!("No env found, creating one");
                Ok(self.add_env()?.clone())
            },
        }
    }

    pub(crate) fn get_active_chain_info(&mut self) -> Result<&ChainInfo, DeployError> {
        let env = self.get_active_env()?;
        match self.chains.iter().find(|x| x.chain_id == env.chain_id) {
            Some(chain_info) => Ok(chain_info),
            None => todo!(),
        }
    }

    pub(crate) fn get_private_key(&mut self) -> Result<SigningKey, DeployError> {
        let chain = self.get_active_chain_info()?;
        let mnemonic = bip32::Mnemonic::new(chain.mnemonic.clone(), Language::English).unwrap();
        let seed = mnemonic.to_seed("password");
        let child_path = "m/0/2147483647'/1/2147483646'";
        let signing_key = cosmrs::crypto::secp256k1::SigningKey::derive_from_path(&seed, &child_path.parse()?)?;
        //  let child_xprv = XPrv::derive_from_path(&seed, &child_path.parse()?)?;        
        // let signing_key: SigningKey = child_xprv.into();
        Ok( signing_key )
    }

    pub(crate) fn get_active_chain_id(&mut self) -> Result<Id, DeployError> {
        Ok(self.get_active_chain_info()?.chain_id.clone())
    }

    pub(crate) fn get_client(&mut self) -> Result<impl Client, DeployError> {
        let url = self.get_active_chain_info()?.url.clone();
        Ok(HttpClient::new(url.as_str()).unwrap())
    }

    pub(crate) fn add_chain_from(&mut self, chain_info: ChainInfo) -> Result<(), DeployError> {
        match self.chains.iter().any(|x| x.chain_id == chain_info.chain_id) {
            true => Err(DeployError::ChainAlreadyExists{}),
            false => Ok(self.chains.push(chain_info)),
        }
    }

    pub(crate) fn add_chain(&mut self) -> Result<(), DeployError> {
        let chain_info = ChainInfo::interactive_parse()?;
        self.add_chain_from(chain_info)?;
        Ok(())
    }

    pub(crate) fn add_env(&mut self) -> Result<&mut Env, DeployError> {
        println!("Creating new deployment environment");
        let env_id = inquire::Text::new("Environment label?")
        .with_help_message("\"dev\", \"prod\", \"other\"")
        .prompt().unwrap();
        if self.envs.iter().any(|x| x.env_id == env_id) {
            return Err(DeployError::EnvAlreadyExists{});
        }
        let chain_id = inquire::Select::new("Chain?", self.chains.clone())
        .with_help_message("\"dev\", \"prod\", \"other\"")
        .prompt().unwrap().chain_id;
        let env = Env {
            is_active: true,
            env_id,
            chain_id,
            contracts: vec![],
        };
        self.envs.push(env);
        Ok(self.envs.last_mut().unwrap())
    }
}

pub fn get_shell_completion_dir() -> Result<Option<PathBuf>, DeployError> {
    let mut config = Config::load()?;
    match config.shell_completion_dir {
        Some(shell_completion_path) => {
            Ok(Some(shell_completion_path))
        },
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
                        },
                        false => Err(DeployError::InvalidDir{}),
                    }
                },
                false => return Ok(None),
            }
        },
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Env {
    is_active: bool,
    env_id: String,
    chain_id: Id,
    contracts: Vec<ContractInfo>,
}

#[derive(Clone, Parser, Serialize, Deserialize)]
pub struct ChainInfo {
    chain_id: Id,
    url: String,
    mnemonic: String,
}

impl Display for ChainInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.chain_id.fmt(f)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    name: String,
    addr: Option<String>,
    code_id: Option<u64>
}
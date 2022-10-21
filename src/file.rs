use std::error::Error;
use std::path::{PathBuf};
use inquire::{Confirm, CustomType};
use serde::{Serialize, Deserialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::prelude::*;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref CONFIG_PATH: PathBuf = PathBuf::from("deployment/.wasm-deploy/config.json");
    pub static ref BUILD_DIR: PathBuf = PathBuf::from("target/release/");
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub shell_completion_dir: Option<PathBuf>
}

impl Config {

    pub fn init() -> Result<Config, Box<dyn Error>> {
        create_dir_all(CONFIG_PATH.parent().expect("Invalid CONFIG_PATH")).unwrap();
        let config = Config::default();
        config.save()?;
        Ok(config)
    }

    pub fn load() -> Result<Config, Box<dyn Error>> {
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

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        create_dir_all(CONFIG_PATH.parent().expect("Invalid CONFIG_PATH")).unwrap();
        let mut file = OpenOptions::new().write(true).create(true).open(CONFIG_PATH.as_path())?;
        let serialized = serde_json::to_vec(self)?;
        file.write_all(&serialized)?;
        Ok(())
    }
}

pub fn get_shell_completion_dir() -> Result<Option<PathBuf>, Box<dyn Error>> {
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
                        false => Err("Invalid path.".into()),
                    }
                },
                false => return Ok(None),
            }
        },
    }
}

pub enum Env {
    Dev,
    Prod
}
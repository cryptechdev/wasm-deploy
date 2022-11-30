use std::{env, process::Command};

use async_recursion::async_recursion;
use clap::{CommandFactory, Subcommand};
use clap_complete::{
    generate_to,
    shells::{Bash, Zsh},
};
use colored::Colorize;
use colored_json::to_colored_json_auto;
use inquire::{MultiSelect, Select};
use interactive_parse::traits::InteractiveParseObj;

#[cfg(wasm_cli)]
use crate::wasm_cli::wasm_cli_import_schemas;
use crate::{
    cli::{Cli, Commands},
    contract::{execute_set_up, execute_store, Contract, Execute, Query},
    cosmwasm::{Coin, CosmWasmClient},
    error::{DeployError, DeployResult},
    file::{get_shell_completion_dir, Config, BUILD_DIR},
};

#[derive(PartialEq)]
pub enum Status {
    Continue,
    Quit,
}

#[async_recursion(?Send)]
pub async fn execute_args<C, S>(cli: &Cli<C, S>) -> Result<Status, DeployError>
where
    C: Contract,
    S: Subcommand,
{
    match &cli.command {
        Commands::Update {} => update::<C, S>(),
        Commands::Init {} => init(),
        Commands::Build { contracts } => build(contracts),
        Commands::Chain { add, delete } => chain(add, delete),
        Commands::Key { add, delete } => key(add, delete),
        Commands::Contract { add, delete } => contract(add, delete),
        Commands::Deploy { contracts, no_build } => deploy(contracts, no_build).await,
        Commands::Env { add, delete, select } => execute_env(add, delete, select),
        Commands::Schema { contracts } => schemas(contracts),
        Commands::StoreCode { contracts } => store_code(contracts).await,
        Commands::Instantiate { contracts } => instantiate(contracts).await,
        Commands::Migrate { contracts } => migrate(contracts).await,
        Commands::Execute { contract } => execute::<C>(contract).await,
        Commands::ExecutePayload { contract, payload } => custom_execute(contract, payload).await,
        Commands::SetConfig { contracts } => set_config(contracts).await,
        Commands::Query { contract } => query::<C>(contract).await,
        Commands::SetUp { contracts } => set_up(contracts).await,
        Commands::CustomCommand { .. } => Ok(Status::Continue),
    }
}

pub fn init() -> DeployResult<Status> {
    let mut config = Config::init()?;
    config.add_key()?;
    config.add_chain()?;
    config.add_env()?;
    config.save()?;
    Ok(Status::Quit)
}

pub fn chain(add: &bool, delete: &bool) -> Result<Status, DeployError> {
    let mut config = Config::load()?;
    if *add {
        config.add_chain()?;
    } else if *delete {
        let all_chains = &mut config.chains;
        let chains = MultiSelect::new("Select which chains to delete", all_chains.clone()).prompt()?;
        for chain in chains {
            all_chains.retain(|x| x != &chain);
        }
    }
    config.save()?;
    Ok(Status::Quit)
}

pub fn key(add: &bool, delete: &bool) -> Result<Status, DeployError> {
    let mut config = Config::load()?;
    if *add {
        config.add_key()?;
    } else if *delete {
        let all_keys = &mut config.keys;
        let keys = MultiSelect::new("Select which keys to delete", all_keys.clone()).prompt()?;
        for key in keys {
            all_keys.retain(|x| x != &key);
        }
    }
    config.save()?;
    Ok(Status::Quit)
}

pub fn contract(add: &bool, delete: &bool) -> Result<Status, DeployError> {
    let mut config = Config::load()?;
    if *add {
        config.add_contract()?;
    } else if *delete {
        let env = config.get_active_env_mut()?;
        let all_contracts = &mut env.contracts;
        let contracts = MultiSelect::new("Select which contracts to delete", all_contracts.clone()).prompt()?;
        for contract in contracts {
            all_contracts.retain(|x| x != &contract);
        }
    }
    config.save()?;
    Ok(Status::Quit)
}

pub fn execute_env(add: &bool, delete: &bool, select: &bool) -> Result<Status, DeployError> {
    let mut config = Config::load()?;
    if *add {
        config.add_env()?;
    } else if *delete {
        let envs = MultiSelect::new("Select which envs to delete", config.envs.clone()).prompt()?;
        for env in envs {
            config.envs.retain(|x| x != &env);
        }
        let env = Select::new("Select which env to activate", config.envs.clone()).prompt()?;
        config.envs.iter_mut().for_each(|x| x.is_active = x == &env);
    } else if *select {
        let env = Select::new("Select which env to activate", config.envs.clone()).prompt()?;
        config.envs.iter_mut().for_each(|x| x.is_active = x == &env);
    }

    config.save()?;
    Ok(Status::Quit)
}

pub async fn deploy(contracts: &Vec<impl Contract>, no_build: &bool) -> Result<Status, DeployError> {
    if !no_build {
        build(contracts)?;
    }
    store_code(contracts).await?;
    instantiate(contracts).await?;
    set_config(contracts).await?;
    set_up(contracts).await?;
    Ok(Status::Continue)
}

pub fn update<C, S>() -> Result<Status, DeployError>
where
    C: Contract,
    S: Subcommand,
{
    Command::new("mv").arg("./target/debug/deploy").arg("./target/debug/deploy.old").spawn()?.wait()?;

    Command::new("cargo").arg("build").current_dir("./deployment").spawn()?.wait()?.exit_ok()?;

    generate_completions::<C, S>()?;

    Ok(Status::Quit)
}

fn generate_completions<C, S>() -> Result<(), DeployError>
where
    C: Contract,
    S: Subcommand,
{
    let shell_completion_dir = match get_shell_completion_dir()? {
        Some(shell_completion_dir) => shell_completion_dir,
        None => return Ok(()),
    };
    let string = env::var_os("SHELL").unwrap().into_string().unwrap();
    let (_, last_word) = string.rsplit_once('/').unwrap();
    let mut cmd = Cli::<C, S>::command();

    match last_word {
        "zsh" => {
            println!("Generating shell completion scripts for zsh");
            println!("Run source ~/.zshrc to update your completion scripts");

            let generated_file = generate_to(
                Zsh,
                &mut cmd,            // We need to specify what generator to use
                "deploy",            // We need to specify the bin name manually
                BUILD_DIR.as_path(), // We need to specify where to write to
            )?;

            let source_path = BUILD_DIR.join(generated_file.file_name().unwrap());
            let target_path = shell_completion_dir.join(generated_file.file_name().unwrap());
            Command::new("rm").arg(target_path.clone()).spawn()?.wait().ok();

            if Command::new("cp").arg(source_path).arg(target_path).spawn()?.wait()?.exit_ok().is_err() {
                println!("could not find {}", shell_completion_dir.to_str().unwrap());
            }
        }
        "bash" => {
            println!("generating shell completion scripts for bash");
            let generated_file = generate_to(
                Bash,
                &mut cmd,            // We need to specify what generator to use
                "deploy",            // We need to specify the bin name manually
                BUILD_DIR.as_path(), // We need to specify where to write to
            )?;

            let source_path = BUILD_DIR.join(generated_file.file_name().unwrap());
            let target_path = shell_completion_dir.join(generated_file.file_name().unwrap());

            if Command::new("cp").arg(source_path).arg(target_path).spawn()?.wait()?.exit_ok().is_err() {
                println!("could not find {}", shell_completion_dir.to_str().unwrap());
            }
        }
        _ => {
            return Err(DeployError::UnsupportedShell {});
        }
    }

    Ok(())
}

pub fn build(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    // Build contracts
    for contract in contracts {
        Command::new("cargo")
            .env("RUSTFLAGS", "-C link-arg=-s")
            .arg("build")
            .arg("--features")
            // TODO: remove for production.
            .arg("neptune_test")
            .arg("--release")
            .arg("--lib")
            .arg("--target=wasm32-unknown-unknown")
            .current_dir(format!("./contracts/{}", contract.name()))
            .spawn()?
            .wait()?
            .exit_ok()?;
    }

    Command::new("mkdir").arg("-p").arg("artifacts").spawn()?.wait()?;

    optimize(contracts)?;
    set_execute_permissions(contracts)?;

    Ok(Status::Quit)
}

pub fn schemas(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    // Generate schemas
    for contract in contracts {
        Command::new("cargo")
            .arg("schema")
            .current_dir(format!("./contracts/{}", contract.name()))
            .spawn()?
            .wait()?
            .exit_ok()?;
    }

    #[cfg(wasm_cli)]
    // Import schemas
    for contract in contracts {
        wasm_cli_import_schemas(&contract.name())?;
    }

    Ok(Status::Quit)
}

pub fn optimize(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    // Optimize contracts
    let mut handles = vec![];
    for contract in contracts {
        let name = contract.name();
        println!("Optimizing {} contract", name);
        handles.push(
            Command::new("wasm-opt")
                .arg("-Os")
                .arg("-o")
                .arg(format!("artifacts/{}.wasm", name))
                .arg(format!("target/wasm32-unknown-unknown/release/{}.wasm", name))
                .spawn()?,
        );
    }
    handles.iter_mut().for_each(|x| {
        x.wait().unwrap();
    });
    Ok(Status::Quit)
}

pub fn set_execute_permissions(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    // change mod
    for contract in contracts {
        let name = contract.name();
        Command::new("chmod").arg("+x").arg(format!("artifacts/{}.wasm", name));
    }
    Ok(Status::Quit)
}

pub async fn store_code(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        execute_store(contract).await?
    }
    Ok(Status::Quit)
}

pub async fn instantiate(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        crate::contract::execute_instantiate(contract).await?;
    }
    Ok(Status::Quit)
}

pub async fn migrate(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    build(contracts)?;
    store_code(contracts).await?;
    for contract in contracts {
        crate::contract::execute_migrate(contract).await?;
    }
    Ok(Status::Quit)
}

pub async fn set_config(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        crate::contract::execute_set_config(contract).await?;
    }
    Ok(Status::Quit)
}

pub async fn set_up(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        execute_set_up(contract).await?;
    }
    Ok(Status::Quit)
}

pub async fn execute<C: Contract>(contract: &impl Contract) -> Result<Status, DeployError> {
    let e = C::ExecuteMsg::parse(contract)?; //parse(contract);
    crate::contract::execute(&e).await?;
    Ok(Status::Quit)
}

pub async fn custom_execute<C: Contract>(contract: &C, string: &str) -> Result<Status, DeployError> {
    println!("Executing {}", contract.name());
    let mut config = Config::load()?;
    let value: serde_json::Value = serde_json::from_str(string)?;
    let color = to_colored_json_auto(&value)?;
    println!("{}", color);
    let payload = serde_json::to_vec(&value)?;
    let chain_info = config.get_active_chain_info()?;
    let client = CosmWasmClient::new(chain_info)?;
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let coins = Vec::<Coin>::parse_to_obj()?;

    let response = client.execute(contract_addr, payload, &config.get_active_key()?, coins).await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.res.gas_wanted.to_string().green(),
        response.res.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.tx_hash.purple());

    Ok(Status::Quit)
}

pub async fn query<C: Contract>(contract: &impl Contract) -> Result<Status, DeployError> {
    let q = C::QueryMsg::parse(contract)?; //parse(contract);
    crate::contract::query(&q).await?;
    Ok(Status::Quit)
}

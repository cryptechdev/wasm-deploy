use std::{env, process::Command, str::FromStr};

use async_recursion::async_recursion;
use clap::{CommandFactory, Subcommand};
use clap_complete::{
    generate_to,
    shells::{Bash, Zsh},
};
use colored::{self, Colorize};
use colored_json::to_colored_json_auto;
use cosm_tome::{
    chain::{coin::Coin, request::TxOptions},
    clients::{client::CosmTome, cosmos_grpc::CosmosgRPC},
    modules::{auth::model::Address, cosmwasm::model::ExecRequest},
};
use inquire::{MultiSelect, Select};
use interactive_parse::traits::InteractiveParseObj;
use log::info;

#[cfg(wasm_cli)]
use crate::wasm_cli::wasm_cli_import_schemas;
use crate::{
    cli::{Cli, Commands},
    contract::{Contract, Cw20Hook, Execute, Query},
    error::{DeployError, DeployResult},
    file::{get_shell_completion_dir, Config, BUILD_DIR},
    wasm_msg::{msg_contract, DeploymentStage},
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
        Commands::Init {} => init().await,
        Commands::Build { contracts } => build(contracts),
        Commands::Chain { add, delete } => chain(add, delete),
        Commands::Key { add, delete } => key(add, delete).await,
        Commands::Contract { add, delete } => contract(add, delete),
        Commands::Deploy { contracts, no_build } => deploy(contracts, no_build).await,
        Commands::Env { add, delete, select } => execute_env(add, delete, select),
        Commands::Schema { contracts } => schemas(contracts),
        Commands::StoreCode { contracts } => store_code(contracts).await,
        Commands::Instantiate { contracts } => instantiate(contracts).await,
        Commands::Migrate { contracts } => migrate(contracts).await,
        Commands::Execute { contract } => execute::<C>(contract).await,
        Commands::Cw20Send { contract } => cw20_send::<C>(contract).await,
        Commands::Cw20Transfer {} => cw20_transfer().await,
        Commands::ExecutePayload { contract, payload } => custom_execute(contract, payload).await,
        Commands::SetConfig { contracts } => set_config(contracts).await,
        Commands::Query { contract } => query::<C>(contract).await,
        Commands::SetUp { contracts } => set_up(contracts).await,
        Commands::CustomCommand { .. } => Ok(Status::Continue),
    }
}

pub async fn init() -> DeployResult<Status> {
    info!("Initializing deploy");
    let mut config = Config::init()?;
    config.add_key().await?;
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
        let chains_to_remove = MultiSelect::new(
            "Select which chains to delete",
            all_chains.iter().map(|x| x.chain_id.clone()).collect::<Vec<_>>(),
        )
        .prompt()?;
        for chain in chains_to_remove {
            all_chains.retain(|x| x.chain_id != chain);
        }
    }
    config.save()?;
    Ok(Status::Quit)
}

pub async fn key(add: &bool, delete: &bool) -> Result<Status, DeployError> {
    let mut config = Config::load()?;
    if *add {
        config.add_key().await?;
    } else if *delete {
        let all_keys = &mut config.keys;
        let keys_to_remove = MultiSelect::new(
            "Select which keys to delete",
            all_keys.iter().map(|x| x.name.clone()).collect::<Vec<_>>(),
        )
        .prompt()?;
        for key in keys_to_remove {
            all_keys.retain(|x| x.name != key);
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
        println!("Optimizing {name} contract");
        handles.push(
            Command::new("wasm-opt")
                .arg("-Os")
                .arg("-o")
                .arg(format!("artifacts/{name}.wasm"))
                .arg(format!("target/wasm32-unknown-unknown/release/{name}.wasm"))
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
        Command::new("chmod").arg("+x").arg(format!("artifacts/{name}.wasm"));
    }
    Ok(Status::Quit)
}

pub async fn store_code(contracts: &[impl Contract]) -> Result<Status, DeployError> {
    let chunks = contracts.chunks(2);
    for chunk in chunks {
        msg_contract(chunk, DeploymentStage::StoreCode).await?;
    }
    Ok(Status::Quit)
}

pub async fn instantiate(contracts: &[impl Contract]) -> Result<Status, DeployError> {
    msg_contract(contracts, DeploymentStage::Instantiate).await?;
    Ok(Status::Quit)
}

pub async fn migrate(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    build(contracts)?;
    store_code(contracts).await?;
    msg_contract(contracts, DeploymentStage::Migrate).await?;
    Ok(Status::Quit)
}

pub async fn set_config(contracts: &[impl Contract]) -> Result<Status, DeployError> {
    msg_contract(contracts, DeploymentStage::SetConfig).await?;
    Ok(Status::Quit)
}

pub async fn set_up(contracts: &[impl Contract]) -> Result<Status, DeployError> {
    msg_contract(contracts, DeploymentStage::SetUp).await?;
    Ok(Status::Quit)
}

pub async fn execute<C: Contract>(contract: &impl Contract) -> Result<Status, DeployError> {
    let e = C::ExecuteMsg::parse(contract)?; //parse(contract);
    crate::contract::execute(&e).await?;
    Ok(Status::Quit)
}

pub async fn cw20_send<C: Contract>(contract: &impl Contract) -> Result<Status, DeployError> {
    let h = C::Cw20HookMsg::parse(contract)?; //parse(contract);
    crate::contract::cw20_send(&h).await?;
    Ok(Status::Quit)
}

pub async fn cw20_transfer() -> Result<Status, DeployError> {
    crate::contract::cw20_transfer().await?;
    Ok(Status::Quit)
}

pub async fn custom_execute<C: Contract>(contract: &C, string: &str) -> Result<Status, DeployError> {
    println!("Executing {}", contract.name());
    let mut config = Config::load()?;
    let value: serde_json::Value = serde_json::from_str(string)?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    let msg = serde_json::to_vec(&value)?;
    let key = config.get_active_key().await?;

    let chain_info = config.get_active_chain_info()?;
    let client = CosmosgRPC::new(chain_info.grpc_endpoint.clone().unwrap());
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

    Ok(Status::Quit)
}

pub async fn query<C: Contract>(contract: &impl Contract) -> Result<Status, DeployError> {
    let q = C::QueryMsg::parse(contract)?; //parse(contract);
    crate::contract::query(&q).await?;
    Ok(Status::Quit)
}

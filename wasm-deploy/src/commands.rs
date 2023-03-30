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
    clients::{client::CosmTome, tendermint_rpc::TendermintRPC},
    modules::{auth::model::Address, cosmwasm::model::ExecRequest},
};
use inquire::{MultiSelect, Select};
use interactive_parse::traits::InteractiveParseObj;
use log::info;

#[cfg(wasm_cli)]
use crate::wasm_cli::wasm_cli_import_schemas;
use crate::{
    cli::{Cli, Commands},
    contract::Contract,
    cw20::{cw20_execute, cw20_instantiate, cw20_send},
    deployment::{execute_deployment, DeploymentStage},
    error::DeployError,
    execute::execute_contract,
    file::{get_shell_completion_dir, Config},
    query::{cw20_query, query_contract},
    settings::WorkspaceSettings,
    utils::BIN_NAME,
};
use std::fmt::Debug;

#[async_recursion(?Send)]
pub async fn execute_args<C, S>(settings: &WorkspaceSettings, cli: &Cli<C, S>) -> anyhow::Result<()>
where
    C: Contract + Clone,
    S: Subcommand + Clone + Debug,
{
    info!("Executing args: {:#?}", cli);
    std::env::set_current_dir(settings.workspace_root.clone()).unwrap();
    // *WORKSPACE_SETTINGS.lock().await = Some(settings.clone());
    match &cli.command {
        Commands::Update {} => update::<C, S>(settings)?,
        Commands::Init {} => init(settings).await?,
        Commands::Build { contracts } => build(settings, contracts, &cli.cargo_args)?,
        Commands::Chain { add, delete } => chain(settings, add, delete)?,
        Commands::Key { add, delete } => key(settings, add, delete).await?,
        Commands::Contract { add, delete } => contract(settings, add, delete)?,
        Commands::Deploy {
            contracts,
            no_build,
        } => deploy(settings, contracts, no_build, &cli.cargo_args).await?,
        Commands::Env {
            add,
            delete,
            select,
            id,
        } => execute_env(settings, add, delete, select, id)?,
        Commands::Schema { contracts } => schemas(contracts)?,
        Commands::StoreCode { contracts } => store_code(settings, contracts).await?,
        Commands::Instantiate { contracts } => instantiate(settings, contracts).await?,
        Commands::Migrate { contracts } => migrate(settings, contracts, &cli.cargo_args).await?,
        Commands::Execute { contract } => execute_contract(settings, contract).await?,
        Commands::Cw20Send { contract } => cw20_send(settings, contract).await?,
        Commands::Cw20Execute {} => cw20_execute(settings).await?,
        Commands::Cw20Query {} => {
            cw20_query(settings).await?;
        }
        Commands::Cw20Instantiate {} => cw20_instantiate(settings).await?,
        Commands::ExecutePayload { contract, payload } => {
            custom_execute(settings, contract, payload).await?
        }
        Commands::SetConfig { contracts } => set_config(settings, contracts).await?,
        Commands::Query { contract } => {
            query_contract(settings, contract).await?;
        }
        Commands::SetUp { contracts } => set_up(settings, contracts).await?,
        Commands::Custom(..) => {}
    };
    Ok(())
}

pub async fn init(settings: &WorkspaceSettings) -> anyhow::Result<()> {
    info!("Initializing wasm-deploy");
    let mut config = Config::init(settings)?;
    config.add_key().await?;
    config.add_chain()?;
    config.add_env()?;
    config.save(settings)?;
    Ok(())
}

pub fn chain(settings: &WorkspaceSettings, add: &bool, delete: &bool) -> anyhow::Result<()> {
    let mut config = Config::load(settings)?;
    if *add {
        config.add_chain()?;
    } else if *delete {
        let all_chains = &mut config.chains;
        let chains_to_remove = MultiSelect::new(
            "Select which chains to delete",
            all_chains
                .iter()
                .map(|x| x.chain_id.clone())
                .collect::<Vec<_>>(),
        )
        .prompt()?;
        for chain in chains_to_remove {
            all_chains.retain(|x| x.chain_id != chain);
        }
    }
    config.save(settings)?;
    Ok(())
}

pub async fn key(settings: &WorkspaceSettings, add: &bool, delete: &bool) -> anyhow::Result<()> {
    let mut config = Config::load(settings)?;
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
    config.save(settings)?;
    Ok(())
}

pub fn contract(settings: &WorkspaceSettings, add: &bool, delete: &bool) -> anyhow::Result<()> {
    let mut config = Config::load(settings)?;
    if *add {
        config.add_contract()?;
    } else if *delete {
        let env = config.get_active_env_mut()?;
        let all_contracts = &mut env.contracts;
        let contracts =
            MultiSelect::new("Select which contracts to delete", all_contracts.clone()).prompt()?;
        for contract in contracts {
            all_contracts.retain(|x| x != &contract);
        }
    }
    config.save(settings)?;
    Ok(())
}

pub fn execute_env(
    settings: &WorkspaceSettings,
    add: &bool,
    delete: &bool,
    select: &bool,
    id: &bool,
) -> anyhow::Result<()> {
    let mut config = Config::load(settings)?;
    if *add {
        config.add_env()?;
        config.save(settings)?;
    } else if *delete {
        let envs = MultiSelect::new("Select which envs to delete", config.envs.clone()).prompt()?;
        for env in envs {
            config.envs.retain(|x| x != &env);
        }
        let env = Select::new("Select which env to activate", config.envs.clone()).prompt()?;
        config.envs.iter_mut().for_each(|x| x.is_active = x == &env);
        config.save(settings)?;
    } else if *select {
        let env = Select::new("Select which env to activate", config.envs.clone()).prompt()?;
        config.envs.iter_mut().for_each(|x| x.is_active = x == &env);
        config.save(settings)?;
    } else if *id {
        println!("{}", config.get_active_env()?.env_id);
    } else {
        println!(
            "{}",
            to_colored_json_auto(&serde_json::to_value(config.get_active_env()?)?)?
        );
    }
    Ok(())
}

pub async fn deploy(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
    no_build: &bool,
    cargo_args: &[String],
) -> anyhow::Result<()> {
    if !no_build {
        build(settings, contracts, cargo_args)?;
    }
    store_code(settings, contracts).await?;
    instantiate(settings, contracts).await?;
    set_config(settings, contracts).await?;
    set_up(settings, contracts).await?;
    Ok(())
}

pub fn update<C, S>(settings: &WorkspaceSettings) -> anyhow::Result<()>
where
    C: Contract + Clone,
    S: Subcommand + Clone + Debug,
{
    Command::new("cargo")
        .arg("install")
        .arg("--path")
        .arg(settings.deployment_dir.clone())
        .spawn()?
        .wait()?;

    generate_completions::<C, S>(settings)?;

    Ok(())
}

pub fn generate_completions<C, S>(settings: &WorkspaceSettings) -> anyhow::Result<()>
where
    C: Contract + Clone,
    S: Subcommand + Clone + Debug,
{
    let shell_completion_dir = match get_shell_completion_dir(settings)? {
        Some(shell_completion_dir) => shell_completion_dir,
        None => return Ok(()),
    };
    let string = env::var_os("SHELL")
        .expect("Failed parsing SHELL string")
        .into_string()
        .unwrap();
    let (_, last_word) = string
        .rsplit_once('/')
        .expect("Failed parsing SHELL string");
    let mut cmd = Cli::<C, S>::command();

    match last_word {
        "zsh" => {
            println!("Generating shell completion scripts for zsh");
            println!("Run source ~/.zshrc to update your completion scripts");

            let generated_file = generate_to(
                Zsh,
                &mut cmd,                    // We need to specify what generator to use
                BIN_NAME.to_string(),        // We need to specify the bin name manually
                settings.target_dir.clone(), // We need to specify where to write to
            )?;

            let source_path = settings
                .target_dir
                .join(generated_file.file_name().unwrap());
            let target_path = shell_completion_dir.join(generated_file.file_name().unwrap());

            if Command::new("cp")
                .arg(source_path)
                .arg(target_path)
                .spawn()?
                .wait()
                .is_err()
            {
                println!("could not find {}", shell_completion_dir.to_str().unwrap());
            }
        }
        "bash" => {
            println!("generating shell completion scripts for bash");
            let generated_file = generate_to(
                Bash,
                &mut cmd,                    // We need to specify what generator to use
                BIN_NAME.to_string(),        // We need to specify the bin name manually
                settings.target_dir.clone(), // We need to specify where to write to
            )?;

            let source_path = settings
                .target_dir
                .join(generated_file.file_name().unwrap());
            let target_path = shell_completion_dir.join(generated_file.file_name().unwrap());

            if Command::new("cp")
                .arg(source_path)
                .arg(target_path)
                .spawn()?
                .wait()
                .is_err()
            {
                println!("could not find {}", shell_completion_dir.to_str().unwrap());
            }
        }
        _ => {
            return Err(DeployError::UnsupportedShell {}.into());
        }
    }

    Ok(())
}

pub fn build(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
    cargo_args: &[String],
) -> anyhow::Result<()> {
    // Build contracts
    for contract in contracts {
        Command::new("cargo")
            .env("RUSTFLAGS", "-C link-arg=-s")
            .arg("build")
            .arg("--release")
            .arg("--lib")
            .arg("--target=wasm32-unknown-unknown")
            .args(cargo_args)
            .current_dir(contract.path())
            .spawn()?
            .wait()?;
    }

    Command::new("mkdir")
        .arg("-p")
        .arg(settings.artifacts_dir.clone())
        .spawn()?
        .wait()?;

    optimize(settings, contracts)?;
    set_execute_permissions(settings, contracts)?;

    Ok(())
}

pub fn schemas(contracts: &[impl Contract]) -> anyhow::Result<()> {
    // Generate schemas
    for contract in contracts {
        Command::new("cargo")
            .arg("schema")
            .current_dir(contract.path())
            .spawn()?
            .wait()?;
    }

    #[cfg(wasm_cli)]
    // Import schemas
    for contract in contracts {
        wasm_cli_import_schemas(&contract.name())?;
    }

    Ok(())
}

pub fn optimize(settings: &WorkspaceSettings, contracts: &[impl Contract]) -> anyhow::Result<()> {
    // Optimize contracts
    let mut handles = vec![];
    for contract in contracts {
        let name = contract.name();
        println!("Optimizing {name} contract");
        handles.push(
            Command::new("wasm-opt")
                .arg("-Oz")
                .arg("-o")
                .arg(
                    settings.artifacts_dir.join(format!("{}.wasm", name)), // .with_file_name(name.clone())
                                                                           // .with_extension("wasm"),
                )
                .arg(
                    settings
                        .target_dir
                        .join(format!("wasm32-unknown-unknown/release/{name}.wasm")),
                )
                .spawn()?,
        );
    }
    handles.iter_mut().for_each(|x| {
        x.wait().unwrap();
    });
    for contract in contracts {
        let name = contract.name();
        handles.push(
            Command::new("gzip")
                .arg("-f")
                .arg("-k")
                .arg(settings.artifacts_dir.join(format!("{name}.wasm")))
                .spawn()?,
        );
    }
    handles.iter_mut().for_each(|x| {
        x.wait().unwrap();
    });
    Ok(())
}

pub fn set_execute_permissions(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
) -> anyhow::Result<()> {
    // change mod
    for contract in contracts {
        let name = contract.name();
        Command::new("chmod")
            .arg("+x")
            .arg(settings.artifacts_dir.join(format!("{name}.wasm")));
    }
    Ok(())
}

pub async fn store_code(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
) -> anyhow::Result<()> {
    let chunk_size = Config::load(settings)?.settings.store_code_chunk_size;
    let chunks = contracts.chunks(chunk_size);
    for chunk in chunks {
        execute_deployment(settings, chunk, DeploymentStage::StoreCode).await?;
    }
    Ok(())
}

pub async fn instantiate(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, DeploymentStage::Instantiate).await?;
    execute_deployment(settings, contracts, DeploymentStage::ExternalInstantiate).await?;
    Ok(())
}

pub async fn migrate(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
    cargo_args: &[String],
) -> anyhow::Result<()> {
    build(settings, contracts, cargo_args)?;
    store_code(settings, contracts).await?;
    execute_deployment(settings, contracts, DeploymentStage::Migrate).await?;
    Ok(())
}

pub async fn set_config(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, DeploymentStage::SetConfig).await?;
    Ok(())
}

pub async fn set_up(
    settings: &WorkspaceSettings,
    contracts: &[impl Contract],
) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, DeploymentStage::SetUp).await?;
    Ok(())
}

pub async fn custom_execute<C: Contract>(
    settings: &WorkspaceSettings,
    contract: &C,
    string: &str,
) -> anyhow::Result<()> {
    println!("Executing {}", contract.name());
    let mut config = Config::load(settings)?;
    let value: serde_json::Value = serde_json::from_str(string)?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    let msg = serde_json::to_vec(&value)?;
    let key = config.get_active_key().await?;

    let chain_info = config.get_active_chain_info()?;
    let client = TendermintRPC::new(
        &chain_info
            .rpc_endpoint
            .clone()
            .ok_or(DeployError::MissingRpc)?,
    )?;
    let cosm_tome = CosmTome::new(chain_info, client);
    let contract_addr = config.get_contract_addr_mut(&contract.to_string())?.clone();
    let funds = Vec::<Coin>::parse_to_obj()?;
    let tx_options = TxOptions {
        timeout_height: None,
        fee: None,
        memo: "wasm_deploy".into(),
    };
    let req = ExecRequest {
        msg,
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

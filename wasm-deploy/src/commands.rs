use std::ffi::OsString;
use std::{env, process::Command, str::FromStr, sync::Arc};

use async_recursion::async_recursion;
use clap::{CommandFactory, Subcommand};
use clap_complete::{
    generate_to,
    shells::{Bash, Zsh},
};
use colored::{self, Colorize};
use colored_json::to_colored_json_auto;
use cosm_utils::prelude::*;
use cosm_utils::{
    chain::{coin::Coin, request::TxOptions},
    modules::{auth::model::Address, cosmwasm::model::ExecRequest},
};
#[cfg(feature = "wasm_opt")]
use futures::future::join_all;
use inquire::{MultiSelect, Select};
use interactive_parse::InteractiveParseObj;
use log::info;
use tendermint_rpc::client::CompatMode;
use tendermint_rpc::{HttpClient, HttpClientUrl};
#[cfg(feature = "wasm_opt")]
use tokio::task::spawn_blocking;
#[cfg(feature = "wasm_opt")]
use wasm_opt::integration::run_from_command_args;

use crate::config::WorkspaceSettings;
#[cfg(wasm_cli)]
use crate::wasm_cli::wasm_cli_import_schemas;
use crate::{
    cli::{Cli, Commands},
    contract::Deploy,
    cw20::{cw20_execute, cw20_instantiate, cw20_send},
    deployment::{execute_deployment, DeploymentStage},
    error::DeployError,
    execute::execute_contract,
    config::{Config, CONFIG, WORKSPACE_SETTINGS},
    query::{cw20_query, query_contract},
    utils::BIN_NAME,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fmt::Debug;
use std::fs::{create_dir, remove_file, File};
use std::io::{copy, BufReader};
use std::path::{Path, PathBuf};

#[async_recursion(?Send)]
pub async fn execute_args<C, S>(settings: &WorkspaceSettings, cli: &Cli<C, S>) -> anyhow::Result<()>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    info!("Executing args: {:#?}", cli);
    std::env::set_current_dir(settings.workspace_root.clone())?;
    *WORKSPACE_SETTINGS.write().await = Some(Arc::new(settings.clone()));
    match &cli.command {
        Commands::Update {} => update::<C, S>(settings).await?,
        Commands::Init {} => init(settings).await?,
        Commands::Build { contracts } => build(settings, contracts, &cli.cargo_args).await?,
        Commands::Chain { add, delete } => chain(settings, add, delete).await?,
        Commands::Key { add, delete } => key(settings, add, delete).await?,
        Commands::Contract { add, delete } => contract(settings, add, delete).await?,
        Commands::Deploy {
            contracts,
            no_build,
        } => deploy(settings, contracts, no_build, &cli.cargo_args).await?,
        Commands::Env {
            add,
            delete,
            select,
            id,
        } => execute_env(settings, add, delete, select, id).await?,
        Commands::Schema { contracts } => schemas(contracts)?,
        Commands::StoreCode { contracts } => store_code(settings, contracts).await?,
        Commands::Instantiate {
            contracts,
            interactive,
        } => instantiate(settings, contracts, *interactive).await?,
        Commands::Migrate {
            contracts,
            interactive,
        } => migrate(settings, contracts, *interactive, &cli.cargo_args).await?,
        Commands::Execute { contract } => execute_contract(contract).await?,
        Commands::Cw20Send { contract } => cw20_send(contract).await?,
        Commands::Cw20Execute {} => cw20_execute().await?,
        Commands::Cw20Query {} => {
            cw20_query().await?;
        }
        Commands::Cw20Instantiate {} => cw20_instantiate().await?,
        Commands::ExecutePayload { contract, payload } => custom_execute(contract, payload).await?,
        Commands::SetConfig { contracts } => set_config(settings, contracts).await?,
        Commands::Query { contract } => {
            query_contract(contract).await?;
        }
        Commands::SetUp { contracts } => set_up(settings, contracts).await?,
        Commands::Custom(..) => {}
    };
    Ok(())
}

pub async fn init(settings: &WorkspaceSettings) -> anyhow::Result<()> {
    info!("Initializing wasm-deploy");
    let mut config = Config::init(settings)?;
    config.add_chain().await?;
    config.add_key().await?;
    config.add_env()?;
    config.save(settings)?;
    Ok(())
}

pub async fn chain(settings: &WorkspaceSettings, add: &bool, delete: &bool) -> anyhow::Result<()> {
    let mut config = CONFIG.write().await;
    if *add {
        config.add_chain().await?;
    } else if *delete {
        let chains = config.chains.clone();
        let chains_to_remove = MultiSelect::new(
            "Select which chains to delete",
            chains.keys().collect(),
        )
        .prompt()?;
        for chain in chains_to_remove {
            config.chains.remove(chain);
        }
    }
    config.save(settings)?;
    Ok(())
}

pub async fn key(settings: &WorkspaceSettings, add: &bool, delete: &bool) -> anyhow::Result<()> {
    let mut config = CONFIG.write().await;
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

pub async fn contract(
    settings: &WorkspaceSettings,
    add: &bool,
    delete: &bool,
) -> anyhow::Result<()> {
    let mut config = CONFIG.write().await;
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

pub async fn execute_env(
    settings: &WorkspaceSettings,
    add: &bool,
    delete: &bool,
    select: &bool,
    id: &bool,
) -> anyhow::Result<()> {
    let mut config = CONFIG.write().await;
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
    contracts: &[impl Deploy],
    no_build: &bool,
    cargo_args: &[String],
) -> anyhow::Result<()> {
    if !no_build {
        build(settings, contracts, cargo_args).await?;
    }
    store_code(settings, contracts).await?;
    instantiate(settings, contracts, false).await?;
    set_config(settings, contracts).await?;
    set_up(settings, contracts).await?;
    Ok(())
}

pub async fn update<C, S>(settings: &WorkspaceSettings) -> anyhow::Result<()>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    Command::new("cargo")
        .arg("install")
        // .arg("--debug")
        .arg("--path")
        .arg(settings.deployment_dir.clone())
        .spawn()?
        .wait()?;

    generate_completions::<C, S>(settings).await?;

    Ok(())
}

pub async fn generate_completions<C, S>(settings: &WorkspaceSettings) -> anyhow::Result<()>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    let mut config = CONFIG.write().await;

    let shell_completion_dir = match config.get_shell_completion_dir() {
        Some(shell_completion_dir) => shell_completion_dir,
        None => match config.set_shell_completion_dir(settings)? {
            Some(shell_completion_dir) => shell_completion_dir,
            None => return Ok(()),
        },
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

pub async fn build(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    cargo_args: &[String],
) -> anyhow::Result<()> {
    // Build contracts
    for contract in contracts {
        Command::new("cargo")
            .env("RUSTFLAGS", "-C link-arg=-s")
            .arg("+stable")
            .arg("build")
            .arg("--release")
            .arg("--lib")
            .arg("--target=wasm32-unknown-unknown")
            .args(cargo_args)
            .current_dir(contract.path())
            .spawn()?
            .wait()?;
    }

    if !Path::exists(Path::new(settings.artifacts_dir.as_path())) {
        create_dir(settings.artifacts_dir.as_path())?;
    }

    optimize(settings, contracts).await?;
    set_execute_permissions(settings, contracts)?;

    Ok(())
}

pub fn schemas(contracts: &[impl Deploy]) -> anyhow::Result<()> {
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

// TODO: contracts with the same code are reprocessed. This is not optimal.
pub async fn optimize(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
) -> anyhow::Result<()> {
    // Optimize contracts
    let mut handles = vec![];
    for contract in contracts {
        let name = contract.name();
        let bin_name = contract.bin_name();
        println!("Optimizing {name} contract");
        #[cfg(feature = "wasm_opt")]
        {
            let mut command = wasm_opt::integration::Command::new("wasm-opt");
            command
                .arg("-Oz")
                .arg("-o")
                .arg(settings.artifacts_dir.join(format!("{}.wasm", bin_name)))
                .arg(
                    settings
                        .target_dir
                        .join(format!("wasm32-unknown-unknown/release/{bin_name}.wasm")),
                );
            handles.push({
                spawn_blocking(move || {
                    run_from_command_args(command).unwrap();
                })
            })
        }
        #[cfg(not(feature = "wasm_opt"))]
        {
            let mut command = Command::new("wasm-opt");
            handles.push(
                command
                    .arg("-Oz")
                    .arg("-o")
                    .arg(settings.artifacts_dir.join(format!("{}.wasm", bin_name)))
                    .arg(
                        settings
                            .target_dir
                            .join(format!("wasm32-unknown-unknown/release/{bin_name}.wasm")),
                    )
                    .spawn()?,
            );
        }
    }
    #[cfg(feature = "wasm_opt")]
    join_all(handles).await;
    #[cfg(not(feature = "wasm_opt"))]
    handles.iter_mut().for_each(|x| {
        x.wait().unwrap();
    });

    let mut task_handles = vec![];
    for contract in contracts {
        let bin_name = contract.bin_name();
        let bin_pathbuf = settings.artifacts_dir.join(format!("{bin_name}.wasm"));
        task_handles.push(gzip_file(bin_pathbuf));
    }

    for handle in task_handles {
        handle.await?;
    }

    Ok(())
}

pub async fn gzip_file(src: PathBuf) -> anyhow::Result<File> {
    let src_path: &Path = src.as_path();
    let mut new_extension = OsString::from(src_path.extension().unwrap());
    new_extension.push(".gz");
    let dst_pathbuf = src_path.with_extension(new_extension);
    let dst = dst_pathbuf.as_path();

    let mut input = BufReader::new(File::open(src_path)?);
    if Path::exists(dst) {
        remove_file(dst)?;
    }

    let output = File::create(dst)?;
    let mut encoder = GzEncoder::new(output, Compression::default());
    copy(&mut input, &mut encoder)?;
    Ok(encoder.finish()?)
}

pub fn set_execute_permissions(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
) -> anyhow::Result<()> {
    // change mod
    for contract in contracts {
        let bin_name = contract.bin_name();
        Command::new("chmod")
            .arg("+x")
            .arg(settings.artifacts_dir.join(format!("{bin_name}.wasm")));
    }
    Ok(())
}

pub async fn store_code(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
) -> anyhow::Result<()> {
    let chunk_size = CONFIG.read().await.settings.store_code_chunk_size;
    let chunks = contracts.chunks(chunk_size);
    for chunk in chunks {
        execute_deployment(settings, chunk, DeploymentStage::StoreCode).await?;
    }
    Ok(())
}

pub async fn instantiate(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    interactive: bool,
) -> anyhow::Result<()> {
    execute_deployment(
        settings,
        contracts,
        DeploymentStage::Instantiate { interactive },
    )
    .await?;
    execute_deployment(settings, contracts, DeploymentStage::ExternalInstantiate).await?;

    Ok(())
}

pub async fn migrate(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    interactive: bool,
    cargo_args: &[String],
) -> anyhow::Result<()> {
    build(settings, contracts, cargo_args).await?;
    store_code(settings, contracts).await?;

    execute_deployment(
        settings,
        contracts,
        DeploymentStage::Migrate { interactive },
    )
    .await?;

    Ok(())
}

pub async fn set_config(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, DeploymentStage::SetConfig).await?;
    Ok(())
}

pub async fn set_up(settings: &WorkspaceSettings, contracts: &[impl Deploy]) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, DeploymentStage::SetUp).await?;
    Ok(())
}

pub async fn custom_execute<C: Deploy>(contract: &C, string: &str) -> anyhow::Result<()> {
    println!("Executing {}", contract.name());
    let config = CONFIG.read().await;
    let value: serde_json::Value = serde_json::from_str(string)?;
    let color = to_colored_json_auto(&value)?;
    println!("{color}");
    let msg = serde_json::to_vec(&value)?;
    let key = config.get_active_key().await?;

    let chain_info = config.get_active_chain_info()?.clone();
    let client =
        HttpClient::builder(HttpClientUrl::from_str(chain_info.rpc_endpoint.as_str()).unwrap())
            .compat_mode(CompatMode::V0_34)
            .build()?; //::new(chain_info.rpc_endpoint.as_str())?.set_compat_mode(CompatMode::V0_34);
    let contract_addr = config.get_contract_addr(&contract.to_string())?.clone();
    let funds = Vec::<Coin>::parse_to_obj()?;
    let req = ExecRequest {
        msg,
        funds,
        address: Address::from_str(&contract_addr)?,
    };

    let response = client
        .wasm_execute_commit(&chain_info.cfg, req, &key, &TxOptions::default())
        .await?;

    println!(
        "gas wanted: {}, gas used: {}",
        response.deliver_tx.gas_wanted.to_string().green(),
        response.deliver_tx.gas_used.to_string().green()
    );
    println!("tx hash: {}", response.hash.to_string().purple());

    Ok(())
}

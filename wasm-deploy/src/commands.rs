use std::ffi::OsString;
use std::{env, process::Command, str::FromStr, sync::Arc};

use anyhow::Context;
use async_recursion::async_recursion;
use clap::{CommandFactory, Subcommand};
use clap_complete::{
    generate_to,
    shells::{Bash, Zsh},
};
use colored::Colorize;
use colored_json::to_colored_json_auto;
use cosm_utils::chain::coin::Denom;
use cosm_utils::modules::bank::model::SendRequest;
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
use crate::query::query;
use crate::utils::print_res;
#[cfg(wasm_cli)]
use crate::wasm_cli::wasm_cli_import_schemas;
use crate::{
    cli::{Cli, Commands},
    config::{Config, CONFIG, WORKSPACE_SETTINGS},
    contract::Deploy,
    cw20::{cw20_execute, cw20_instantiate, cw20_send},
    deployment::{execute_deployment, DeploymentStage},
    error::DeployError,
    execute::execute_contract,
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
        Commands::Update { features } => update::<C, S>(settings, features).await?,
        Commands::Init {} => init(settings).await?,
        Commands::Build { contracts } => build(settings, contracts, &cli.cargo_args).await?,
        Commands::Chain { add, delete } => chain(settings, add, delete).await?,
        Commands::Key { add, delete, show } => key(settings, add, delete, show).await?,
        Commands::Contract { add, delete } => contract(settings, add, delete).await?,
        Commands::Deploy {
            contracts,
            no_build,
            dry_run,
        } => deploy(settings, contracts, *no_build, *dry_run, &cli.cargo_args).await?,
        Commands::Env {
            add,
            delete,
            select,
            id,
        } => execute_env(settings, add, delete, select, id).await?,
        Commands::Schema { contracts } => schemas(contracts)?,
        Commands::StoreCode { contracts, dry_run } => {
            store_code(settings, contracts, *dry_run).await?
        }
        Commands::Instantiate {
            contracts,
            interactive,
            dry_run,
        } => instantiate(settings, contracts, *interactive, *dry_run).await?,
        Commands::Migrate {
            contracts,
            interactive,
            no_build,
            dry_run,
        } => {
            migrate(
                settings,
                contracts,
                *interactive,
                *no_build,
                *dry_run,
                &cli.cargo_args,
            )
            .await?
        }
        Commands::Execute { contract, dry_run } => execute_contract(contract, *dry_run).await?,
        Commands::Cw20Send { contract, dry_run } => cw20_send(contract, *dry_run).await?,
        Commands::Cw20Execute { dry_run } => cw20_execute(*dry_run).await?,
        Commands::Cw20Query { dry_run } => {
            cw20_query(*dry_run).await?;
        }
        Commands::Cw20Instantiate { dry_run } => cw20_instantiate(*dry_run).await?,
        Commands::ExecutePayload { address, payload } => custom_execute(address, payload).await?,
        Commands::QueryPayload { address, payload } => custom_query(address, payload).await?,
        Commands::SetConfig { contracts, dry_run } => {
            set_config(settings, contracts, *dry_run).await?
        }
        Commands::Query { contract, dry_run } => {
            query_contract(contract, *dry_run).await?;
        }
        Commands::SetUp { contracts, dry_run } => set_up(settings, contracts, *dry_run).await?,
        Commands::Send {
            address,
            denom,
            amount,
        } => send(address, denom, amount).await?,
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
        let chains_to_remove =
            MultiSelect::new("Select which chains to delete", chains.keys().collect()).prompt()?;
        for chain in chains_to_remove {
            config.chains.remove(chain);
        }
    }
    config.save(settings)?;
    Ok(())
}

pub async fn key(
    settings: &WorkspaceSettings,
    add: &bool,
    delete: &bool,
    show: &bool,
) -> anyhow::Result<()> {
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
    } else if *show {
        for key in &config.keys {
            let chain_info = config.get_active_chain_info()?;
            let addr = key
                .to_addr(&chain_info.cfg.prefix, &chain_info.cfg.derivation_path)
                .await?;
            println!("name: {}", key.name);
            println!("key: {:?}", key.key);
            println!("address: {}\n", addr);
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
    no_build: bool,
    dry_run: bool,
    cargo_args: &[String],
) -> anyhow::Result<()> {
    if !no_build {
        build(settings, contracts, cargo_args).await?;
    }
    store_code(settings, contracts, dry_run).await?;
    instantiate(settings, contracts, dry_run, false).await?;
    set_config(settings, contracts, dry_run).await?;
    set_up(settings, contracts, dry_run).await?;
    Ok(())
}

pub async fn update<C, S>(
    settings: &WorkspaceSettings,
    features: &Option<Vec<String>>,
) -> anyhow::Result<()>
where
    C: Deploy + Clone,
    S: Subcommand + Clone + Debug,
{
    // This must be called BEFORE install
    // so that the bin_name is valid on linux
    let bin_name = BIN_NAME.clone();
    let mut command = Command::new("cargo");
    command.arg("install");
    if let Some(features) = features {
        command
            .arg("--no-default-features")
            .arg("--features")
            .arg(features.join(","));
    }
    command
        .arg("--path")
        .arg(settings.deployment_dir.clone())
        .spawn()?
        .wait()?;

    generate_completions::<C, S>(settings, bin_name).await?;

    Ok(())
}

pub async fn generate_completions<C, S>(
    settings: &WorkspaceSettings,
    bin_name: String,
) -> anyhow::Result<()>
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
            println!(
                "{} shell completion scripts for zsh",
                "  Generating".bright_green().bold()
            );
            println!("Run source ~/.zshrc to update your completion scripts");

            let generated_file = generate_to(
                Zsh,
                &mut cmd,                    // We need to specify what generator to use
                bin_name,                    // We need to specify the bin name manually
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
                bin_name,                    // We need to specify the bin name manually
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
    // Make sure the toolchain is installed
    Command::new("rustup")
        .arg("install")
        .arg("1.69.0")
        .spawn()?
        .wait_with_output()?;

    Command::new("rustup")
        .arg("+1.69.0")
        .arg("target")
        .arg("add")
        .arg("wasm32-unknown-unknown")
        .spawn()?
        .wait_with_output()?;

    // TODO: this is a better method
    // let mut command = Command::new("cargo");
    // command
    //     .env("RUSTFLAGS", "-C link-arg=-s")
    //     .env("RUSTUP_TOOLCHAIN", "1.69.0")
    //     .arg("build")
    //     .arg("--release")
    //     .arg("--lib")
    //     .arg("--target=wasm32-unknown-unknown")
    //     .args(cargo_args);

    // // Build contracts
    // for contract in contracts {
    //     command.arg("-p");
    //     command.arg(contract.package_id());
    // }

    // command.spawn()?.wait()?;

    // Build contracts
    for contract in contracts {
        Command::new("cargo")
            .env("RUSTFLAGS", "-C link-arg=-s")
            .env("RUSTUP_TOOLCHAIN", "1.69.0")
            .arg("build")
            .arg("--release")
            .arg("--lib")
            .arg("--target=wasm32-unknown-unknown")
            .args(cargo_args)
            .arg("-p")
            .arg(contract.package_id())
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
        println!("{} {name} contract", "  Optimizing".bold().bright_green());
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
                    .spawn()
                    .context(
                        "Failed optimizing with user installed wasm-opt. \
                        Check that wasm-opt is installed and in your PATH, \
                        or compile wasm-deploy with the `wasm-opt` feature.",
                    )?,
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
    dry_run: bool,
) -> anyhow::Result<()> {
    let chunk_size = CONFIG.read().await.settings.store_code_chunk_size;
    let chunks = contracts.chunks(chunk_size);
    for chunk in chunks {
        execute_deployment(settings, chunk, dry_run, DeploymentStage::StoreCode).await?;
    }
    Ok(())
}

pub async fn instantiate(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    interactive: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    execute_deployment(
        settings,
        contracts,
        dry_run,
        DeploymentStage::Instantiate { interactive },
    )
    .await?;
    execute_deployment(
        settings,
        contracts,
        dry_run,
        DeploymentStage::ExternalInstantiate,
    )
    .await?;

    Ok(())
}

pub async fn migrate(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    interactive: bool,
    no_build: bool,
    dry_run: bool,
    cargo_args: &[String],
) -> anyhow::Result<()> {
    if !no_build {
        build(settings, contracts, cargo_args).await?;
        store_code(settings, contracts, dry_run).await?;
    }

    execute_deployment(
        settings,
        contracts,
        dry_run,
        DeploymentStage::Migrate { interactive },
    )
    .await?;

    Ok(())
}

pub async fn set_config(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    dry_run: bool,
) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, dry_run, DeploymentStage::SetConfig).await?;
    Ok(())
}

pub async fn set_up(
    settings: &WorkspaceSettings,
    contracts: &[impl Deploy],
    dry_run: bool,
) -> anyhow::Result<()> {
    execute_deployment(settings, contracts, dry_run, DeploymentStage::SetUp).await?;
    Ok(())
}

pub async fn send(address: &str, denom: &Denom, amount: &u128) -> anyhow::Result<()> {
    let config = CONFIG.read().await;
    let key = config.get_active_key().await?;
    let chain_info = config.get_active_chain_info()?.clone();
    let from = key
        .to_addr(&chain_info.cfg.prefix, &chain_info.cfg.derivation_path)
        .await?;
    let client =
        HttpClient::builder(HttpClientUrl::from_str(chain_info.rpc_endpoint.as_str()).unwrap())
            .compat_mode(CompatMode::V0_34)
            .build()?;
    let to = Address::from_str(address)?;
    let coin = Coin {
        denom: denom.clone(),
        amount: *amount,
    };

    let req = SendRequest {
        from,
        to,
        amounts: vec![coin],
    };

    let res = client
        .bank_send_commit(&chain_info.cfg, req, &key, &TxOptions::default())
        .await?;

    print_res(res);

    Ok(())
}

pub async fn custom_execute(address: &str, payload: &str) -> anyhow::Result<()> {
    println!("Executing {}", address);
    let config = CONFIG.read().await;
    let msg: serde_json::Value = serde_json::from_str(payload)?;
    let color = to_colored_json_auto(&msg)?;
    println!("{color}");
    let key = config.get_active_key().await?;

    let chain_info = config.get_active_chain_info()?.clone();
    let client =
        HttpClient::builder(HttpClientUrl::from_str(chain_info.rpc_endpoint.as_str()).unwrap())
            .compat_mode(CompatMode::V0_34)
            .build()?; //::new(chain_info.rpc_endpoint.as_str())?.set_compat_mode(CompatMode::V0_34);
    let funds = Vec::<Coin>::parse_to_obj()?;
    let req = ExecRequest {
        msg,
        funds,
        address: Address::from_str(address)?,
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

pub async fn custom_query(address: &str, payload: &str) -> anyhow::Result<()> {
    println!("Querying {}", address);
    let config = CONFIG.read().await;
    let value: serde_json::Value = serde_json::from_str(payload)?;
    let color_value = to_colored_json_auto(&value)?;
    println!("{color_value}");
    let res = query(&config, address.to_string(), value).await?;
    let color_res = to_colored_json_auto(&res)?;
    println!("{color_res}");

    Ok(())
}

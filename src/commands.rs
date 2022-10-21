use std::env;
use std::{error::Error};
use std::process::{Command};
use clap::{Subcommand, CommandFactory};
use clap_complete::generate_to;
use clap_complete::shells::{Bash, Zsh};
use clap_interactive::InteractiveParse;
use crate::cli::{Cli, Commands};
use crate::contract::{ Contract, Execute, Query};
use crate::file::{BUILD_DIR, get_shell_completion_dir};
use crate::wasm_cli::{wasm_cli_store_code, wasm_cli_import_schemas};

#[derive(PartialEq)]
pub enum Status {
    Continue,
    Quit
}

pub fn execute_args<C, E, Q>(cli: &Cli<C, E, Q>) -> Result<Status, Box<dyn Error>> 
where C: Contract,
      E: Execute + Subcommand,
      Q: Query + Subcommand
{
    match &cli.command {
        Commands::Update {  } => update::<C, E, Q>(),
        Commands::Build { contracts } => build(contracts),
        Commands::Deploy { contracts, no_build } => deploy(contracts, no_build),
        Commands::Schema { contracts } => schemas(contracts),
        Commands::StoreCode { contracts } => store_code(contracts),
        Commands::Instantiate { contracts } => instantiate(contracts),
        Commands::Migrate { contracts } => migrate(contracts),
        Commands::Execute { execute_command } => execute(execute_command),
        Commands::SetConfig { contracts } => set_config(contracts),
        Commands::Query { contract } => query(contract),
        Commands::Interactive {  } => interactive::<C, E, Q>(),
        Commands::SetUp {  } => set_up(),
    }
}

pub fn deploy(contracts: &Vec<impl Contract>, no_build: &bool) -> Result<Status, Box<dyn Error>> {
    if !no_build { build(contracts)?; }
    store_code(contracts)?;
    instantiate(contracts)?;
    set_config(contracts)?;
    Ok(Status::Continue)
}

pub fn update<C, E, Q>() -> Result<Status, Box<dyn Error>> 
where C: Contract,
      E: Subcommand + Execute,
      Q: Subcommand + Query   
{

    Command::new("mv")
        .arg("./target/release/deploy")
        .arg("./target/release/deploy.old")
        .spawn()?
        .wait()?;

    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir("./deployment")
        .spawn()?
        .wait()?
        .exit_ok()?;

    generate_completions::<C, E, Q>()?; 

    Ok(Status::Quit)
}

fn generate_completions<C, E, Q>() -> Result<(), Box<dyn Error>> 
where C: Contract,
      E: Subcommand + Execute,
      Q: Subcommand + Query   
{

    let shell_completion_dir = match get_shell_completion_dir()? {
        Some(shell_completion_dir) => shell_completion_dir,
        None => return Ok(()),
    };
    let string = env::var_os("SHELL").unwrap().into_string().unwrap();
    let (_, last_word) = string.rsplit_once('/').unwrap();
    let mut cmd = Cli::<C, E, Q>::command();

    match last_word {
        "zsh" => {

            println!("Generating shell completion scripts for zsh");
            println!("Run source ~/.zshrc to update your completion scripts");

            let generated_file = generate_to(
                Zsh,
                &mut cmd,  // We need to specify what generator to use
                "deploy",  // We need to specify the bin name manually
                BUILD_DIR.as_path(),    // We need to specify where to write to
            )?;

            let source_path = BUILD_DIR.join(generated_file.file_name().unwrap());
            let target_path = shell_completion_dir.join(generated_file.file_name().unwrap());

            if Command::new("cp")
                .arg(source_path)
                .arg(target_path)
                .spawn()?
                .wait()?
                .exit_ok().is_err() {
                    println!("could not find {}", shell_completion_dir.to_str().unwrap());
                }

        },
        "bash" => {
            println!("generating shell completion scripts for bash");
            let generated_file = generate_to(
                Bash,
                &mut cmd,  // We need to specify what generator to use
                "deploy",  // We need to specify the bin name manually
                BUILD_DIR.as_path(),    // We need to specify where to write to
            )?;

            let source_path = BUILD_DIR.join(generated_file.file_name().unwrap());
            let target_path = shell_completion_dir.join(generated_file.file_name().unwrap());
            
            if Command::new("cp")
            .arg(source_path)
            .arg(target_path)
            .spawn()?
            .wait()?
            .exit_ok().is_err() {
                println!("could not find {}", shell_completion_dir.to_str().unwrap());
            }
        },
        _ => {
            return Err("Unsupported shell".into());
        }
    }    

    Ok(())
}

pub fn build(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    // Build contracts
    for contract in contracts {
        Command::new("cargo")
            .env("RUSTFLAGS", "-C link-arg=-s")
            .arg("build")
            .arg("--release")
            .arg("--target=wasm32-unknown-unknown")
            .current_dir(format!("./contracts/{}", contract.name()))
            .spawn()?
            .wait()?
            .exit_ok()?;
    }
    
    Command::new("mkdir")
        .arg("-p")
        .arg("artifacts")
        .spawn()?
        .wait()?;

    optimize(contracts)?;
    set_execute_permissions(contracts)?;

    Ok(Status::Quit)
}

pub fn schemas(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    // Generate schemas
    for contract in contracts {
        Command::new("cargo")
            .arg("schema")
            .current_dir(format!("./contracts/{}", contract.name()))
            .spawn()?
            .wait()?
            .exit_ok()?;
    }

    // Import schemas
    for contract in contracts {
        wasm_cli_import_schemas(&contract.name())?;
    }

    Ok(Status::Quit)
}

pub fn optimize(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
        // Optimize contracts
        let mut handles = vec![];
        for contract in contracts {
            let name = contract.name();
            println!("Optimizing {} contract", name);
            handles.push(Command::new("wasm-opt")
                .arg("-Os")
                .arg("-o")
                .arg(format!("artifacts/{}.wasm", name))
                .arg(format!("target/wasm32-unknown-unknown/release/{}.wasm", name))
                .spawn()?
            );
        }
        handles.iter_mut().for_each(|x| {x.wait().unwrap();} );
        Ok(Status::Quit)
}

pub fn set_execute_permissions(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    // change mod
    for contract in contracts {
        let name = contract.name();
        Command::new("chmod")
            .arg("+x")
            .arg(format!("artifacts/{}.wasm", name));
    }
    Ok(Status::Quit)
}

pub fn store_code(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    for contract in contracts {
        let name = contract.name();
        wasm_cli_store_code(&name)?
    }
    Ok(Status::Quit)
}

pub fn instantiate(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    for contract in contracts {
        crate::contract::instantiate(contract)?;
        // TODO: figure out how to make this happen
        // if contract.name() == "market".to_string() {
        //     for (_, _, instantiate_msg) in INSTANTIATE_RECEIPTS.as_ref() {
        //         let payload = serde_json::to_string(instantiate_msg)?;
        //         wasm_cli_instantiate_with_code_id(&instantiate_msg.name, RECEIPT_CODE_ID, &payload)?;
        //     }
        // }

    }
    Ok(Status::Quit)
}

pub fn migrate(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    build(contracts)?;
    store_code(contracts)?;
    for contract in contracts {
        crate::contract::migrate(contract)?;
    }
    Ok(Status::Quit)
}

pub fn set_config(contracts: &Vec<impl Contract>) -> Result<Status, Box<dyn Error>> {
    for contract in contracts {
        crate::contract::set_config(contract)?;
    }
    Ok(Status::Quit)
}

pub fn execute<E: Execute>(contract: &E) -> Result<Status, Box<dyn Error>> {
    crate::contract::execute(contract)?;
    Ok(Status::Quit)
}

pub fn set_up() -> Result<Status, Box<dyn Error>> {

    // for asset in PRICE_ORACLE_ASSETS.as_ref() {
    //     PriceOracle{}.add_asset(asset)?;
    // }

    // for asset in ADD_INTEREST_MODEL_ASSETS.as_ref() {
    //     wasm_cli_execute(&"interest_model".to_string(), &serde_json::to_string(asset)?)?;
    // }

    // for asset in MARKET_ASSETS.as_ref() {
    //     wasm_cli_execute(&"market".to_string(), &serde_json::to_string(asset)?)?;
    // }

    // for asset in COLLATERAL_ASSETS.as_ref() {
    //     wasm_cli_execute(&"market".to_string(), &serde_json::to_string(asset)?)?;
    // }

    // for pool in ADD_LIQUIDITY_POOLS.as_ref() {
    //     wasm_cli_execute(&"market".to_string(), &serde_json::to_string(pool)?)?;
    // }

    Ok(Status::Quit)
}

pub fn query<Q: Query>(contract: &Q) -> Result<Status, Box<dyn Error>> {
    crate::contract::query(contract)?;
    Ok(Status::Quit)
}

pub fn quit() -> Result<Status, Box<dyn Error>> {
    Ok(Status::Quit)
}

pub fn interactive<C, E, Q>() -> Result<Status, Box<dyn Error>> 
where C: Contract,
      E: Subcommand + Execute,
      Q: Subcommand + Query   
{
    let cli = Cli::<C, E, Q>::interactive_parse()?;
    Ok(execute_args(&cli)?)
}

pub fn test() -> Result<Status, Box<dyn Error>> {
    todo!()
}
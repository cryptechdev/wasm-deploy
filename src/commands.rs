use std::env;
use std::process::{Command};
use clap::{CommandFactory};
use clap_complete::generate_to;
use clap_complete::shells::{Bash, Zsh};
use clap_interactive::InteractiveParse;
use crate::cli::{Cli, Commands};
use crate::contract::{ Contract, Execute, Query, execute_set_up};
use crate::error::DeployError;
use crate::file::{BUILD_DIR, get_shell_completion_dir};
use crate::wasm_cli::{wasm_cli_store_code, wasm_cli_import_schemas};

#[derive(PartialEq)]
pub enum Status {
    Continue,
    Quit
}

pub fn execute_args<C, E, Q>(cli: &Cli<C, E, Q>) -> Result<Status, DeployError> 
where C: Contract,
      E: Execute,
      Q: Query
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
        Commands::SetUp { contracts } => set_up(contracts),
        Commands::Interactive {  } => interactive::<C, E, Q>(),
    }
}

pub fn deploy(contracts: &Vec<impl Contract>, no_build: &bool) -> Result<Status, DeployError> {
    if !no_build { build(contracts)?; }
    store_code(contracts)?;
    instantiate(contracts)?;
    set_config(contracts)?;
    set_up(contracts)?;
    Ok(Status::Continue)
}

pub fn update<C, E, Q>() -> Result<Status, DeployError> 
where C: Contract,
      E: Execute,
      Q: Query   
{

    Command::new("mv")
        .arg("./target/debug/deploy")
        .arg("./target/debug/deploy.old")
        .spawn()?
        .wait()?;

    Command::new("cargo")
        .arg("build")
        .current_dir("./deployment")
        .spawn()?
        .wait()?
        .exit_ok()?;

    generate_completions::<C, E, Q>()?; 

    Ok(Status::Quit)
}

fn generate_completions<C, E, Q>() -> Result<(), DeployError> 
where C: Contract,
      E: Execute,
      Q: Query   
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
            return Err(DeployError::UnsupportedShell{});
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
            .arg("--release")
            .arg("--lib")
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

pub fn set_execute_permissions(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    // change mod
    for contract in contracts {
        let name = contract.name();
        Command::new("chmod")
            .arg("+x")
            .arg(format!("artifacts/{}.wasm", name));
    }
    Ok(Status::Quit)
}

pub fn store_code(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        let name = contract.name();
        wasm_cli_store_code(&name)?
    }
    Ok(Status::Quit)
}

pub fn instantiate(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        crate::contract::execute_instantiate(contract)?;
    }
    Ok(Status::Quit)
}

pub fn migrate(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    build(contracts)?;
    store_code(contracts)?;
    for contract in contracts {
        crate::contract::execute_migrate(contract)?;
    }
    Ok(Status::Quit)
}

pub fn set_config(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    for contract in contracts {
        crate::contract::execute_set_config(contract)?;
    }
    Ok(Status::Quit)
}

pub fn set_up(contracts: &Vec<impl Contract>) -> Result<Status, DeployError> {
    contracts.iter().try_for_each(|x| execute_set_up(x))?;
    Ok(Status::Quit)
}

pub fn execute<E: Execute>(execute: &Option<E>) -> Result<Status, DeployError> {
    match execute {
        Some(e) => {
            crate::contract::execute(e)?;
        },
        None => {
            let e = &E::interactive_parse()?;
            crate::contract::execute(e)?;
        },
    }
    Ok(Status::Quit)
}

pub fn query<Q: Query>(query: &Option<Q>) -> Result<Status, DeployError> {
    match query {
        Some(q) => {
            crate::contract::query(q)?;
        },
        None => {
            let q = &Q::interactive_parse()?;
            crate::contract::query(q)?;
        },
    }
    Ok(Status::Quit)
}

pub fn interactive<C, E, Q>() -> Result<Status, DeployError> 
where C: Contract,
      E: Execute,
      Q: Query   
{
    let cli = Cli::<C, E, Q>::interactive_parse()?;
    Ok(execute_args(&cli)?)
}
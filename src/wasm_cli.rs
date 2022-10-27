use std::{process::Command};

use crate::error::DeployError;

pub fn wasm_cli_execute(contract_name: &String, payload: &String) -> Result<(), DeployError>{
    println!("executing {} contract", contract_name);
    Command::new("wasm-cli")
        .arg("tx")
        .arg("-a")
        .arg(format!("&{}", contract_name))
        .arg("-p")
        .arg(payload.as_str())
        .spawn()?
        .wait()?;
        Ok(())
}

pub fn wasm_cli_execute_silent(contract_name: &String, payload: &String) -> Result<(), DeployError>{
    println!("executing {} contract", contract_name);
    Command::new("wasm-cli")
        .arg("tx")
        .arg("-s")
        .arg("-a")
        .arg(format!("&{}", contract_name))
        .arg("-p")
        .arg(payload.as_str())
        .spawn()?
        .wait()?;
        Ok(())
}

pub fn wasm_cli_migrate(contract_name: &String, payload: &String) -> Result<(), DeployError>{
    Command::new("wasm-cli")
        .arg("migrate")
        .arg("-s")
        .arg("--name")
        .arg(contract_name)
        .arg(payload.as_str())
        .spawn()?
        .wait()?;
        Ok(())
}

pub fn wasm_cli_instantiate(admin: &String, contract_name: &String, payload: &String) -> Result<(), DeployError>{
    println!("Instantiating {} contract", contract_name);
    Command::new("wasm-cli")
        .arg("instantiate")
        .arg("-s")
        .arg("-a")
        .arg(admin)
        .arg("-n")
        .arg(contract_name)
        .arg("-p")
        .arg(payload)
        .spawn()?
        .wait()?;
        Ok(())
}

pub fn wasm_cli_instantiate_with_code_id(admin: &String, contract_name: &String, code_id: u64, payload: &String) -> Result<(), DeployError>{
    println!("Instantiating {} contract", contract_name);
    Command::new("wasm-cli")
        .arg("instantiate")
        .arg("-s")
        .arg("-a")
        .arg(admin)
        .arg("-n")
        .arg(contract_name)
        .arg("-c")
        .arg(code_id.to_string())
        .arg("-p")
        .arg(payload)
        .spawn()?
        .wait()?;
        Ok(())
}

pub fn wasm_cli_store_code(name: &String) -> Result<(), DeployError>{
    println!("Storing code for {} contract", name);
    Command::new("wasm-cli")
        .arg("store")
        .arg("-s")
        .arg("--name")
        .arg(format!("{}", name))
        .arg(format!("artifacts/{}.wasm", name))
        .spawn()?
        .wait()?;
    Ok(())
}

pub fn wasm_cli_query(contract_name: &String, payload: &String) -> Result<(), DeployError>{
    println!("Querying {} contract", contract_name);
    Command::new("wasm-cli")
        .arg("query")
        .arg("-s")
        .arg("-a")
        .arg(format!("&{}", contract_name))
        .arg("-p")
        .arg(payload.as_str())
        .spawn()?
        .wait()?;
        Ok(())
}

pub fn wasm_cli_import_schemas(name: &String) -> Result<(), DeployError> {
    println!("Importing schemas for {} contract", name);
    Command::new("wasm-cli")
    .arg("import")
    .arg("-s")
    .arg("--name")
    .arg(&name)
    .arg(format!("contracts/{}/schema", &name))
    .spawn()?
    .wait()?
    .exit_ok()?;
    Ok(())
}

pub fn wasm_cli_import_receipt_schemas(name: &String) -> Result<(), DeployError> {
    println!("Importing schemas for {} contract", name);
    Command::new("wasm-cli")
        .arg("import")
        .arg("-s")
        .arg("--name")
        .arg(name)
        .arg("contracts/receipt/schema")
        .spawn()?
        .wait()?
        .exit_ok()?;
    Ok(())
}
use std::process::Command;

use crate::error::DeployError;

pub fn wasm_cli_import_schemas(name: &String) -> Result<(), DeployError> {
    println!("Importing schemas for {} contract", name);
    Command::new("wasm-cli")
        .arg("import")
        .arg("-s")
        .arg("--name")
        .arg(name)
        .arg(format!("contracts/{}/schema", &name))
        .spawn()?
        .wait()?
        .exit_ok()?;
    Ok(())
}

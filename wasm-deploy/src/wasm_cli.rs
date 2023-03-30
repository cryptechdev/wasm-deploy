use std::process::Command;

use crate::error::DeployError;

pub fn wasm_cli_import_schemas(name: &String) -> anyhow::Result<()> {
    println!("Importing schemas for {} contract", name);
    let mut schema_path = PROJECT_ROOT.clone();
    schema_path.push(format!("contracts/{}/schema", &name));
    Command::new("wasm-cli")
        .arg("import")
        .arg("-s")
        .arg("--name")
        .arg(name)
        .arg(schema_path)
        .spawn()?
        .wait()?
        .exit_ok()?;
    Ok(())
}

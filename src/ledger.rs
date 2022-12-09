use async_recursion::async_recursion;
use inquire::Select;
use ledger_cosmos_secp256k1::*;
use ledger_utility::{Connection, Device};
use serde::{Deserialize, Serialize};

use crate::{error::DeployError, file::ChainInfo, key::path_to_array};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LedgerInfo {
    pub address:     String,
    pub device_name: String,
}

#[async_recursion(?Send)]
pub async fn select_ledger(connection: &Connection) -> Result<Device, DeployError> {
    let mut ledgers = connection.get_all_ledgers().await?;
    let mut options = Vec::new();
    for ledger in &ledgers {
        options.push(ledger.name().await?);
    }
    options.push("Refresh".to_string());

    match Select::new("Select Device", options.iter().collect::<Vec<_>>()).prompt()?.as_str() {
        "Refresh" => select_ledger(connection).await,
        name => {
            let index = options.iter().position(|x| x.as_str() == name).unwrap();
            Ok(ledgers.swap_remove(index))
        }
    }
}

// #[async_recursion(?Send)]
pub async fn get_ledger_info(connection: &Connection, chain_info: ChainInfo) -> Result<LedgerInfo, DeployError> {
    let device = select_ledger(connection).await?;
    let device_name = device.name().await?;
    let ledger = connection.connect(device).await?;
    let app = CosmosApp::new(ledger);
    let path = path_to_array(&chain_info.derivation_path)?;
    let display_on_ledger = false;
    println!("Requesting public key from ledger...");
    let secp256k1_res = app.get_addr_secp256k1(path, &chain_info.prefix, display_on_ledger).await.unwrap();

    Ok(LedgerInfo { address: secp256k1_res.addr, device_name })
}

#[cfg(test)]
mod test {

    use super::*;

    #[tokio::test]
    async fn test_select_ledger() {
        let connection = Connection::new().await;
        let ledger = select_ledger(&connection).await.unwrap();
        println!("Selected: {}", ledger.name().await.unwrap());
    }
}

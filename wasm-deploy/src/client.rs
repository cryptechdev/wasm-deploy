use std::str::FromStr;

use lazy_static::lazy_static;
use log::info;
use tendermint_rpc::{client::CompatMode, HttpClient};
use tendermint_rpc::{Client, HttpClientUrl};
use tokio::sync::RwLock;

lazy_static! {
    static ref COMPAT_MODE: RwLock<Option<CompatMode>> = RwLock::new(None);
}

async fn set_compat_mode(compat_mode: CompatMode) {
    *COMPAT_MODE.write().await = Some(compat_mode);
}

async fn get_compat_mode(rpc_endpoint: &str) -> CompatMode {
    let maybe_compat_mode = *COMPAT_MODE.read().await;
    match maybe_compat_mode {
        Some(compat_mode) => compat_mode,
        None => {
            let client = HttpClient::new(rpc_endpoint).expect("invalid rpc endpoint");
            let version = client.status().await.unwrap().node_info.version;
            info!("using tendermint version: {}", version);
            let compat_mode = CompatMode::from_version(version).unwrap();
            set_compat_mode(compat_mode).await;
            compat_mode
        }
    }
}
pub async fn get_client(rpc_endpoint: &str) -> anyhow::Result<HttpClient> {
    let compat_mode = get_compat_mode(rpc_endpoint).await;
    let client =
        HttpClient::builder(HttpClientUrl::from_str(rpc_endpoint).expect("invalid rpc endpoint"))
            .compat_mode(compat_mode)
            .build()?;

    Ok(client)
}

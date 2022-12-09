use std::str::FromStr;

use cosmos_sdk_proto::{
    cosmos::{
        auth::v1beta1::{BaseAccount, QueryAccountRequest, QueryAccountResponse},
        tx::v1beta1::{service_client::ServiceClient, SimulateRequest},
    },
    traits::Message,
};
use cosmrs::{
    rpc::{
        endpoint::{abci_query::AbciQuery, broadcast::tx_commit::Response},
        Client, HttpClient,
    },
    tendermint::abci::{Code, Event},
    tx::{self, Fee, Msg, Raw, SignerInfo},
    AccountId, Coin, Denom,
};
use serde::Serialize;

use crate::{chain_res::ChainResponse, error::DeployError, file::ChainInfo, key::UserKey};

pub async fn send_tx(
    client: &HttpClient, msg: impl Msg + Serialize, key: &UserKey, account_id: AccountId, cfg: &ChainInfo,
) -> Result<Response, DeployError> {
    let account = account(client, account_id).await?;
    let memo: String = "wasm-deply".into();

    let fee = simulate_gas_fee(key, &account, cfg, memo.clone(), vec![msg.clone()]).await?;

    println!("fee: {:?}", fee);

    let tx_raw = key.sign(&account, fee.into(), cfg, memo, vec![msg]).await?;

    let tx_commit_response = tx_raw.broadcast_commit(client).await?;

    if tx_commit_response.check_tx.code.is_err() {
        return Err(DeployError::CosmosSdk { res: tx_commit_response.check_tx.into() });
    }
    if tx_commit_response.deliver_tx.code.is_err() {
        return Err(DeployError::CosmosSdk { res: tx_commit_response.deliver_tx.into() });
    }

    Ok(tx_commit_response)
}

pub async fn abci_query<T: Message>(client: &HttpClient, req: T, path: &str) -> Result<AbciQuery, DeployError> {
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf)?;

    let res = client.abci_query(Some(path.parse().unwrap()), buf, None, false).await?;

    if res.code != Code::Ok {
        return Err(DeployError::CosmosSdk { res: res.into() });
    }

    Ok(res)
}

async fn account(client: &HttpClient, account_id: AccountId) -> Result<BaseAccount, DeployError> {
    let res = abci_query(
        client,
        QueryAccountRequest { address: account_id.as_ref().into() },
        "/cosmos.auth.v1beta1.Query/Account",
    )
    .await?;

    let res = QueryAccountResponse::decode(res.value.as_slice())?
        .account
        .ok_or(DeployError::AccountId { id: account_id.to_string() })?;

    let base_account = BaseAccount::decode(res.value.as_slice())?;

    Ok(base_account)
}

#[allow(deprecated)]
async fn simulate_gas_fee(
    user_key: &UserKey, account: &BaseAccount, chain_info: &ChainInfo, memo: String, msgs: Vec<impl Msg + Serialize>,
) -> Result<Fee, DeployError> {
    let timeout_height = 0u16;
    let anys = msgs.iter().map(|msg| msg.to_any()).collect::<Result<Vec<_>, _>>().unwrap();
    let tx_body = tx::Body::new(anys, memo.clone(), timeout_height);
    let public_key = user_key.public_key(&chain_info.derivation_path).await?.into();
    let fee = Fee::from_amount_and_gas(
        Coin { denom: Denom::from_str(&chain_info.denom).unwrap(), amount: 0u64.into() },
        0u64,
    );
    let auth_info = SignerInfo::single_direct(Some(public_key), account.sequence).auth_info(fee.clone());

    let raw: Raw = cosmos_sdk_proto::cosmos::tx::v1beta1::TxRaw {
        body_bytes:      tx_body.into_bytes().unwrap(),
        auth_info_bytes: auth_info.into_bytes().unwrap(),
        signatures:      vec![vec![0u8; 33]],
    }
    .into();

    let mut client = ServiceClient::connect(chain_info.grpc_endpoint.clone()).await.unwrap();

    let gas_info = client
        .simulate(SimulateRequest { tx: None, tx_bytes: raw.to_bytes()? })
        .await
        .map_err(|e| DeployError::CosmosSdk {
            res: ChainResponse {
                code: Code::Err((e.code() as u32).try_into().unwrap()),
                log: e.message().to_string(),
                ..Default::default()
            },
        })?
        .into_inner()
        .gas_info
        .unwrap();

    let gas_limit = (gas_info.gas_used as f64 * chain_info.gas_adjustment).ceil();
    let amount = Coin {
        denom:  Denom::from_str(&chain_info.denom).unwrap(),
        amount: ((gas_limit * chain_info.gas_price).ceil() as u64).into(),
    };

    Ok(Fee::from_amount_and_gas(amount, gas_limit as u64))
}

pub fn find_event(res: &Response, key_name: &str) -> Option<Event> {
    for event in &res.deliver_tx.events {
        if event.kind == key_name {
            return Some(event.clone());
        }
    }
    None
}

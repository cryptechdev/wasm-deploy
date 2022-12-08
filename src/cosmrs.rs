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
    tx::{
        Fee, Msg, SignDoc, SignerInfo, {self},
    },
    AccountId, Any, Coin, Denom,
};

use crate::{error::DeployError, file::ChainInfo, key::UserKey};

pub async fn send_tx(
    client: &HttpClient, msg: impl Msg, key: &UserKey, account_id: AccountId, cfg: &ChainInfo,
) -> Result<Response, DeployError> {
    //let signing_key: secp256k1::SigningKey = key.try_into().unwrap();

    let account = account(client, account_id).await?;

    let fee = simulate_gas_fee(&tx_body, &account, key, cfg).await?;

    let tx_raw = key
        .sign(
            &cfg.derivation_path,
            fee.into(),
            cfg.chain_id,
            "Sent With Wasm-Deploy".into(),
            account.account_number,
            account.sequence,
            vec![msg],
        )
        .await?;

    let tx_commit_response = tx_raw.broadcast_commit(client).await.map_err(DeployError::proto_encoding)?;

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
    req.encode(&mut buf).map_err(DeployError::prost_proto_en)?;

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

    let res = QueryAccountResponse::decode(res.value.as_slice())
        .map_err(DeployError::prost_proto_de)?
        .account
        .ok_or(DeployError::AccountId { id: account_id.to_string() })?;

    let base_account = BaseAccount::decode(res.value.as_slice()).map_err(DeployError::prost_proto_de)?;

    Ok(base_account)
}

#[allow(deprecated)]
async fn simulate_gas_fee(
    tx: &tx::Body, account: &BaseAccount, user_key: &UserKey, cfg: &ChainInfo,
) -> Result<Fee, DeployError> {
    // TODO: support passing in the exact fee too (should be on a per process_msg() call)
    let denom: Denom = cfg.denom.parse().map_err(|_| DeployError::Denom { name: cfg.denom.clone() })?;

    let signer_info =
        SignerInfo::single_direct(Some(user_key.public_key(&cfg.derivation_path).await?.into()), account.sequence);
    let auth_info =
        signer_info.auth_info(Fee::from_amount_and_gas(Coin { denom: denom.clone(), amount: 0u64.into() }, 0u64));

    let sign_doc = SignDoc::new(tx, &auth_info, &cfg.chain_id.clone().try_into().unwrap(), account.account_number)
        .map_err(DeployError::proto_encoding)?;

    let tx_raw = user_key.sign(&cfg.derivation_path, sign_doc).await?;

    let mut client = ServiceClient::connect(cfg.grpc_endpoint.clone()).await?;

    let gas_info = client
        .simulate(SimulateRequest { tx: None, tx_bytes: tx_raw.to_bytes().map_err(DeployError::proto_encoding)? })
        .await
        .map_err(|e| DeployError::CosmosSdk {
            res: ChainResponse { code: Code::Err(e.code() as u32), log: e.message().to_string(), ..Default::default() },
        })?
        .into_inner()
        .gas_info
        .unwrap();

    let gas_limit = (gas_info.gas_used as f64 * cfg.gas_adjustment).ceil();
    let amount = Coin { denom: denom.clone(), amount: ((gas_limit * cfg.gas_price).ceil() as u64).into() };

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

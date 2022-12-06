use cosm_orc::client::{chain_res::ChainResponse, error::ClientError};
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
        Fee, SignDoc, SignerInfo, {self},
    },
    AccountId, Any, Coin, Denom,
};

use crate::{file::ChainInfo, key::UserKey};

// const MEMO: &str = "hello";
// const TIMEOUT_HEIGHT: u16 = 10_000u16;
// const GAS: u64 = 100_000u64;

// my attempt
// pub async fn send_msgs<I: IntoIterator<Item = Any>>(messages: I, config: &mut Config) ->
// Result<(), DeployError>{

//     let private_key = config.get_private_key()?;
//     let public_key = private_key.public_key();

//     // Create transaction body from the MsgSend, memo, and timeout height
//     let tx_body = tx::Body::new(messages, MEMO, TIMEOUT_HEIGHT);

//     // Create signer info from public key and sequence number.
//     // This uses a standard "direct" signature from a single signer.
//     let signer_info = SignerInfo::single_direct(Some(public_key), 0);

//     let amount = Coin {
//         amount: 0u128,
//         denom: "ucrd".parse().unwrap(),
//     };

//     // Compute auth info from signer info by associating a fee.
//     let auth_info = signer_info.auth_info(Fee::from_amount_and_gas(amount, GAS));

//     //////////////////////////
//     // Signing transactions //
//     //////////////////////////

//     // The "sign doc" contains a message to be signed.
//     let sign_doc = SignDoc::new(&tx_body, &auth_info, &config.get_active_chain_id()?,
// 1).unwrap();

//     // Sign the "sign doc" with the sender's private key, producing a signed raw transaction.
//     let tx_signed = sign_doc.sign(&private_key).unwrap();

//     // Send it!
//     let response = tx_signed.broadcast_commit(&config.get_client()?).await;

//     println!("response: {:?}", response);

//     // Serialize the raw transaction as bytes (i.e. `Vec<u8>`).
//     //let tx_bytes = tx_signed.to_bytes().unwrap();

//     //////////////////////////
//     // Parsing transactions //
//     //////////////////////////

//     // Parse the serialized bytes from above into a `cosmrs::Tx`
//     //let tx_parsed = Tx::from_bytes(&tx_bytes).unwrap();

//     Ok(())
// }

pub async fn send_tx(
    client: &HttpClient, msg: Any, key: &UserKey, account_id: AccountId, cfg: &ChainInfo,
) -> Result<Response, ClientError> {
    //let signing_key: secp256k1::SigningKey = key.try_into().unwrap();
    let public_key = key.public_key(&cfg.derivation_path).await?.into();
    let timeout_height = 0u16; // TODO
    let account = account(client, account_id).await?;

    let tx_body = tx::Body::new(vec![msg], "MEMO", timeout_height);

    let fee = simulate_gas_fee(&tx_body, &account, key, cfg).await?;

    // NOTE: if we are making requests in parallel with the same key, we need to serialize
    // `account.sequence` to avoid errors
    let auth_info = SignerInfo::single_direct(Some(public_key), account.sequence).auth_info(fee);

    let sign_doc =
        SignDoc::new(&tx_body, &auth_info, &cfg.chain_id.clone().try_into().unwrap(), account.account_number)
            .map_err(ClientError::proto_encoding)?;

    let tx_raw = key.sign(&cfg.derivation_path, sign_doc).await?;

    let tx_commit_response = tx_raw.broadcast_commit(client).await.map_err(ClientError::proto_encoding)?;

    if tx_commit_response.check_tx.code.is_err() {
        return Err(ClientError::CosmosSdk { res: tx_commit_response.check_tx.into() });
    }
    if tx_commit_response.deliver_tx.code.is_err() {
        return Err(ClientError::CosmosSdk { res: tx_commit_response.deliver_tx.into() });
    }

    Ok(tx_commit_response)
}

pub async fn abci_query<T: Message>(client: &HttpClient, req: T, path: &str) -> Result<AbciQuery, ClientError> {
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf).map_err(ClientError::prost_proto_en)?;

    let res = client.abci_query(Some(path.parse().unwrap()), buf, None, false).await?;

    if res.code != Code::Ok {
        return Err(ClientError::CosmosSdk { res: res.into() });
    }

    Ok(res)
}

async fn account(client: &HttpClient, account_id: AccountId) -> Result<BaseAccount, ClientError> {
    let res = abci_query(
        client,
        QueryAccountRequest { address: account_id.as_ref().into() },
        "/cosmos.auth.v1beta1.Query/Account",
    )
    .await?;

    let res = QueryAccountResponse::decode(res.value.as_slice())
        .map_err(ClientError::prost_proto_de)?
        .account
        .ok_or(ClientError::AccountId { id: account_id.to_string() })?;

    let base_account = BaseAccount::decode(res.value.as_slice()).map_err(ClientError::prost_proto_de)?;

    Ok(base_account)
}

#[allow(deprecated)]
async fn simulate_gas_fee(
    tx: &tx::Body, account: &BaseAccount, user_key: &UserKey, cfg: &ChainInfo,
) -> Result<Fee, ClientError> {
    // TODO: support passing in the exact fee too (should be on a per process_msg() call)
    let denom: Denom = cfg.denom.parse().map_err(|_| ClientError::Denom { name: cfg.denom.clone() })?;

    let signer_info =
        SignerInfo::single_direct(Some(user_key.public_key(&cfg.derivation_path).await?.into()), account.sequence);
    let auth_info =
        signer_info.auth_info(Fee::from_amount_and_gas(Coin { denom: denom.clone(), amount: 0u64.into() }, 0u64));

    let sign_doc = SignDoc::new(tx, &auth_info, &cfg.chain_id.clone().try_into().unwrap(), account.account_number)
        .map_err(ClientError::proto_encoding)?;

    let tx_raw = user_key.sign(&cfg.derivation_path, sign_doc).await?;

    let mut client = ServiceClient::connect(cfg.grpc_endpoint.clone()).await?;

    let gas_info = client
        .simulate(SimulateRequest { tx: None, tx_bytes: tx_raw.to_bytes().map_err(ClientError::proto_encoding)? })
        .await
        .map_err(|e| ClientError::CosmosSdk {
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
        if event.type_str == key_name {
            return Some(event.clone());
        }
    }
    None
}

use std::{str::FromStr, time::Duration};

use clap::Parser;
use cosm_orc::client::{
    chain_res::{ExecResponse, InstantiateResponse, MigrateResponse, QueryResponse, StoreCodeResponse},
    error::ClientError,
};
use cosmos_sdk_proto::{
    cosmwasm::wasm::v1::{QuerySmartContractStateRequest, QuerySmartContractStateResponse},
    traits::Message,
};
use cosmrs::{
    cosmwasm::{MsgExecuteContract, MsgInstantiateContract, MsgMigrateContract, MsgStoreCode},
    rpc::{Client, HttpClient},
    tendermint::abci::tag::Key,
    tx::Msg,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time;

use crate::{
    cosmrs::{abci_query, find_event, send_tx},
    file::ChainInfo,
    key::UserKey,
};

#[derive(Deserialize, Parser, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, JsonSchema)]
pub struct Coin {
    pub denom:  String,
    pub amount: u64,
}

impl TryFrom<Coin> for cosmrs::Coin {
    type Error = ClientError;

    fn try_from(value: Coin) -> Result<Self, ClientError> {
        Ok(Self {
            denom:  value.denom.parse().map_err(|_| ClientError::Denom { name: value.denom.clone() })?,
            amount: value.amount.into(),
        })
    }
}

#[cfg_attr(test, faux::create)]
#[derive(Clone, Debug)]
pub struct CosmWasmClient {
    // http tendermint RPC client
    rpc_client: HttpClient,
    cfg:        ChainInfo,
}

#[cfg_attr(test, faux::methods)]
impl CosmWasmClient {
    // HACK: faux doesn't support mocking a struct wrapped in a Result
    // so we are just ignoring the constructor for this crate's tests
    //#[cfg(not(test))]
    pub fn new(cfg: ChainInfo) -> Result<Self, ClientError> {
        Ok(Self { rpc_client: HttpClient::new(cfg.rpc_endpoint.as_str())?, cfg })
    }

    pub async fn store(
        &self, payload: Vec<u8>, key: &UserKey, instantiate_perms: Option<cosm_orc::orchestrator::AccessConfig>,
    ) -> Result<StoreCodeResponse, ClientError> {
        let account_id = key.to_account(&self.cfg.prefix).await?;

        let msg = MsgStoreCode {
            sender:                 account_id.clone(),
            wasm_byte_code:         payload,
            instantiate_permission: instantiate_perms
                .map(|p| p.try_into())
                .transpose()
                .map_err(|e| ClientError::InstantiatePerms { source: e })?,
        }
        .to_any()
        .map_err(ClientError::proto_encoding)?;

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        let res = find_event(&tx_res, "store_code").unwrap();

        let code_id = res
            .attributes
            .iter()
            .find(|a| a.key == Key::from_str("code_id").unwrap())
            .unwrap()
            .value
            .as_ref()
            .parse::<u64>()
            .unwrap();

        Ok(StoreCodeResponse {
            code_id,
            tx_hash: tx_res.hash.to_string(),
            height: tx_res.height.into(),
            res: tx_res.deliver_tx.into(),
        })
    }

    pub async fn instantiate(
        &self, code_id: u64, payload: Vec<u8>, key: &UserKey, admin: Option<String>, funds: Vec<Coin>,
    ) -> Result<InstantiateResponse, ClientError> {
        let account_id = key.to_account(&self.cfg.prefix).await?;

        let mut cosm_funds = vec![];
        for fund in funds {
            cosm_funds.push(fund.try_into()?);
        }

        let msg = MsgInstantiateContract {
            sender: account_id.clone(),
            admin: admin.map(|s| s.parse()).transpose().map_err(|_| ClientError::AdminAddress)?,
            code_id,
            label: Some("cosm-orc".to_string()),
            msg: payload,
            funds: cosm_funds,
        }
        .to_any()
        .map_err(ClientError::proto_encoding)?;

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        let res = find_event(&tx_res, "instantiate").unwrap();

        let addr = res
            .attributes
            .iter()
            .find(|a| a.key == Key::from_str("_contract_address").unwrap())
            .unwrap()
            .value
            .to_string();

        Ok(InstantiateResponse {
            address: addr,
            tx_hash: tx_res.hash.to_string(),
            height:  tx_res.height.into(),
            res:     tx_res.deliver_tx.into(),
        })
    }

    pub async fn execute(
        &self, address: String, payload: Vec<u8>, key: &UserKey, funds: Vec<Coin>,
    ) -> Result<ExecResponse, ClientError> {
        let account_id = key.to_account(&self.cfg.prefix).await?;

        let mut cosm_funds = vec![];
        for fund in funds {
            cosm_funds.push(fund.try_into()?);
        }

        let msg = MsgExecuteContract {
            sender:   account_id.clone(),
            contract: address.parse().unwrap(),
            msg:      payload,
            funds:    cosm_funds,
        }
        .to_any()
        .map_err(ClientError::proto_encoding)?;

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        Ok(ExecResponse {
            tx_hash: tx_res.hash.to_string(),
            height:  tx_res.height.into(),
            res:     tx_res.deliver_tx.into(),
        })
    }

    pub async fn query(&self, address: String, payload: Vec<u8>) -> Result<QueryResponse, ClientError> {
        let res = abci_query(
            &self.rpc_client,
            QuerySmartContractStateRequest { address: address.parse().unwrap(), query_data: payload },
            "/cosmwasm.wasm.v1.Query/SmartContractState",
        )
        .await?;

        let res = QuerySmartContractStateResponse::decode(res.value.as_slice()).map_err(ClientError::prost_proto_de)?;

        Ok(QueryResponse { res: res.into() })
    }

    pub async fn migrate(
        &self, address: String, new_code_id: u64, payload: Vec<u8>, key: &UserKey,
    ) -> Result<MigrateResponse, ClientError> {
        let account_id = key.to_account(&self.cfg.prefix).await?;

        let msg = MsgMigrateContract {
            sender:   account_id.clone(),
            contract: address.parse().unwrap(),
            code_id:  new_code_id,
            msg:      payload,
        }
        .to_any()
        .map_err(ClientError::proto_encoding)?;

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        Ok(MigrateResponse {
            tx_hash: tx_res.hash.to_string(),
            height:  tx_res.height.into(),
            res:     tx_res.deliver_tx.into(),
        })
    }

    pub async fn poll_for_n_blocks(&self, n: u64, is_first_block: bool) -> Result<(), ClientError> {
        if is_first_block {
            self.rpc_client.wait_until_healthy(Duration::from_secs(5)).await?;

            while let Err(e) = self.rpc_client.latest_block().await {
                if !matches!(e.detail(), cosmrs::rpc::error::ErrorDetail::Serde(_)) {
                    return Err(e.into());
                }
                time::sleep(Duration::from_millis(500)).await;
            }
        }

        let mut curr_height: u64 = self.rpc_client.latest_block().await?.block.header.height.into();
        let target_height: u64 = curr_height + n;

        while curr_height < target_height {
            time::sleep(Duration::from_millis(500)).await;

            curr_height = self.rpc_client.latest_block().await?.block.header.height.into();
        }

        Ok(())
    }
}

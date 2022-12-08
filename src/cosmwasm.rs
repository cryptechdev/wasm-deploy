use std::time::Duration;

use clap::Parser;
use cosmos_sdk_proto::{
    cosmwasm::wasm::v1::{QuerySmartContractStateRequest, QuerySmartContractStateResponse},
    traits::Message,
};
use cosmrs::{
    cosmwasm::{AccessConfig, MsgExecuteContract, MsgInstantiateContract, MsgStoreCode},
    rpc::{Client, HttpClient},
};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::time;

use crate::{
    chain_res::*,
    cosmrs::{abci_query, find_event, send_tx},
    error::DeployError,
    file::ChainInfo,
    key::UserKey,
};

#[derive(Deserialize, Parser, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, JsonSchema)]
pub struct Coin {
    pub denom:  String,
    pub amount: u64,
}

impl TryFrom<Coin> for cosmrs::Coin {
    type Error = DeployError;

    fn try_from(value: Coin) -> Result<Self, DeployError> {
        Ok(Self {
            denom:  value.denom.parse().map_err(|_| DeployError::Denom { name: value.denom.clone() })?,
            amount: value.amount.into(),
        })
    }
}

#[cfg_attr(test, faux::create)]
#[derive(Clone, Debug)]
pub struct CosmWasmClient {
    // http tendermint RPC client
    pub rpc_client: HttpClient,
    pub cfg:        ChainInfo,
}

#[cfg_attr(test, faux::methods)]
impl CosmWasmClient {
    // HACK: faux doesn't support mocking a struct wrapped in a Result
    // so we are just ignoring the constructor for this crate's tests
    //#[cfg(not(test))]
    pub fn new(cfg: ChainInfo) -> Result<Self, DeployError> {
        Ok(Self { rpc_client: HttpClient::new(cfg.rpc_endpoint.as_str())?, cfg })
    }

    pub async fn store(
        &self, payload: Vec<u8>, key: &UserKey, instantiate_perms: Option<AccessConfig>,
    ) -> Result<StoreCodeResponse, DeployError> {
        let account_id = key.to_account(&self.cfg.derivation_path, &self.cfg.prefix).await?;
        let msg = MsgStoreCode {
            sender:                 account_id.clone(),
            wasm_byte_code:         payload,
            instantiate_permission: instantiate_perms,
        };

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        let res = find_event(&tx_res, "store_code").unwrap();

        let code_id = res.attributes.iter().find(|a| a.key == "code_id").unwrap().value.parse::<u64>().unwrap();

        Ok(StoreCodeResponse {
            code_id,
            tx_hash: tx_res.hash.to_string(),
            height: tx_res.height.into(),
            res: tx_res.deliver_tx.into(),
        })
    }

    pub async fn instantiate(
        &self, code_id: u64, payload: Vec<u8>, key: &UserKey, admin: Option<String>, funds: Vec<Coin>,
    ) -> Result<InstantiateResponse, DeployError> {
        let account_id = key.to_account(&self.cfg.derivation_path, &self.cfg.prefix).await?;

        let mut cosm_funds = vec![];
        for fund in funds {
            cosm_funds.push(fund.try_into()?);
        }

        let msg = MsgInstantiateContract {
            sender: account_id.clone(),
            admin: admin.map(|s| s.parse()).transpose().map_err(|_| DeployError::AdminAddress)?,
            code_id,
            label: Some("cosm-orc".to_string()),
            msg: payload,
            funds: cosm_funds,
        };

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        let res = find_event(&tx_res, "instantiate").unwrap();

        let addr = res.attributes.iter().find(|a| a.key == "_contract_address").unwrap().value.to_string();

        Ok(InstantiateResponse {
            address: addr,
            tx_hash: tx_res.hash.to_string(),
            height:  tx_res.height.into(),
            res:     tx_res.deliver_tx.into(),
        })
    }

    pub async fn execute(
        &self, address: String, payload: Vec<u8>, key: &UserKey, funds: Vec<Coin>,
    ) -> Result<ExecResponse, DeployError> {
        let account_id = key.to_account(&self.cfg.derivation_path, &self.cfg.prefix).await?;

        let mut cosm_funds = vec![];
        for fund in funds {
            cosm_funds.push(fund.try_into()?);
        }

        let msg = MsgExecuteContract {
            sender:   account_id.clone(),
            contract: address.parse().unwrap(),
            msg:      payload,
            funds:    cosm_funds,
        };

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        Ok(ExecResponse {
            tx_hash: tx_res.hash.to_string(),
            height:  tx_res.height.into(),
            res:     tx_res.deliver_tx.into(),
        })
    }

    pub async fn query(&self, address: String, payload: Vec<u8>) -> Result<QueryResponse, DeployError> {
        let res = abci_query(
            &self.rpc_client,
            QuerySmartContractStateRequest { address: address.parse().unwrap(), query_data: payload },
            "/cosmwasm.wasm.v1.Query/SmartContractState",
        )
        .await?;

        let res = QuerySmartContractStateResponse::decode(res.value.as_slice())?;

        Ok(QueryResponse { res: res.into() })
    }

    pub async fn migrate(
        &self, address: String, new_code_id: u64, payload: Vec<u8>, key: &UserKey,
    ) -> Result<MigrateResponse, DeployError> {
        let account_id = key.to_account(&self.cfg.derivation_path, &self.cfg.prefix).await?;

        let msg = crate::msg_execute_contract::MsgMigrateContract {
            sender:   account_id.clone(),
            contract: address.parse().unwrap(),
            code_id:  new_code_id,
            msg:      payload,
        };

        let tx_res = send_tx(&self.rpc_client, msg, key, account_id, &self.cfg).await?;

        Ok(MigrateResponse {
            tx_hash: tx_res.hash.to_string(),
            height:  tx_res.height.into(),
            res:     tx_res.deliver_tx.into(),
        })
    }

    pub async fn poll_for_n_blocks(&self, n: u64, is_first_block: bool) -> Result<(), DeployError> {
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

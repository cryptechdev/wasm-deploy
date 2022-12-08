use std::convert::TryFrom;

use cosmrs::{tx::Msg, AccountId};

/// MsgMigrateContract runs a code upgrade/ downgrade for a smart contract
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct MsgMigrateContract {
    /// Sender is the that actor that signed the messages
    pub sender: AccountId,

    /// Contract is the address of the smart contract
    pub contract: AccountId,

    /// CodeID references the new WASM code
    pub code_id: u64,

    /// Msg json encoded message to be passed to the contract on migration
    pub msg: Vec<u8>,
}

impl Msg for MsgMigrateContract {
    type Proto = cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract;
}

impl TryFrom<cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract> for MsgMigrateContract {
    type Error = cosmrs::ErrorReport;

    fn try_from(
        proto: cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract,
    ) -> Result<MsgMigrateContract, cosmrs::ErrorReport> {
        Ok(MsgMigrateContract {
            sender:   proto.sender.parse()?,
            contract: proto.contract.parse()?,
            code_id:  proto.code_id,
            msg:      proto.msg,
        })
    }
}

impl From<MsgMigrateContract> for cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract {
    fn from(msg: MsgMigrateContract) -> cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract {
        cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContract {
            sender:   msg.sender.to_string(),
            contract: msg.contract.to_string(),
            code_id:  msg.code_id,
            msg:      msg.msg,
        }
    }
}

/// MsgMigrateContractResponse returns contract migration result data.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct MsgMigrateContractResponse {
    /// Data contains same raw bytes returned as data from the wasm contract.
    /// (May be empty)
    pub data: Vec<u8>,
}

impl Msg for MsgMigrateContractResponse {
    type Proto = cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContractResponse;
}

impl TryFrom<cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContractResponse> for MsgMigrateContractResponse {
    type Error = cosmrs::ErrorReport;

    fn try_from(
        proto: cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContractResponse,
    ) -> Result<MsgMigrateContractResponse, cosmrs::ErrorReport> {
        Ok(MsgMigrateContractResponse { data: proto.data })
    }
}

impl From<MsgMigrateContractResponse> for cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContractResponse {
    fn from(msg: MsgMigrateContractResponse) -> cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContractResponse {
        cosmos_sdk_proto::cosmwasm::wasm::v1::MsgMigrateContractResponse { data: msg.data }
    }
}

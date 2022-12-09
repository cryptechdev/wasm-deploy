pub use cosmrs::{proto, ErrorReport};
use cosmrs::{tx::Msg, AccountId};
use serde::{Deserialize, Serialize};
/// MsgStoreCode submit Wasm code to the system
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct MsgStoreCode {
    /// Sender is the that actor that signed the messages
    pub sender: AccountId,

    /// WASMByteCode can be raw or gzip compressed
    pub wasm_byte_code: Vec<u8>,

    /// InstantiatePermission access control to apply on contract creation,
    /// optional
    pub instantiate_permission: Option<()>,
}

impl Msg for MsgStoreCode {
    type Proto = proto::cosmwasm::wasm::v1::MsgStoreCode;
}

impl TryFrom<proto::cosmwasm::wasm::v1::MsgStoreCode> for MsgStoreCode {
    type Error = ErrorReport;

    fn try_from(proto: proto::cosmwasm::wasm::v1::MsgStoreCode) -> Result<MsgStoreCode, ErrorReport> {
        Ok(MsgStoreCode {
            sender:                 proto.sender.parse()?,
            wasm_byte_code:         proto.wasm_byte_code,
            instantiate_permission: None,
        })
    }
}

impl From<MsgStoreCode> for proto::cosmwasm::wasm::v1::MsgStoreCode {
    fn from(msg: MsgStoreCode) -> proto::cosmwasm::wasm::v1::MsgStoreCode {
        proto::cosmwasm::wasm::v1::MsgStoreCode {
            sender:                 msg.sender.to_string(),
            wasm_byte_code:         msg.wasm_byte_code,
            instantiate_permission: None,
        }
    }
}

/// MsgStoreCodeResponse returns store result data.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct MsgStoreCodeResponse {
    /// CodeID is the reference to the stored WASM code
    pub code_id: u64,
}

impl Msg for MsgStoreCodeResponse {
    type Proto = proto::cosmwasm::wasm::v1::MsgStoreCodeResponse;
}

impl TryFrom<proto::cosmwasm::wasm::v1::MsgStoreCodeResponse> for MsgStoreCodeResponse {
    type Error = ErrorReport;

    fn try_from(proto: proto::cosmwasm::wasm::v1::MsgStoreCodeResponse) -> Result<MsgStoreCodeResponse, ErrorReport> {
        Ok(MsgStoreCodeResponse { code_id: proto.code_id })
    }
}

impl From<MsgStoreCodeResponse> for proto::cosmwasm::wasm::v1::MsgStoreCodeResponse {
    fn from(msg: MsgStoreCodeResponse) -> proto::cosmwasm::wasm::v1::MsgStoreCodeResponse {
        proto::cosmwasm::wasm::v1::MsgStoreCodeResponse { code_id: msg.code_id }
    }
}
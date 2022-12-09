use cosmrs::AccountId;
pub use cosmrs::{proto, ErrorReport};
use serde::{Deserialize, Serialize};

use super::ContractCodeId;
/// CodeInfoResponse contains code meta data from CodeInfo
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct CodeInfoResponse {
    /// CodeId of the stored contract code.
    pub code_id: ContractCodeId,

    /// Bech32 [`AccountId`] of the creator of this smart contract.
    pub creator: AccountId,

    /// sha256 hash of the code stored
    pub data_hash: Vec<u8>,
}

impl TryFrom<proto::cosmwasm::wasm::v1::CodeInfoResponse> for CodeInfoResponse {
    type Error = ErrorReport;

    fn try_from(proto: proto::cosmwasm::wasm::v1::CodeInfoResponse) -> Result<CodeInfoResponse, ErrorReport> {
        Ok(
            CodeInfoResponse {
                code_id:   proto.code_id,
                creator:   proto.creator.parse()?,
                data_hash: proto.data_hash,
            },
        )
    }
}

impl From<CodeInfoResponse> for proto::cosmwasm::wasm::v1::CodeInfoResponse {
    fn from(code_info: CodeInfoResponse) -> Self {
        proto::cosmwasm::wasm::v1::CodeInfoResponse {
            code_id:   code_info.code_id,
            creator:   code_info.creator.to_string(),
            data_hash: code_info.data_hash,
        }
    }
}

use cosmos_sdk_proto::cosmwasm::wasm::v1::ContractCodeHistoryOperationType;
use cosmrs::Error;
pub use cosmrs::{proto, ErrorReport};

use super::{AbsoluteTxPosition, ContractCodeId};
/// ContractCodeHistoryEntry metadata to a contract.
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct ContractCodeHistoryEntry {
    /// The source of this history entry.
    pub operation: ContractCodeHistoryOperationType,

    /// Reference to the stored Wasm code.
    pub code_id: ContractCodeId,

    /// Updated Tx position when the operation was executed.
    pub updated: Option<AbsoluteTxPosition>,

    /// Raw message returned by a wasm contract.
    pub msg: Vec<u8>,
}

impl TryFrom<proto::cosmwasm::wasm::v1::ContractCodeHistoryEntry> for ContractCodeHistoryEntry {
    type Error = ErrorReport;

    fn try_from(
        proto: proto::cosmwasm::wasm::v1::ContractCodeHistoryEntry,
    ) -> Result<ContractCodeHistoryEntry, ErrorReport> {
        Ok(ContractCodeHistoryEntry {
            operation: ContractCodeHistoryOperationType::from_i32(proto.operation)
                .ok_or(Error::InvalidEnumValue { name: "operation", found_value: proto.operation })?,
            code_id:   proto.code_id,
            updated:   None,
            msg:       vec![],
        })
    }
}

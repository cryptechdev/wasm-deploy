use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Env {
    pub is_active: bool,
    pub env_id: String,
    #[serde(alias = "chain_id")]
    pub chain_label: String,
    pub contracts: Vec<ContractInfo>,
    pub key_name: String,
}

impl Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.env_id.fmt(f)
    }
}

#[derive(Clone, Debug, JsonSchema, PartialEq, Serialize, Deserialize)]
pub struct ContractInfo {
    pub name: String,
    pub addr: Option<String>,
    pub code_id: Option<u64>,
}

impl Display for ContractInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

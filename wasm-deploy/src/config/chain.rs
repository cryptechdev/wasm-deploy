use std::collections::HashMap;

use cosm_utils::config::cfg::ChainConfig;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use shrinkwraprs::Shrinkwrap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChainInfo {
    #[serde(flatten)]
    pub cfg: ChainConfig,
    pub rpc_endpoint: String,
}

// Prior versions
pub type ChainsV0_1 = Vec<ChainInfo>;

/// Most recent version
#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(from = "ChainVersions")]
pub struct Chains(pub HashMap<String, ChainInfo>);

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum ChainVersions {
    V0_6(Chains),
    V0_1(ChainsV0_1),
}

impl From<ChainVersions> for Chains {
    fn from(value: ChainVersions) -> Self {
        match value {
            ChainVersions::V0_6(v) => v,
            ChainVersions::V0_1(v) =>  {
                let map = v
                .into_iter()
                .map(|x| (x.cfg.chain_id.clone(), x))
                .collect::<HashMap<String, ChainInfo>>();
                Chains(map)
            },
        }
    }
}
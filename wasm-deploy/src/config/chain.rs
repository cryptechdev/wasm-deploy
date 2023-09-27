use std::collections::HashMap;

use cosm_utils::config::cfg::ChainConfig;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChainInfo {
    pub cfg: ChainConfig,
    pub rpc_endpoint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct ChainInfoV0_1 {
    #[serde(flatten)]
    pub cfg: ChainConfig,
    pub rpc_endpoint: String,
}

// All versions
type ChainsV0_6 = HashMap<String, ChainInfo>;
type ChainsV0_1 = Vec<ChainInfoV0_1>;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
enum ChainVersions {
    V0_6(ChainsV0_6),
    V0_1(ChainsV0_1),
}

/// Most recent version
#[derive(Shrinkwrap)]
#[shrinkwrap(mutable)]
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
// TODO: why did this suddenly break?
// #[serde(from = "ChainVersions")]
pub struct Chains(pub ChainsV0_6);

impl From<ChainVersions> for Chains {
    fn from(value: ChainVersions) -> Self {
        match value {
            ChainVersions::V0_6(v) => Chains(v),
            ChainVersions::V0_1(v) => {
                let map = v
                    .into_iter()
                    .map(|x| {
                        (
                            x.cfg.chain_id.clone(),
                            ChainInfo {
                                cfg: x.cfg,
                                rpc_endpoint: x.rpc_endpoint,
                            },
                        )
                    })
                    .collect::<HashMap<String, ChainInfo>>();
                Chains(map)
            }
        }
    }
}

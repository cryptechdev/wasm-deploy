// Use this file to define the various default message you want deploy to use
use cw20::MinterResponse;
use lazy_static::lazy_static;
use wasm_deploy::{
    contract::{ContractInteractive, ExternalInstantiate},
    utils::{get_addr, get_code_id},
};

use crate::contract::Contracts;

pub const ADMIN: &str = "noria19n42dwl6mgwcep5ytqt7qpthy067ssq72gjsrk";

// Using lazy_static helps us create the messages that we need for the various deployment stages.
lazy_static! {

    /// Here we define the default instantiate message for the cw20_base contract/
    /// This message will be sent every time we redeploy the contract.
    pub static ref CW20_INSTANTIATE: cw20_base::msg::InstantiateMsg = cw20_base::msg::InstantiateMsg {
        decimals: 6,
        initial_balances: vec![],
        marketing: None,
        mint: Some(MinterResponse { cap: None, minter: ADMIN.into() }),
        symbol: "uwasmdeploy".into(),
        name: "WASM_DEPLOY_TEST".into(),
    };

    /// Perhaps we want to mint some tokens after the contract is deployed.
    /// We could send this message as part of the set_up_msgs.
    pub static ref CW20_MINT: Vec<cw20_base::msg::ExecuteMsg> = vec![
        cw20_base::msg::ExecuteMsg::Mint { recipient: ADMIN.into(), amount: 1_000_000_000u64.into() },
        cw20_base::msg::ExecuteMsg::Mint { recipient: ADMIN.into(), amount: 1_200_000_000u64.into() },
    ];

    /// External instantiate is a niche feature that allows you to instantiate external contracts from a code_id
    /// alongside your own contract. This is useful if your contract depends on other external contracts.
    pub static ref EXTERNAL_INSTANTIATE: Vec<ExternalInstantiate<cw20_base::msg::InstantiateMsg>> = {
        // We can fetch the current code id from an existing contract like this:
        let code_id = get_code_id(Contracts::Cw20Base.name().as_str()).unwrap();
        // you can fetch the current address of an instantiated contract like this:
        let _addr = get_addr(Contracts::Cw20Base.name().as_str()).unwrap();
        // This external instantiate will use the code id we just fetched
        vec![
            ExternalInstantiate {
                name: "cw20_base".into(),
                msg: CW20_INSTANTIATE.to_owned(),
                code_id,
            }
        ]
    };
}

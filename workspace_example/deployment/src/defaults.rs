// Use this file to define the various default message you want deploy to use
use cw20::MinterResponse;
use lazy_static::lazy_static;

pub const ADMIN: &str = "cosmos19r3350dnszl6r7r9mtlneccr9p9hpwe6gzs0l8";

lazy_static! {
    pub static ref CW20_INSTANTIATE: cw20_base::msg::InstantiateMsg = cw20_base::msg::InstantiateMsg {
        decimals:         6,
        initial_balances: vec![],
        marketing:        None,
        mint:             Some(MinterResponse { cap: None, minter: ADMIN.into() }),
        symbol:           "uwasmdeploy".into(),
        name:             "WASM_DEPLOY_TEST.into()".into(),
    };
    pub static ref CW20_MINT: Vec<cw20_base::msg::ExecuteMsg> = vec![
        cw20_base::msg::ExecuteMsg::Mint { recipient: ADMIN.into(), amount: 1_000_000_000u64.into() },
        cw20_base::msg::ExecuteMsg::Mint { recipient: ADMIN.into(), amount: 1_200_000_000u64.into() },
    ];
}

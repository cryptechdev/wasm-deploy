use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use wasm_deploy_derive::contracts;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub enum SampleMsg {
    Foo,
    Bar,
}

pub const ADMIN: &str = "I am const";

fn main() {
    let my_var = "asdfasdf";
    contracts!(
        {
            name: "doodoo",
            admin: "admin",
            instantiate: SampleMsg,
            execute: String,
            query: String,
            migrate: String,
        },
        {
            name: "asdfasdf",
            admin: "asdfasdf",
            instantiate: SampleMsg,
        }
    );
}

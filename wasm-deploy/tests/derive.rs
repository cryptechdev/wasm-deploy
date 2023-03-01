use wasm_deploy_derive::contract;

#[contract]
enum Contract {
    Hello,
    World,
}

#[test]
fn test() {
    let contract = Contract::Hello;
    assert_eq!(contract.to_string(), "hello");
}

use wasm_deploy_derive::contracts;

pub const ADMIN: &str = "I am const";

#[test]
fn test() {
    #[contracts]
    pub enum TestContracts {
        #[contract(
            rename = "renamed", 
            bin_name = "my_bin", 
            path = "my_path", 
            admin = "my_admin", 
            instantiate = String,
            execute = String,
            query = String,
            migrate = String,
            cw20_send = String,
        )]
        FooBar,

        #[contract(
            rename = ADMIN, 
            bin_name = ADMIN, 
            path = ADMIN, 
            admin = ADMIN, 
            instantiate = String,
            execute = String,
            query = String,
            migrate = String,
            cw20_send = String,
        )]
        Baz,
    }
}

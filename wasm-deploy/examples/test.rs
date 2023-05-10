use wasm_deploy_derive::contracts;

pub const ADMIN: &str = "I am const";

fn main() {
    let admin = "woah_dude";
    #[contracts]
    pub enum TestContracts {
        #[contract(admin = ADMIN, instantiate = String)]
        Foo,
    }
}

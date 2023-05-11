# wasm-deploy

**wasm-deploy is a fully featured deployment suite for complex, multicontract cosmwasm projects**

## Demo

---

![](https://i.imgur.com/AVneDxD.gif)

---

# Headlining Features that make this Awesome!
## Automatically interfaces with your contracts and their APIs including
 - ExecuteMsg
 - QueryMsg
 - InstantiateMsg
 - MigrateMsg
 - QueryMsg
 - Cw20HookMsg
 
## Interactive Parsing of all your JsonSchema types
 - This makes is super easy to send messages to the chain
 - Complicated messages can be sent in a type safe manner with only a few clicks

## Full Build Automation
 - with a single command ```deploy d``` you can build, instantiate, set your configs, and execute any set up for numerous contracts.

## Batch messaging
 - Messages are batched together to save you time!

 ## Awesome Proc Macros
 - wasm-deploy is built on top of some verbose trait, and these macros really simplify the setup process.

  ## Automatically negotiates the tendermint version
 - automatically queries the node in order to correctly negotiate the tendermint version.

---

# Getting the example working

The first step is installing wasm-opt and ensuring that it is in your path. Run 
```bash
wasm-opt --version  
``` 

Alternatively you can use the experimental `wasm-opt` feature within wasm-deploy. Simply change the line in your toml file to
```toml
wasm-deploy = { version = "0.5.0", features = ["wasm-opt"] }
```

Go ahead and clone this repo and `cd` into the `workspace_example` folder.

Install wasm-deploy globally with 
```bash
cargo install --path deployment
```
Then you should be able to run
```bash
deploy init
```
This will initialize the deployment config and will prompt you for a bunch of important information.
Before you deploy the contracts, please be sure to change the ADMIN constant in deployment/src/defaults.rs to your personal dev address.

Deploy all contracts with
```bash
deploy d
```

Or specific ones with
```bash
deploy d -c contract_1,contract_2
```

after deploying them to the chain, you can execute the contract with
```bash
deploy execute <contract_name>
```
in this case, use cw20_base in place of contract_name.

If you make changes to your contract API or deployment code you will need to update the wasm-deploy binary by running
```bash
deploy u
```
This currently will install the binary globally.

To see a list of commands please run 
```bash
deploy --help
```

Code Ids and addresses of local contracts can be fetched using `get_code_id(contract_name: &str)` and `get_addr(contract_name: &str)`. This allows you to send messages to other contracts without having to manually insert the address.

# Configuring wasm-deploy to work with a preexisting cosmwasm project

First ensure you have cargo-generate and wasm-opt installed as above.

Install cargo generate with
```bash
cargo install cargo-generate
```

Then cd into your project `cd my-contracts` and run
```bash
cargo generate --init cryptechdev/wasm-deploy template
```
and be sure to name the project after your folder, and pick a custom name for the binary/executable, such as `projd`, that will replace the `deploy` name.

Install wasm-deploy globally with 
```bash
cargo install --path deployment
```

Important Note: The generated deployment folder is a template only. You will have to modify BOTH deployment/src/contract.rs deployment/src/defaults.rs to match your project. The template will not work out of the box. The generated files should have the correct skeleton and plenty of comments to help you along.

## What To Expect

In my opinion, the most powerful cosmwasm deployment software ever built. It is infinitely configurable, automatically interfaces with the apis of your contracts, and is super easy to set up.

## What Not To Expect

Seamless upgrades to newer versions or a super quick initial installation. Since every smart contract workspace requires custom logic for how deployments should proceed, setting up wasm-deploy requires an inherent underlying complexity. This project is made almost entirely in my spare time and is extremely young. I have plans to support it for quite a long while to come, and I should be very responsive to any issues you may have, so please open an issue on github if you run into one. Or better yet, please contribute and submit a PR. This crate is still VERY much in early Alpha stage. This means the entire API is subject to change, Error messages are not likely to be very helpful, and improper use or edge cases are likely to error or cause a panic.

## Project Structure

---
```
workspace-root/
├─ artifacts/
│  ├─ contract_1.wasm
│  ├─ contract_2.wasm
├─ target/
│  ├─ debug/
│  │  ├─ deploy
├─ deploy -> target/debug/deploy
├─ deployment/
│  ├─ src/
│  │  ├─ contracts.rs
│  │  ├─ main.rs
│  │  ├─ Cargo.toml
├─ contracts/
│  ├─ contract_1/
│  │  ├─ Cargo.toml
│  │  ├─ src/
│  ├─ contract_2/
│  │  ├─ Cargo.toml
│  │  ├─ src/
├─ packages/
│  ├─ my_project/
│  │  ├─ contract_1.rs
│  │  ├─ contract_2.rs
```

## Feature List

- [x] Support for tendermint 0.34
- [x] Support for tendermint 0.37
- [x] default compatibility mode for either version
- [x] Full deployment automation
- [x] Interactive parsing of all jsonschema types
- [x] Automatic contract address insertion.
- [x] ExecuteMsg
- [x] QueryMsg
- [x] InstantiateMsg
- [x] MigrateMsg
- [x] QueryMsg
- [x] Cw20HookMsg
- [x] Batching messages of the same type
- [ ] Batching messages of different types
- [x] HTTP client
- [ ] Automatic wasm-deploy compilation
- [x] Mnemonic key
- [x] OS Keyring key
- [ ] Ledger key

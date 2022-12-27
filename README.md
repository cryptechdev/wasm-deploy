# wasm-deploy

**wasm-deploy is a fully featured deployment suite for complex, multicontract cosmwasm projects**

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

---

# Getting the example working

The first step is installing wasm-opt and ensuring that it is in your path. Run 
```bash
wasm-opt --version  
``` 
to ensure that it is installed correctly.

cd into the workspace_example directory and run 
```bash
cargo build
```
This will build the deploy binary. You will notice there is a sym link from the root of the workspace to target/debug/deploy. This is so that you can run the deploy binary from the root of the workspace.

Then you should be able to run
```bash
./deploy init
```
This will innitialize the deployment config and will prompt you for a bunch of information. Please ensure you fill out the optional gRPC endpoint as it is the only client which is currently fully working.

Before you deploy the contracts, please be sure to change the ADMIN constant in deployment/src/defaults.rs to your personal dev address.

Deploy all contracts with
```bash
./deploy d
```

Or specific ones with
```bash
./deploy d -c contract_1,contract_2
```

after deploying them to the chain, you can execute the contract with
```bash
./deploy execute <contract_name>
```
in this case, use cw20_base in place of contract_name.

If you make changes to your contract API or deployment code you will need to update the wasm-deploy binary by running
```bash
./deploy u
```

To see a list of commands please run 
```bash
./deploy --help
```

## What To Expect

In my opinion, the most powerful cosmwasm deployment software ever built. It is infinitely configurable, automatically interfaces with the apis of your contracts, and is super easy to set up.

## What Not To Expect

A bug free experience, and seemless upgrades to newer versions. This project is made almost entirely in my spare time and is extremely young. I have plans to support it for quite a long while to come, and I should be very responsive to any issues you may have, so please open an issue on github if you run into one. Or better yet, please contribute and submit a PR. This crate is still VERY much in early Alpha stage. This means the entire API is subject to change, Error messages are not likely to be very helpful, and improper use or edge cases are likely to error or cause a panic.

## Project Structure

---
```
workspace-root/
├─ artifacts/
│  ├─ contract_1.wasm
│  ├─ .contract_2.wasm
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

your project must be layed out using cargo workspaces. The API and data types that your contracts use should be defined in the packages directory, so that the deployment crate has access to them.

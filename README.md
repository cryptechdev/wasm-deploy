# wasm-deploy

**wasm-deploy is a fully featured deployment sweet for complex, multicontract cosmwasm projects**

---

## Project Structure

---

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

your project must be layed out using cargo workspaces. The API and data types that your contracts use should be defined in the packages directory, so that the deployment crate has access to them. Please

---

## interacting with the contracts

---

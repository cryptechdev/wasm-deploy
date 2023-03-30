# deployment-template

**This template can be used with cargo-generate in order to insert the deployment package withing your preexisting cosmwasm workspace**

To generate just the deployment folder run 
```bash
cargo install cargo-generate
cargo generate cryptechdev/wasm-deploy workspace_example/deployment
```

Please go through all the files in the src folder of the template. Comments should walk you through the process of setting things up.
# deployment-template

**This template can be used with cargo-generate in order to insert the deployment package withing your preexisting cosmwasm workspace**

To generate just the deployment folder run 
```bash
cargo install cargo-generate
cargo generate --init cryptechdev/wasm-deploy cryptechdev/wasm-deploy minimal_template
```
from your project directory

Please go through all the files in the src folder of the template. Comments should walk you through the process of setting things up.
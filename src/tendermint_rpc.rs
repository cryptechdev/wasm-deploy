#[cfg(test)]
mod test {
    use tendermint_rpc::{HttpClient, Client};

    #[tokio::test]
    pub async fn latest_block() {
        let client = HttpClient::new("http://167.99.177.244:26657").unwrap();
        println!("{:?}", client.latest_block().await);
    }
}
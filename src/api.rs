pub async fn fetch_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let method = if self.query.len() == 44 {
            "getAccountInfo"
        } else {
            "getTransaction"
        };

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": [self.query, {"encoding": "base58"}]
        });

        let response = self
            .client
            .post("https://rpc.devnet.soo.network/rpc")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            self.json_response = Some(response.json().await?);
        } else {
            eprintln!("Request failed with status: {}", response.status());
            self.json_response = None;
        }

        Ok(())
    }


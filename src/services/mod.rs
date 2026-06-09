pub mod bedrock;
pub mod indicators;
pub mod kr;
pub mod krx;
pub mod news;
pub mod simulation;
pub mod yahoo;

use std::time::Duration;

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (StockMonitor/1.0; Rust)")
        .build()
        .expect("valid reqwest client")
}

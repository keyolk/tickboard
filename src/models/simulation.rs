use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartData {
    pub prices: Vec<f64>,
    pub dates: Vec<String>,
    pub volumes: Vec<u64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub price: f64,
    pub volume: u64,
    pub is_bid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorRow {
    pub date: String,
    pub individual: i64,
    pub foreign: i64,
    pub institution: i64,
}

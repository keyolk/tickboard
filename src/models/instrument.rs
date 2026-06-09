use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstrumentKind {
    Stock,
    Etf,
    BondFund,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingInfo {
    pub name: String,
    pub weight_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentProfile {
    pub symbol: String,
    pub display_name: String,
    pub kind: InstrumentKind,
    pub issuer: String,
    pub category: String,
    pub objective: String,
    pub benchmark: Option<String>,
    pub expense_ratio_pct: Option<f64>,
    pub distribution_yield_pct: Option<f64>,
    pub duration_years: Option<f64>,
    pub top_holdings: Vec<HoldingInfo>,
    pub notes: Vec<String>,
}

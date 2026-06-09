use std::collections::HashMap;

use crate::models::{
    EconomicIndicator, FxRate, InvestorRow, MarketIndex, NewsItem, OrderBookEntry, StockDetail,
    StockQuote,
};

#[derive(Debug)]
#[allow(clippy::large_enum_variant, clippy::enum_variant_names)]
pub enum AppMsg {
    DashboardIndicesLoaded {
        generation: u64,
        us: Vec<MarketIndex>,
        kr: Vec<MarketIndex>,
    },
    DashboardIndicatorsLoaded {
        generation: u64,
        indicators: Vec<EconomicIndicator>,
        fx_rates: Vec<FxRate>,
    },
    DashboardQuotesLoaded {
        generation: u64,
        us: Vec<StockQuote>,
        kr: Vec<StockQuote>,
        etfs: Vec<StockQuote>,
        bonds: Vec<StockQuote>,
    },
    DashboardMarketCapsLoaded {
        generation: u64,
        us: HashMap<String, f64>,
        kr: HashMap<String, f64>,
        etfs: HashMap<String, f64>,
        bonds: HashMap<String, f64>,
    },
    NewsLoaded {
        generation: u64,
        news: Vec<NewsItem>,
    },
    DetailLoaded {
        generation: u64,
        detail: Option<StockDetail>,
        error: Option<String>,
    },
    FundamentalsLoaded {
        generation: u64,
        data: HashMap<String, f64>,
    },
    CompanyNewsLoaded {
        generation: u64,
        news: Vec<NewsItem>,
    },
    OrderBookLoaded {
        generation: u64,
        entries: Vec<OrderBookEntry>,
    },
    InvestorTrendsLoaded {
        generation: u64,
        rows: Vec<InvestorRow>,
    },
    StockAiLoaded {
        generation: u64,
        result: String,
    },
    ArticleLoaded {
        generation: u64,
        content: String,
        analysis: String,
    },
}

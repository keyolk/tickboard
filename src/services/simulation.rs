use chrono::{Datelike, Duration, Local};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::models::{InvestorRow, Market, OrderBookEntry};

pub async fn fetch_order_book(symbol: &str, market: Market) -> Vec<OrderBookEntry> {
    let price = current_price(symbol, market).await.unwrap_or_default();
    if price <= 0.0 {
        return Vec::new();
    }
    simulate_order_book(price, market)
}

pub async fn fetch_investor_trends(symbol: &str, market: Market, days: usize) -> Vec<InvestorRow> {
    let mut rows = match market {
        Market::Us => fetch_us_investor_trends(symbol, days).await,
        Market::Kr => {
            let ohlcv = super::krx::fetch_recent_ohlcv(symbol, 40).await;
            build_kr_investor_rows_from_ohlcv(ohlcv, days)
        }
    };
    if rows.is_empty() {
        rows = simulate_investor_rows(symbol, market, days);
    }
    rows
}

async fn fetch_us_investor_trends(symbol: &str, days: usize) -> Vec<InvestorRow> {
    let client = super::http_client();
    let series = super::yahoo::fetch_chart(&client, symbol, "1mo", "1d")
        .await
        .ok();
    let mut rows = Vec::new();
    if let Some(series) = series {
        let n = days.min(series.closes.len());
        let start = series.closes.len().saturating_sub(n);
        for idx in start..series.closes.len() {
            let close = series.closes[idx];
            let open = series.opens.get(idx).copied().unwrap_or(close);
            let volume = series.volumes.get(idx).copied().unwrap_or_default() as i64;
            let direction = if close >= open { 1 } else { -1 };
            rows.push(InvestorRow {
                date: series.dates.get(idx).cloned().unwrap_or_default(),
                individual: (volume as f64 * 0.1 * 0.05) as i64 * -direction,
                foreign: (volume as f64 * 0.15 * 0.8) as i64 * direction,
                institution: (volume as f64 * 0.7 * 0.1) as i64 * direction,
            });
        }
    }
    rows
}

fn build_kr_investor_rows_from_ohlcv(
    rows: Vec<super::krx::OhlcvRow>,
    days: usize,
) -> Vec<InvestorRow> {
    let n = days.min(rows.len());
    let start = rows.len().saturating_sub(n);
    rows[start..]
        .iter()
        .map(|row| {
            let volume = row.volume as i64;
            let direction = if row.close >= row.open { 1 } else { -1 };
            InvestorRow {
                date: format!("{:02}/{:02}", row.date.month(), row.date.day()),
                individual: (volume as f64 * 0.6 * 0.05) as i64 * -direction,
                foreign: (volume as f64 * 0.3 * 0.08) as i64 * direction,
                institution: (volume as f64 * 0.1 * 0.15) as i64 * direction,
            }
        })
        .collect()
}

pub fn simulate_order_book(price: f64, market: Market) -> Vec<OrderBookEntry> {
    let bid = price * 0.999;
    let ask = price * 1.001;
    let tick = match market {
        Market::Kr if price > 50_000.0 => 500.0,
        Market::Kr => 100.0,
        Market::Us if price < 10.0 => 0.01,
        Market::Us if price < 50.0 => 0.05,
        Market::Us => 0.10,
    };
    let mut rng = StdRng::seed_from_u64((price * 100.0) as u64);
    let mut entries = Vec::with_capacity(20);
    for idx in 0..10 {
        entries.push(OrderBookEntry {
            price: ask + tick * idx as f64,
            volume: (100.0 * rng.gen_range(0.3..3.0)) as u64,
            is_bid: false,
        });
    }
    for idx in 0..10 {
        entries.push(OrderBookEntry {
            price: bid - tick * idx as f64,
            volume: (100.0 * rng.gen_range(0.3..3.0)) as u64,
            is_bid: true,
        });
    }
    entries
}

async fn current_price(symbol: &str, market: Market) -> Option<f64> {
    let client = super::http_client();
    match market {
        Market::Us => {
            if let Ok(summary) = super::yahoo::fetch_quote_summary(&client, symbol).await {
                if summary.price > 0.0 {
                    return Some(summary.price);
                }
            }
            if let Ok(series) = super::yahoo::fetch_chart(&client, symbol, "7d", "1d").await {
                if let Some(price) = series.latest_close() {
                    if price > 0.0 {
                        return Some(price);
                    }
                }
            }
            None
        }
        Market::Kr => {
            if let Some(data) = super::krx::fetch_quote_data(&client, symbol).await {
                return Some(data.price);
            }
            for suffix in crate::config::kr_yahoo_candidates(symbol) {
                let yahoo_symbol = format!("{symbol}{suffix}");
                if let Ok(summary) = super::yahoo::fetch_quote_summary(&client, &yahoo_symbol).await
                {
                    if summary.price > 0.0 {
                        return Some(summary.price);
                    }
                }
            }
            None
        }
    }
}

fn simulate_investor_rows(symbol: &str, market: Market, days: usize) -> Vec<InvestorRow> {
    let seed = symbol.bytes().fold(0_u64, |acc, b| acc + b as u64) + market as u64;
    let mut rng = StdRng::seed_from_u64(seed);
    (0..days)
        .rev()
        .map(|offset| {
            let date = Local::now().date_naive() - Duration::days(offset as i64);
            let direction = if rng.gen_bool(0.5) { 1 } else { -1 };
            InvestorRow {
                date: format!("{:02}/{:02}", date.month(), date.day()),
                individual: rng.gen_range(1_000..50_000) * -direction,
                foreign: rng.gen_range(1_000..40_000) * direction,
                institution: rng.gen_range(1_000..35_000) * direction,
            }
        })
        .collect()
}

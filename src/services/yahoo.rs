use std::collections::HashMap;

use chrono::{Local, TimeZone};
use futures::{stream, StreamExt};
use serde_json::Value;

use crate::{
    config,
    error::AppResult,
    models::{Currency, Market, MarketIndex, StockDetail, StockQuote},
};

const YAHOO_CHART_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const YAHOO_QUOTE_BASE: &str = "https://query1.finance.yahoo.com/v7/finance/quote";

#[derive(Debug, Clone, Default)]
pub struct ChartSeries {
    pub closes: Vec<f64>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub volumes: Vec<u64>,
    pub dates: Vec<String>,
}

impl ChartSeries {
    pub fn latest_close(&self) -> Option<f64> {
        self.closes
            .last()
            .copied()
            .filter(|v| v.is_finite() && *v > 0.0)
    }

    pub fn previous_close(&self) -> Option<f64> {
        if self.closes.len() >= 2 {
            self.closes.get(self.closes.len() - 2).copied()
        } else {
            self.latest_close()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QuoteSummary {
    pub price: f64,
    pub prev_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub volume: u64,
    pub market_cap: f64,
    pub week52_high: f64,
    pub week52_low: f64,
}

pub async fn fetch_indices(indices: &[(&str, &str)]) -> Vec<MarketIndex> {
    let client = super::http_client();
    let owned = indices
        .iter()
        .map(|(symbol, name)| ((*symbol).to_string(), (*name).to_string()))
        .collect::<Vec<_>>();
    stream::iter(owned)
        .map(|(symbol, name)| {
            let client = client.clone();
            async move {
                let series = fetch_chart(&client, &symbol, "5d", "1d").await.ok()?;
                let value = series.latest_close()?;
                let prev = series.previous_close().unwrap_or(value);
                let change = value - prev;
                Some(MarketIndex {
                    symbol,
                    name,
                    value,
                    change,
                    change_pct: if prev != 0.0 {
                        change / prev * 100.0
                    } else {
                        0.0
                    },
                    last_updated: Some(Local::now()),
                })
            }
        })
        .buffer_unordered(6)
        .filter_map(async move |item| item)
        .collect()
        .await
}

pub async fn fetch_us_indices() -> Vec<MarketIndex> {
    fetch_indices(config::US_INDICES).await
}

pub async fn fetch_kr_indices() -> Vec<MarketIndex> {
    fetch_indices(config::KR_INDICES).await
}

pub async fn fetch_us_quotes(symbols: &[&str]) -> Vec<StockQuote> {
    let client = super::http_client();
    let owned = symbols.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    stream::iter(owned)
        .map(|symbol| {
            let client = client.clone();
            async move {
                fetch_stock_quote(&client, &symbol, &symbol, Market::Us)
                    .await
                    .ok()
                    .flatten()
            }
        })
        .buffer_unordered(10)
        .filter_map(async move |item| item)
        .collect()
        .await
}

pub async fn fetch_stock_quote(
    client: &reqwest::Client,
    display_symbol: &str,
    yahoo_symbol: &str,
    market: Market,
) -> AppResult<Option<StockQuote>> {
    let series = fetch_chart(client, yahoo_symbol, "7d", "1d").await?;
    let price = match series.latest_close() {
        Some(price) if price > 0.0 => price,
        _ => return Ok(None),
    };
    let prev_close = series.previous_close().unwrap_or(price);
    let change = price - prev_close;
    let currency = if market == Market::Kr {
        Currency::Krw
    } else {
        Currency::Usd
    };
    let sector = config::stock_sector(market, display_symbol).to_string();
    let tags = config::instrument_tags(display_symbol, &sector);
    Ok(Some(StockQuote {
        symbol: display_symbol.to_string(),
        name: {
            let configured = config::stock_name(market, display_symbol);
            if configured.is_empty() {
                display_symbol.to_string()
            } else {
                configured.to_string()
            }
        },
        price,
        change,
        change_pct: if prev_close != 0.0 {
            change / prev_close * 100.0
        } else {
            0.0
        },
        volume: series.volumes.last().copied().unwrap_or_default(),
        market,
        currency,
        sector,
        tags,
        market_cap: 0.0,
        history_7d: series.closes,
        last_updated: Some(Local::now()),
    }))
}

pub async fn fetch_us_market_caps(symbols: &[&str]) -> HashMap<String, f64> {
    fetch_quote_market_caps(
        symbols
            .iter()
            .map(|s| ((*s).to_string(), (*s).to_string()))
            .collect(),
    )
    .await
}

pub async fn fetch_quote_market_caps(pairs: Vec<(String, String)>) -> HashMap<String, f64> {
    let client = super::http_client();
    let mut result = HashMap::new();
    for chunk in pairs.chunks(25) {
        let yahoo_symbols = chunk
            .iter()
            .map(|(_, y)| y.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "{YAHOO_QUOTE_BASE}?symbols={}",
            urlencoding::encode(&yahoo_symbols)
        );
        if let Ok(value) = client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            if let Ok(json) = value.json::<Value>().await {
                if let Some(items) = json
                    .pointer("/quoteResponse/result")
                    .and_then(Value::as_array)
                {
                    for item in items {
                        let yahoo = item
                            .get("symbol")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        if let Some((display, _)) = chunk.iter().find(|(_, y)| y.as_str() == yahoo)
                        {
                            if let Some(cap) = value_to_f64(item.get("marketCap")) {
                                result.insert(display.clone(), cap);
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

pub async fn fetch_us_stock_detail(symbol: &str) -> Option<StockDetail> {
    let client = super::http_client();
    fetch_stock_detail(&client, symbol, symbol, Market::Us)
        .await
        .ok()
        .flatten()
}

pub async fn fetch_stock_detail(
    client: &reqwest::Client,
    display_symbol: &str,
    yahoo_symbol: &str,
    market: Market,
) -> AppResult<Option<StockDetail>> {
    let (series, intraday, summary) = tokio::join!(
        fetch_chart(client, yahoo_symbol, "1y", "1d"),
        fetch_chart(client, yahoo_symbol, "1d", "30m"),
        fetch_quote_summary(client, yahoo_symbol),
    );
    let series = series?;
    let intraday = intraday.unwrap_or_default();
    let summary = summary.unwrap_or_default();
    let price = summary.price.max(series.latest_close().unwrap_or_default());
    if price <= 0.0 {
        return Ok(None);
    }
    let prev_close = if summary.prev_close > 0.0 {
        summary.prev_close
    } else {
        series.previous_close().unwrap_or(price)
    };
    let change = price - prev_close;
    let currency = if market == Market::Kr {
        Currency::Krw
    } else {
        Currency::Usd
    };
    let len = series.closes.len();
    let slice = |n: usize| -> (Vec<f64>, Vec<u64>, Vec<String>) {
        let start = len.saturating_sub(n);
        (
            series.closes[start..].to_vec(),
            series.volumes[start..].to_vec(),
            series.dates[start..].to_vec(),
        )
    };
    let (history_7d, volume_history_7d, history_dates_7d) = slice(7);
    let (history_30d, volume_history_30d, history_dates_30d) = slice(30);
    let (history_90d, volume_history_90d, history_dates_90d) = slice(90);
    let (history_1y, volume_history_1y, history_dates_1y) = slice(250);
    let avg_volume = if series.volumes.is_empty() {
        0
    } else {
        let take = series.volumes.len().min(10);
        series.volumes[series.volumes.len() - take..]
            .iter()
            .sum::<u64>()
            / take as u64
    };
    let intraday_times = intraday.dates.clone();
    let intraday_volumes = intraday.volumes.clone();
    let intraday_up_bars = intraday
        .closes
        .iter()
        .zip(intraday.opens.iter())
        .map(|(close, open)| close >= open)
        .collect::<Vec<_>>();
    Ok(Some(StockDetail {
        symbol: display_symbol.to_string(),
        name: config::stock_name(market, display_symbol).to_string(),
        market,
        currency,
        price,
        change,
        change_pct: if prev_close != 0.0 {
            change / prev_close * 100.0
        } else {
            0.0
        },
        open_price: if summary.open > 0.0 {
            summary.open
        } else {
            series.opens.last().copied().unwrap_or(price)
        },
        high: if summary.high > 0.0 {
            summary.high
        } else {
            series.highs.last().copied().unwrap_or(price)
        },
        low: if summary.low > 0.0 {
            summary.low
        } else {
            series.lows.last().copied().unwrap_or(price)
        },
        prev_close,
        volume: if summary.volume > 0 {
            summary.volume
        } else {
            series.volumes.last().copied().unwrap_or_default()
        },
        avg_volume,
        market_cap: summary.market_cap,
        pe_ratio: None,
        week52_high: if summary.week52_high > 0.0 {
            summary.week52_high
        } else {
            max_f64(&series.highs).unwrap_or(price)
        },
        week52_low: if summary.week52_low > 0.0 {
            summary.week52_low
        } else {
            min_f64(&series.lows).unwrap_or(price)
        },
        history_7d,
        history_30d,
        history_90d,
        history_1y,
        intraday_times,
        intraday_volumes,
        intraday_up_bars,
        volume_history_7d,
        volume_history_30d,
        volume_history_90d,
        volume_history_1y,
        history_dates_7d,
        history_dates_30d,
        history_dates_90d,
        history_dates_1y,
        day_change: change,
        day_change_pct: if prev_close != 0.0 {
            change / prev_close * 100.0
        } else {
            0.0
        },
        eps: None,
        dividend_yield: None,
        beta: None,
        sector: config::stock_sector(market, display_symbol).to_string(),
        last_updated: Some(Local::now()),
    }))
}

pub async fn fetch_us_fundamentals(symbol: &str) -> HashMap<String, f64> {
    let client = super::http_client();
    let summary = fetch_quote_summary(&client, symbol)
        .await
        .unwrap_or_default();
    let mut result = HashMap::new();
    if summary.market_cap > 0.0 {
        result.insert("market_cap".to_string(), summary.market_cap);
    }
    result.extend(scrape_yahoo_fundamentals(&client, symbol).await);
    result
}

pub async fn fetch_quote_summary(
    client: &reqwest::Client,
    symbol: &str,
) -> AppResult<QuoteSummary> {
    let url = format!("{YAHOO_QUOTE_BASE}?symbols={}", urlencoding::encode(symbol));
    let json: Value = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let Some(item) = json.pointer("/quoteResponse/result/0") else {
        return Ok(QuoteSummary::default());
    };
    Ok(QuoteSummary {
        price: value_to_f64(item.get("regularMarketPrice")).unwrap_or_default(),
        prev_close: value_to_f64(item.get("regularMarketPreviousClose")).unwrap_or_default(),
        open: value_to_f64(item.get("regularMarketOpen")).unwrap_or_default(),
        high: value_to_f64(item.get("regularMarketDayHigh")).unwrap_or_default(),
        low: value_to_f64(item.get("regularMarketDayLow")).unwrap_or_default(),
        volume: value_to_u64(item.get("regularMarketVolume")).unwrap_or_default(),
        market_cap: value_to_f64(item.get("marketCap")).unwrap_or_default(),
        week52_high: value_to_f64(item.get("fiftyTwoWeekHigh")).unwrap_or_default(),
        week52_low: value_to_f64(item.get("fiftyTwoWeekLow")).unwrap_or_default(),
    })
}

pub async fn fetch_chart(
    client: &reqwest::Client,
    symbol: &str,
    range: &str,
    interval: &str,
) -> AppResult<ChartSeries> {
    let url = format!(
        "{YAHOO_CHART_BASE}/{}?range={range}&interval={interval}&includePrePost=false&events=div%2Csplits",
        urlencoding::encode(symbol)
    );
    let json: Value = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let Some(result) = json.pointer("/chart/result/0") else {
        return Ok(ChartSeries::default());
    };
    let timestamps = result
        .get("timestamp")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let quote = result
        .pointer("/indicators/quote/0")
        .unwrap_or(&Value::Null);
    let closes = quote
        .get("close")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let opens = quote
        .get("open")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let highs = quote
        .get("high")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let lows = quote
        .get("low")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let volumes = quote
        .get("volume")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut series = ChartSeries::default();
    for idx in 0..closes.len() {
        let Some(close) = value_to_f64(closes.get(idx)) else {
            continue;
        };
        if !close.is_finite() || close <= 0.0 {
            continue;
        }
        series.closes.push(close);
        series
            .opens
            .push(value_to_f64(opens.get(idx)).unwrap_or(close));
        series
            .highs
            .push(value_to_f64(highs.get(idx)).unwrap_or(close));
        series
            .lows
            .push(value_to_f64(lows.get(idx)).unwrap_or(close));
        series
            .volumes
            .push(value_to_u64(volumes.get(idx)).unwrap_or_default());
        let date = timestamps
            .get(idx)
            .and_then(Value::as_i64)
            .and_then(|ts| Local.timestamp_opt(ts, 0).single())
            .map(|dt| dt.format("%m/%d").to_string())
            .unwrap_or_default();
        series.dates.push(date);
    }
    Ok(series)
}

async fn scrape_yahoo_fundamentals(client: &reqwest::Client, symbol: &str) -> HashMap<String, f64> {
    let mut result = HashMap::new();
    let url = format!(
        "https://finance.yahoo.com/quote/{}/",
        urlencoding::encode(symbol)
    );
    let Ok(response) = client.get(url).send().await else {
        return result;
    };
    let Ok(text) = response.text().await else {
        return result;
    };
    for (key, pattern) in [
        ("pe_ratio", r#"trailingPE"[^>]*>([\d.]+)"#),
        (
            "eps",
            r#"epsTrailingTwelveMonths"[^>]*data-value="([\d.-]+)"#,
        ),
        ("dividend_yield", r#""dividendYield":\{"raw":([\d.]+)"#),
        (
            "beta",
            r#"Beta \(5Y Monthly\).*?class="value[^"]*">([\d.]+)<"#,
        ),
        ("market_cap", r#""marketCap":\{"raw":(\d+)"#),
    ] {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if let Some(value) = regex
                .captures(&text)
                .and_then(|caps| caps.get(1))
                .and_then(|m| m.as_str().parse::<f64>().ok())
            {
                result.insert(key.to_string(), value);
            }
        }
    }
    result
}

fn value_to_f64(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.replace(',', "").parse().ok(),
        _ => None,
    }
}

fn value_to_u64(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|v| v as u64)),
        Value::String(s) => s.replace(',', "").parse().ok(),
        _ => None,
    }
}

fn max_f64(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .reduce(f64::max)
}

fn min_f64(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.0)
        .reduce(f64::min)
}

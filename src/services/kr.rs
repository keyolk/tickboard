use std::collections::HashMap;

use futures::{stream, StreamExt};
use regex::Regex;

use crate::{
    config,
    models::{Market, StockDetail, StockQuote},
};

pub async fn fetch_kr_quotes(symbols: &[&str]) -> Vec<StockQuote> {
    let quotes = super::krx::fetch_quotes(symbols).await;
    if quotes.len() >= symbols.len() / 2 {
        return quotes;
    }
    let client = super::http_client();
    let owned = symbols.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    stream::iter(owned)
        .map(|symbol| {
            let client = client.clone();
            async move { fetch_kr_quote(&client, &symbol).await }
        })
        .buffer_unordered(10)
        .filter_map(async move |item| item)
        .collect()
        .await
}

pub async fn fetch_kr_stock_detail(symbol: &str) -> Option<StockDetail> {
    if let Some(mut detail) = super::krx::fetch_stock_detail(symbol).await {
        let fundamentals = fetch_kr_fundamentals(symbol).await;
        merge_fundamentals(&mut detail, &fundamentals);
        return Some(detail);
    }
    let client = super::http_client();
    for yahoo_symbol in yahoo_candidates(symbol) {
        if let Ok(Some(mut detail)) =
            super::yahoo::fetch_stock_detail(&client, symbol, &yahoo_symbol, Market::Kr).await
        {
            let fundamentals = fetch_kr_fundamentals(symbol).await;
            merge_fundamentals(&mut detail, &fundamentals);
            return Some(detail);
        }
    }
    None
}

pub async fn fetch_kr_market_caps(symbols: &[&str]) -> HashMap<String, f64> {
    let client = super::http_client();
    let owned = symbols.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    stream::iter(owned)
        .map(|symbol| {
            let client = client.clone();
            async move {
                fetch_naver_market_cap(&client, &symbol)
                    .await
                    .map(|cap| (symbol, cap))
            }
        })
        .buffer_unordered(10)
        .filter_map(async move |item| item)
        .collect()
        .await
}

pub async fn fetch_kr_fundamentals(symbol: &str) -> HashMap<String, f64> {
    let client = super::http_client();
    let mut result = scrape_naver_fundamentals(&client, symbol).await;
    for yahoo_symbol in yahoo_candidates(symbol) {
        if let Ok(summary) = super::yahoo::fetch_quote_summary(&client, &yahoo_symbol).await {
            if summary.market_cap > 0.0 {
                result.insert("market_cap".to_string(), summary.market_cap);
                break;
            }
        }
    }
    result
}

pub fn merge_fundamentals(detail: &mut StockDetail, data: &HashMap<String, f64>) {
    if let Some(value) = data.get("market_cap") {
        detail.market_cap = *value;
    }
    if let Some(value) = data.get("pe_ratio") {
        detail.pe_ratio = Some(*value);
    }
    if let Some(value) = data.get("eps") {
        detail.eps = Some(*value);
    }
    if let Some(value) = data.get("beta") {
        detail.beta = Some(*value);
    }
}

async fn fetch_kr_quote(client: &reqwest::Client, symbol: &str) -> Option<StockQuote> {
    for yahoo_symbol in yahoo_candidates(symbol) {
        if let Ok(Some(quote)) =
            super::yahoo::fetch_stock_quote(client, symbol, &yahoo_symbol, Market::Kr).await
        {
            return Some(quote);
        }
    }
    fetch_naver_quote(client, symbol).await
}

async fn fetch_naver_quote(client: &reqwest::Client, symbol: &str) -> Option<StockQuote> {
    let html = fetch_naver_html(client, symbol).await.ok()?;
    let price = capture_number(
        &html,
        r#"<p class="no_today">.*?<span class="blind">([\d,]+)</span>"#,
    )?;
    let prev_close = capture_number(
        &html,
        r#"전일</span>.*?<span class="blind">([\d,]+)</span>"#,
    )
    .unwrap_or(price);
    let volume = capture_number(
        &html,
        r#"거래량</span>.*?<span class="blind">([\d,]+)</span>"#,
    )
    .unwrap_or_default() as u64;
    let change = price - prev_close;
    let sector = config::stock_sector(Market::Kr, symbol).to_string();
    let tags = config::instrument_tags(symbol, &sector);
    Some(StockQuote {
        symbol: symbol.to_string(),
        name: {
            let configured = config::stock_name(Market::Kr, symbol);
            if configured.is_empty() {
                symbol.to_string()
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
        volume,
        market: Market::Kr,
        currency: crate::models::Currency::Krw,
        sector,
        tags,
        market_cap: 0.0,
        history_7d: Vec::new(),
        last_updated: Some(chrono::Local::now()),
    })
}

async fn scrape_naver_fundamentals(client: &reqwest::Client, symbol: &str) -> HashMap<String, f64> {
    let mut result = HashMap::new();
    let Ok(html) = fetch_naver_html(client, symbol).await else {
        return result;
    };
    if let Some(value) = capture_number(&html, r#"PER\(배\)</strong></th>\s*<td[^>]*>\s*([\d,.]+)"#)
    {
        result.insert("pe_ratio".to_string(), value);
    }
    if let Some(value) = capture_number(&html, r#"EPS\(원\)</strong></th>\s*<td[^>]*>\s*([\d,.]+)"#)
    {
        result.insert("eps".to_string(), value);
    }
    if let Some(value) = capture_number(&html, r#"PBR\(배\)</strong></th>\s*<td[^>]*>\s*([\d,.]+)"#)
    {
        result.insert("beta".to_string(), value);
    }
    if let Some(value) = parse_market_cap_from_html(&html) {
        result.insert("market_cap".to_string(), value);
    }
    result
}

async fn fetch_naver_market_cap(client: &reqwest::Client, symbol: &str) -> Option<f64> {
    let html = fetch_naver_html(client, symbol).await.ok()?;
    parse_market_cap_from_html(&html)
}

fn parse_market_cap_from_html(html: &str) -> Option<f64> {
    capture_number(html, r#"시가총액\(억\)</span></th>\s*<td>([\d,]+)</td>"#)
        .map(|value| value * 1e8)
}

async fn fetch_naver_html(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<String, reqwest::Error> {
    let url = format!("https://finance.naver.com/item/main.naver?code={symbol}");
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
}

fn capture_number(text: &str, pattern: &str) -> Option<f64> {
    let regex = Regex::new(pattern).ok()?;
    regex
        .captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().replace(',', "").parse::<f64>().ok())
}

fn yahoo_candidates(symbol: &str) -> Vec<String> {
    config::kr_yahoo_candidates(symbol)
        .into_iter()
        .map(|suffix| format!("{symbol}{suffix}"))
        .collect()
}

use chrono::{Datelike, Duration, Local, NaiveDate};
use futures::{stream, StreamExt};

use crate::{
    config,
    models::{Currency, Market, StockDetail, StockQuote},
};

#[derive(Debug, Clone)]
pub struct OhlcvRow {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

#[derive(Debug, Clone)]
pub struct KrQuoteData {
    pub price: f64,
    pub prev_close: f64,
    pub volume: u64,
    pub history_7d: Vec<f64>,
}

pub async fn fetch_quotes(symbols: &[&str]) -> Vec<StockQuote> {
    let client = super::http_client();
    let owned = symbols
        .iter()
        .map(|symbol| (*symbol).to_string())
        .collect::<Vec<_>>();
    stream::iter(owned)
        .map(|symbol| {
            let client = client.clone();
            async move { fetch_quote(&client, &symbol).await }
        })
        .buffer_unordered(10)
        .filter_map(async move |item| item)
        .collect()
        .await
}

pub async fn fetch_quote(client: &reqwest::Client, symbol: &str) -> Option<StockQuote> {
    let data = fetch_quote_data(client, symbol).await?;
    let change = data.price - data.prev_close;
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
        price: data.price,
        change,
        change_pct: if data.prev_close != 0.0 {
            change / data.prev_close * 100.0
        } else {
            0.0
        },
        volume: data.volume,
        market: Market::Kr,
        currency: Currency::Krw,
        sector,
        tags,
        market_cap: 0.0,
        history_7d: data.history_7d,
        last_updated: Some(Local::now()),
    })
}

pub async fn fetch_quote_data(client: &reqwest::Client, symbol: &str) -> Option<KrQuoteData> {
    let rows = fetch_recent_ohlcv_with_client(client, symbol, 14)
        .await
        .ok()?;
    quote_data_from_rows(&rows)
}

pub async fn fetch_stock_detail(symbol: &str) -> Option<StockDetail> {
    let client = super::http_client();
    let rows = fetch_recent_ohlcv_with_client(&client, symbol, 400)
        .await
        .ok()?;
    detail_from_rows(symbol, rows)
}

pub async fn fetch_recent_ohlcv(symbol: &str, calendar_days: i64) -> Vec<OhlcvRow> {
    let client = super::http_client();
    fetch_recent_ohlcv_with_client(&client, symbol, calendar_days)
        .await
        .unwrap_or_default()
}

pub async fn fetch_ohlcv(
    client: &reqwest::Client,
    symbol: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<OhlcvRow>, reqwest::Error> {
    let url = format!(
        "https://api.finance.naver.com/siseJson.naver?symbol={symbol}&requestType=1&startTime={}&endTime={}&timeframe=day",
        start.format("%Y%m%d"),
        end.format("%Y%m%d")
    );
    let text = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(parse_sise_json(&text))
}

async fn fetch_recent_ohlcv_with_client(
    client: &reqwest::Client,
    symbol: &str,
    calendar_days: i64,
) -> Result<Vec<OhlcvRow>, reqwest::Error> {
    let end = recent_trading_day();
    let start = end - Duration::days(calendar_days);
    fetch_ohlcv(client, symbol, start, end).await
}

fn quote_data_from_rows(rows: &[OhlcvRow]) -> Option<KrQuoteData> {
    let latest = rows.last()?;
    let prev = rows.iter().rev().nth(1).unwrap_or(latest);
    Some(KrQuoteData {
        price: latest.close,
        prev_close: prev.close,
        volume: latest.volume,
        history_7d: rows
            .iter()
            .rev()
            .take(7)
            .map(|row| row.close)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
    })
}

fn detail_from_rows(symbol: &str, rows: Vec<OhlcvRow>) -> Option<StockDetail> {
    let latest = rows.last()?;
    let prev = rows.iter().rev().nth(1).unwrap_or(latest);
    let change = latest.close - prev.close;
    let closes = rows.iter().map(|row| row.close).collect::<Vec<_>>();
    let volumes = rows.iter().map(|row| row.volume).collect::<Vec<_>>();
    let dates = rows
        .iter()
        .map(|row| format!("{:02}/{:02}", row.date.month(), row.date.day()))
        .collect::<Vec<_>>();
    let len = closes.len();
    let slice = |n: usize| -> (Vec<f64>, Vec<u64>, Vec<String>) {
        let start = len.saturating_sub(n);
        (
            closes[start..].to_vec(),
            volumes[start..].to_vec(),
            dates[start..].to_vec(),
        )
    };
    let (history_7d, volume_history_7d, history_dates_7d) = slice(7);
    let (history_30d, volume_history_30d, history_dates_30d) = slice(30);
    let (history_90d, volume_history_90d, history_dates_90d) = slice(90);
    let (history_1y, volume_history_1y, history_dates_1y) = slice(250);
    let avg_volume = if rows.is_empty() {
        0
    } else {
        rows.iter()
            .rev()
            .take(10)
            .map(|row| row.volume)
            .sum::<u64>()
            / rows.len().min(10) as u64
    };
    let week52_high = rows
        .iter()
        .map(|row| row.high)
        .reduce(f64::max)
        .unwrap_or(latest.close);
    let week52_low = rows
        .iter()
        .map(|row| row.low)
        .filter(|value| *value > 0.0)
        .reduce(f64::min)
        .unwrap_or(latest.close);
    Some(StockDetail {
        symbol: symbol.to_string(),
        name: config::stock_name(Market::Kr, symbol).to_string(),
        market: Market::Kr,
        currency: Currency::Krw,
        price: latest.close,
        change,
        change_pct: if prev.close != 0.0 {
            change / prev.close * 100.0
        } else {
            0.0
        },
        open_price: latest.open,
        high: latest.high,
        low: latest.low,
        prev_close: prev.close,
        volume: latest.volume,
        avg_volume,
        market_cap: 0.0,
        pe_ratio: None,
        week52_high,
        week52_low,
        history_7d,
        history_30d,
        history_90d,
        history_1y,
        intraday_times: Vec::new(),
        intraday_volumes: Vec::new(),
        intraday_up_bars: Vec::new(),
        volume_history_7d,
        volume_history_30d,
        volume_history_90d,
        volume_history_1y,
        history_dates_7d,
        history_dates_30d,
        history_dates_90d,
        history_dates_1y,
        day_change: change,
        day_change_pct: if prev.close != 0.0 {
            change / prev.close * 100.0
        } else {
            0.0
        },
        eps: None,
        dividend_yield: None,
        beta: None,
        sector: config::stock_sector(Market::Kr, symbol).to_string(),
        last_updated: Some(Local::now()),
    })
}

fn parse_sise_json(text: &str) -> Vec<OhlcvRow> {
    text.lines()
        .filter_map(parse_sise_row)
        .filter(|row| row.close > 0.0)
        .collect()
}

fn parse_sise_row(line: &str) -> Option<OhlcvRow> {
    let cleaned = line
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(',')
        .trim_end_matches(']');
    let fields = cleaned
        .split(',')
        .map(|field| field.trim().trim_matches('\'').trim_matches('"'))
        .collect::<Vec<_>>();
    if fields.len() < 6 || fields[0].len() != 8 || !fields[0].chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(OhlcvRow {
        date: NaiveDate::parse_from_str(fields[0], "%Y%m%d").ok()?,
        open: parse_number(fields[1])?,
        high: parse_number(fields[2])?,
        low: parse_number(fields[3])?,
        close: parse_number(fields[4])?,
        volume: parse_number(fields[5]).unwrap_or_default() as u64,
    })
}

fn parse_number(value: &str) -> Option<f64> {
    let normalized = value.trim().replace(',', "");
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("null") {
        None
    } else {
        normalized.parse::<f64>().ok()
    }
}

fn recent_trading_day() -> NaiveDate {
    let today = Local::now().date_naive();
    for offset in 0..7 {
        let day = today - Duration::days(offset);
        if day.weekday().num_days_from_monday() < 5 {
            return day;
        }
    }
    today
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_naver_sise_rows() {
        let text = "[['날짜', '시가', '고가', '저가', '종가', '거래량'],\n['20260511', 1000, 1200, 900, 1100, 12345],\n['20260512', 1100, 1300, 1000, 1250, 22222]]";
        let rows = parse_sise_json(text);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].close, 1100.0);
        assert_eq!(rows[1].volume, 22222);
    }
}

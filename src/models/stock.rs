use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Market {
    Us,
    Kr,
}

impl Market {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Us => "US",
            Self::Kr => "KR",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            Self::Us => "NYSE/NASDAQ",
            Self::Kr => "KRX",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    Usd,
    Krw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    GoldenCross,
    DeathCross,
    BullTrend,
    BearTrend,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalInfo {
    pub kind: SignalKind,
    pub age_days: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowKind {
    Buying,
    Selling,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowInfo {
    pub kind: FlowKind,
    pub strength: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuote {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: u64,
    pub market: Market,
    pub currency: Currency,
    pub sector: String,
    pub tags: Vec<String>,
    pub market_cap: f64,
    pub history_7d: Vec<f64>,
    pub last_updated: Option<DateTime<Local>>,
}

impl StockQuote {
    pub fn arrow(&self) -> &'static str {
        if self.change > 0.0 {
            "▲"
        } else if self.change < 0.0 {
            "▼"
        } else {
            "-"
        }
    }

    pub fn formatted_price(&self) -> String {
        match self.currency {
            Currency::Krw => format_number(self.price, 0),
            Currency::Usd => format_number(self.price, 2),
        }
    }

    pub fn formatted_change(&self) -> String {
        let sign = if self.change >= 0.0 { "+" } else { "" };
        match self.currency {
            Currency::Krw => format!("{}{}", sign, format_number(self.change, 0)),
            Currency::Usd => format!("{}{}", sign, format_number(self.change, 2)),
        }
    }

    pub fn formatted_change_pct(&self) -> String {
        let sign = if self.change_pct >= 0.0 { "+" } else { "" };
        format!("{}{:.2}%", sign, self.change_pct)
    }

    pub fn signal_info(&self) -> SignalInfo {
        if self.history_7d.len() < 5 {
            return SignalInfo {
                kind: SignalKind::Neutral,
                age_days: 99,
            };
        }
        let len = self.history_7d.len();
        let current_ma3 = moving_average_at(&self.history_7d, 3, len - 1).unwrap_or(self.price);
        let current_ma5 = moving_average_at(&self.history_7d, 5, len - 1).unwrap_or(self.price);
        let current_kind = if current_ma3 > current_ma5 {
            SignalKind::BullTrend
        } else if current_ma3 < current_ma5 {
            SignalKind::BearTrend
        } else {
            SignalKind::Neutral
        };

        let mut age_days = 99;
        let mut kind = current_kind;
        for idx in (4..len).rev() {
            let ma3 = moving_average_at(&self.history_7d, 3, idx).unwrap_or(self.history_7d[idx]);
            let ma5 = moving_average_at(&self.history_7d, 5, idx).unwrap_or(self.history_7d[idx]);
            let prev_ma3 =
                moving_average_at(&self.history_7d, 3, idx - 1).unwrap_or(self.history_7d[idx - 1]);
            let prev_ma5 =
                moving_average_at(&self.history_7d, 5, idx - 1).unwrap_or(self.history_7d[idx - 1]);
            if prev_ma3 <= prev_ma5 && ma3 > ma5 {
                kind = SignalKind::GoldenCross;
                age_days = len - 1 - idx;
                break;
            }
            if prev_ma3 >= prev_ma5 && ma3 < ma5 {
                kind = SignalKind::DeathCross;
                age_days = len - 1 - idx;
                break;
            }
        }
        SignalInfo { kind, age_days }
    }

    pub fn technical_signal_label(&self) -> String {
        let info = self.signal_info();
        match info.kind {
            SignalKind::GoldenCross => {
                if info.age_days <= 1 {
                    "GC!".to_string()
                } else if info.age_days <= 3 {
                    format!("GC{}", info.age_days)
                } else {
                    "GC~".to_string()
                }
            }
            SignalKind::DeathCross => {
                if info.age_days <= 1 {
                    "DC!".to_string()
                } else if info.age_days <= 3 {
                    format!("DC{}", info.age_days)
                } else {
                    "DC~".to_string()
                }
            }
            SignalKind::BullTrend => "UP".to_string(),
            SignalKind::BearTrend => "DN".to_string(),
            SignalKind::Neutral => "—".to_string(),
        }
    }

    pub fn flow_info(&self) -> FlowInfo {
        let avg_volume = if self.history_7d.len() > 1 {
            self.volume.max(1) as f64 / self.history_7d.len() as f64
        } else {
            self.volume.max(1) as f64
        };
        let ratio = self.volume as f64 / avg_volume.max(1.0);
        let strength = if ratio >= 3.0 {
            3
        } else if ratio >= 1.8 {
            2
        } else if ratio >= 1.1 {
            1
        } else {
            0
        };
        let kind = if self.change_pct > 0.6 {
            FlowKind::Buying
        } else if self.change_pct < -0.6 {
            FlowKind::Selling
        } else {
            FlowKind::Quiet
        };
        FlowInfo { kind, strength }
    }

    pub fn flow_label(&self) -> String {
        let info = self.flow_info();
        match info.kind {
            FlowKind::Buying => format!("B{}", info.strength.max(1)),
            FlowKind::Selling => format!("S{}", info.strength.max(1)),
            FlowKind::Quiet => "Q0".to_string(),
        }
    }

    pub fn formatted_market_cap(&self) -> String {
        format_market_cap(self.market_cap, self.currency)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketIndex {
    pub symbol: String,
    pub name: String,
    pub value: f64,
    pub change: f64,
    pub change_pct: f64,
    pub last_updated: Option<DateTime<Local>>,
}

impl MarketIndex {
    pub fn formatted_value(&self) -> String {
        format_number(self.value, 2)
    }

    pub fn formatted_change_pct(&self) -> String {
        let sign = if self.change_pct >= 0.0 { "+" } else { "" };
        format!("{}{:.2}%", sign, self.change_pct)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicIndicator {
    pub symbol: String,
    pub name: String,
    pub value: f64,
    pub change: f64,
    pub change_pct: f64,
    pub unit: String,
    pub last_updated: Option<DateTime<Local>>,
}

impl EconomicIndicator {
    pub fn formatted_value(&self) -> String {
        match self.unit.as_str() {
            "%" => format!("{:.2}%", self.value),
            "$" => format!("${}", format_number(self.value, 2)),
            "W" => format_number(self.value, 0),
            _ => format_number(self.value, 2),
        }
    }

    pub fn formatted_change_pct(&self) -> String {
        let sign = if self.change_pct >= 0.0 { "+" } else { "" };
        format!("{}{:.2}%", sign, self.change_pct)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRate {
    pub symbol: String,
    pub pair: String,
    pub quote_currency: String,
    pub base_currency: String,
    pub value: f64,
    pub change: f64,
    pub change_pct: f64,
    pub history_7d: Vec<f64>,
    pub history_30d: Vec<f64>,
    pub history_90d: Vec<f64>,
    pub history_1y: Vec<f64>,
    pub last_updated: Option<DateTime<Local>>,
}

impl FxRate {
    pub fn formatted_value(&self) -> String {
        if self.base_currency == "JPY" {
            format!(
                "{}{} / 100{}",
                self.quote_currency_symbol(),
                format_number(self.value, 0),
                self.base_currency
            )
        } else {
            format!(
                "{}{}",
                self.quote_currency_symbol(),
                format_number(self.value, 2)
            )
        }
    }

    pub fn formatted_change_pct(&self) -> String {
        let sign = if self.change_pct >= 0.0 { "+" } else { "" };
        format!("{}{:.2}%", sign, self.change_pct)
    }

    pub fn quote_currency_symbol(&self) -> &'static str {
        match self.quote_currency.as_str() {
            "KRW" => "₩",
            "USD" => "$",
            "EUR" => "€",
            "JPY" => "¥",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockDetail {
    pub symbol: String,
    pub name: String,
    pub market: Market,
    pub currency: Currency,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub open_price: f64,
    pub high: f64,
    pub low: f64,
    pub prev_close: f64,
    pub volume: u64,
    pub avg_volume: u64,
    pub market_cap: f64,
    pub pe_ratio: Option<f64>,
    pub week52_high: f64,
    pub week52_low: f64,
    pub history_7d: Vec<f64>,
    pub history_30d: Vec<f64>,
    pub history_90d: Vec<f64>,
    pub history_1y: Vec<f64>,
    pub intraday_times: Vec<String>,
    pub intraday_volumes: Vec<u64>,
    pub intraday_up_bars: Vec<bool>,
    pub volume_history_7d: Vec<u64>,
    pub volume_history_30d: Vec<u64>,
    pub volume_history_90d: Vec<u64>,
    pub volume_history_1y: Vec<u64>,
    pub history_dates_7d: Vec<String>,
    pub history_dates_30d: Vec<String>,
    pub history_dates_90d: Vec<String>,
    pub history_dates_1y: Vec<String>,
    pub day_change: f64,
    pub day_change_pct: f64,
    pub eps: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub beta: Option<f64>,
    pub sector: String,
    pub last_updated: Option<DateTime<Local>>,
}

impl StockDetail {
    pub fn formatted_market_cap(&self) -> String {
        format_market_cap(self.market_cap, self.currency)
    }

    pub fn week52_position(&self) -> f64 {
        if (self.week52_high - self.week52_low).abs() < f64::EPSILON {
            0.5
        } else {
            (self.price - self.week52_low) / (self.week52_high - self.week52_low)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartPeriod {
    OneWeek,
    OneMonth,
    ThreeMonths,
    OneYear,
}

impl ChartPeriod {
    pub const ALL: [Self; 4] = [
        Self::OneWeek,
        Self::OneMonth,
        Self::ThreeMonths,
        Self::OneYear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OneWeek => "1W",
            Self::OneMonth => "1M",
            Self::ThreeMonths => "3M",
            Self::OneYear => "1Y",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

fn moving_average_at(values: &[f64], window: usize, end_index: usize) -> Option<f64> {
    if end_index + 1 < window {
        None
    } else {
        let start = end_index + 1 - window;
        Some(values[start..=end_index].iter().sum::<f64>() / window as f64)
    }
}

pub fn format_volume(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.2}B", value as f64 / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.0}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

pub fn format_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return "—".to_string();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let abs = value.abs();
    let raw = format!("{:.*}", decimals, abs);
    let (int_part, frac_part) = raw.split_once('.').unwrap_or((raw.as_str(), ""));
    let mut chars: Vec<char> = int_part.chars().rev().collect();
    let mut grouped = String::new();
    for (idx, ch) in chars.drain(..).enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let int_grouped: String = grouped.chars().rev().collect();
    if decimals == 0 {
        format!("{}{}", sign, int_grouped)
    } else {
        format!("{}{}.{}", sign, int_grouped, frac_part)
    }
}

pub fn format_market_cap(value: f64, currency: Currency) -> String {
    if value <= 0.0 || !value.is_finite() {
        return "—".to_string();
    }
    match currency {
        Currency::Krw => {
            if value >= 1e16 {
                format!("{:.0}경", value / 1e16)
            } else if value >= 1e12 {
                format!("{:.0}조", value / 1e12)
            } else if value >= 1e8 {
                format!("{:.0}억", value / 1e8)
            } else {
                format_number(value, 0)
            }
        }
        Currency::Usd => {
            if value >= 1e12 {
                format!("${:.2}T", value / 1e12)
            } else if value >= 1e9 {
                format!("${:.1}B", value / 1e9)
            } else if value >= 1e6 {
                format!("${:.0}M", value / 1e6)
            } else {
                format!("${}", format_number(value, 0))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_with_commas() {
        assert_eq!(format_number(1234567.891, 2), "1,234,567.89");
        assert_eq!(format_number(-1234567.0, 0), "-1,234,567");
    }

    #[test]
    fn formats_market_caps() {
        assert_eq!(
            format_market_cap(2_500_000_000_000.0, Currency::Usd),
            "$2.50T"
        );
        assert_eq!(format_market_cap(1_200_000_000_000.0, Currency::Krw), "1조");
    }
}

use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::models::{HoldingInfo, InstrumentKind, InstrumentProfile, Market};

pub const REFRESH_INTERVAL_SECS: u64 = 45;
pub const NEWS_REFRESH_INTERVAL_SECS: u64 = 120;

pub const US_INDICES: &[(&str, &str)] =
    &[("^GSPC", "S&P 500"), ("^IXIC", "NASDAQ"), ("^DJI", "DOW")];

pub const KR_INDICES: &[(&str, &str)] = &[("^KS11", "KOSPI"), ("^KQ11", "KOSDAQ")];

pub const US_STOCKS: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "NVDA", "META", "TSLA", "BRK-B", "JPM", "V", "JNJ", "UNH",
    "WMT", "MA", "PG", "HD", "XOM", "CVX", "LLY", "ABBV", "PFE", "KO", "PEP", "MRK", "COST",
    "AVGO", "AMD", "ORCL", "CRM", "NFLX", "ADBE", "CSCO", "ACN", "TXN", "INTC", "QCOM", "INTU",
    "AMAT", "BKNG", "ISRG", "MDLZ", "ADP", "REGN", "VRTX", "GILD", "PANW", "LRCX", "MU", "KLAC",
    "SNPS",
];

pub const US_ETFS: &[&str] = &[
    "SPY", "QQQ", "VTI", "VOO", "IWM", "DIA", "XLK", "XLF", "XLE", "XLI", "XLV", "XLP", "XLY",
    "XLU", "XLB", "XLRE", "SMH", "SOXX", "ARKK", "KWEB",
];

pub const US_BONDS: &[&str] = &[
    "TLT", "IEF", "SHY", "BND", "AGG", "LQD", "HYG", "TIP", "MBB", "BIL", "SGOV", "VGIT", "GOVT",
    "EDV", "JNK",
];

pub const KR_STOCKS: &[&str] = &[
    "005930", "000660", "373220", "005380", "000270", "207940", "006400", "035420", "035720",
    "005490", "068270", "028260", "105560", "055550", "012330", "066570", "003670", "051910",
    "096770", "034730", "000810", "003550", "032830", "009150", "086790", "010130", "033780",
    "011200", "247540", "377300", "030200", "017670", "018260", "036570", "316140", "003490",
    "034020", "011170", "024110", "010950", "006800", "004020", "000720", "002790", "138040",
    "259960", "326030", "323410", "361610", "352820",
];

pub const INDICATORS: &[(&str, &str, &str)] = &[
    ("CL=F", "WTI Oil", "$"),
    ("GC=F", "Gold", "$"),
    ("SI=F", "Silver", "$"),
    ("HG=F", "Copper", "$"),
    ("EURUSD=X", "EUR/USD", ""),
    ("KRW=X", "USD/KRW", "W"),
    ("JPY=X", "USD/JPY", ""),
    ("CNY=X", "USD/CNY", ""),
    ("^TNX", "US 10Y", "%"),
    ("BTC-USD", "Bitcoin", "$"),
    ("ETH-USD", "Ethereum", "$"),
];

pub const NEWS_FEEDS: &[(&str, &str)] = &[
    ("yahoo", "https://finance.yahoo.com/news/rssindex"),
    (
        "yahoo_markets",
        "https://feeds.finance.yahoo.com/rss/2.0/headline?s=^GSPC&region=US&lang=en-US",
    ),
    ("hankyung", "https://www.hankyung.com/feed/economy"),
    ("mk", "https://www.mk.co.kr/rss/30100041/"),
];

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomWatchlists {
    pub us: Vec<String>,
    pub kr: Vec<String>,
    pub etf: Vec<String>,
    pub bond: Vec<String>,
}

pub fn load_custom_watchlists() -> CustomWatchlists {
    let path = watchlist_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<CustomWatchlists>(&content).ok())
        .unwrap_or_default()
}

pub fn save_custom_watchlists(watchlists: &CustomWatchlists) -> anyhow::Result<()> {
    let path = watchlist_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(watchlists)?)?;
    Ok(())
}

pub fn merged_us_symbols(custom: &CustomWatchlists) -> Vec<String> {
    merge_symbols(US_STOCKS, &custom.us)
}

pub fn merged_kr_symbols(custom: &CustomWatchlists) -> Vec<String> {
    merge_symbols(KR_STOCKS, &custom.kr)
}

pub fn merged_etf_symbols(custom: &CustomWatchlists) -> Vec<String> {
    merge_symbols(US_ETFS, &custom.etf)
}

pub fn merged_bond_symbols(custom: &CustomWatchlists) -> Vec<String> {
    merge_symbols(US_BONDS, &custom.bond)
}

fn merge_symbols(defaults: &[&str], custom: &[String]) -> Vec<String> {
    let mut seen = BTreeMap::new();
    for symbol in defaults
        .iter()
        .map(|symbol| (*symbol).to_string())
        .chain(custom.iter().cloned())
    {
        seen.insert(symbol.clone(), symbol);
    }
    seen.into_values().collect()
}

fn watchlist_path() -> PathBuf {
    PathBuf::from(".tickboard-watchlists.json")
}

pub fn generic_instrument_profile(symbol: &str, market: Market) -> InstrumentProfile {
    let sector = stock_sector(market, symbol);
    let category = if sector.is_empty() {
        "Custom Watchlist"
    } else {
        sector
    };
    InstrumentProfile {
        symbol: symbol.to_string(),
        display_name: if stock_name(market, symbol).is_empty() {
            symbol.to_string()
        } else {
            stock_name(market, symbol).to_string()
        },
        kind: InstrumentKind::Stock,
        issuer: "Unknown / Custom".to_string(),
        category: category.to_string(),
        objective: "User-added symbol. Static metadata is limited until a dedicated profile is defined.".to_string(),
        benchmark: None,
        expense_ratio_pct: None,
        distribution_yield_pct: None,
        duration_years: None,
        top_holdings: Vec::new(),
        notes: vec![
            "사용자가 직접 추가한 종목/상품입니다.".to_string(),
            "현재는 기본 정보만 표시하며, 미리 정의된 ETF/채권 프로필처럼 상세 holdings/benchmark 정보는 없습니다.".to_string(),
        ],
    }
}

pub fn stock_name(market: Market, symbol: &str) -> &'static str {
    match market {
        Market::Us => us_stock_name(symbol),
        Market::Kr => kr_stock_name(symbol),
    }
}

pub fn stock_sector(market: Market, symbol: &str) -> &'static str {
    match market {
        Market::Us => us_stock_sector(symbol),
        Market::Kr => kr_stock_sector(symbol),
    }
}

pub fn us_stock_name(symbol: &str) -> &'static str {
    match symbol {
        "AAPL" => "Apple",
        "MSFT" => "Microsoft",
        "GOOGL" => "Alphabet",
        "AMZN" => "Amazon",
        "NVDA" => "Nvidia",
        "META" => "Meta Platforms",
        "TSLA" => "Tesla",
        "BRK-B" => "Berkshire Hathaway",
        "JPM" => "JPMorgan Chase",
        "V" => "Visa",
        "JNJ" => "Johnson & Johnson",
        "UNH" => "UnitedHealth",
        "WMT" => "Walmart",
        "MA" => "Mastercard",
        "PG" => "Procter & Gamble",
        "HD" => "Home Depot",
        "XOM" => "Exxon Mobil",
        "CVX" => "Chevron",
        "LLY" => "Eli Lilly",
        "ABBV" => "AbbVie",
        "PFE" => "Pfizer",
        "KO" => "Coca-Cola",
        "PEP" => "PepsiCo",
        "MRK" => "Merck",
        "COST" => "Costco",
        "AVGO" => "Broadcom",
        "AMD" => "AMD",
        "ORCL" => "Oracle",
        "CRM" => "Salesforce",
        "NFLX" => "Netflix",
        "ADBE" => "Adobe",
        "CSCO" => "Cisco",
        "ACN" => "Accenture",
        "TXN" => "Texas Instruments",
        "INTC" => "Intel",
        "QCOM" => "Qualcomm",
        "INTU" => "Intuit",
        "AMAT" => "Applied Materials",
        "BKNG" => "Booking Holdings",
        "ISRG" => "Intuitive Surgical",
        "MDLZ" => "Mondelez",
        "ADP" => "ADP",
        "REGN" => "Regeneron",
        "VRTX" => "Vertex Pharma",
        "GILD" => "Gilead Sciences",
        "PANW" => "Palo Alto Networks",
        "LRCX" => "Lam Research",
        "MU" => "Micron",
        "KLAC" => "KLA Corp",
        "SPY" => "SPDR S&P 500 ETF",
        "QQQ" => "Invesco QQQ Trust",
        "VTI" => "Vanguard Total Stock Market ETF",
        "VOO" => "Vanguard S&P 500 ETF",
        "IWM" => "iShares Russell 2000 ETF",
        "DIA" => "SPDR Dow Jones Industrial Average ETF",
        "XLK" => "Technology Select Sector SPDR Fund",
        "XLF" => "Financial Select Sector SPDR Fund",
        "XLE" => "Energy Select Sector SPDR Fund",
        "XLI" => "Industrial Select Sector SPDR Fund",
        "XLV" => "Health Care Select Sector SPDR Fund",
        "XLP" => "Consumer Staples Select Sector SPDR Fund",
        "XLY" => "Consumer Discretionary Select Sector SPDR Fund",
        "XLU" => "Utilities Select Sector SPDR Fund",
        "XLB" => "Materials Select Sector SPDR Fund",
        "XLRE" => "Real Estate Select Sector SPDR Fund",
        "SMH" => "VanEck Semiconductor ETF",
        "SOXX" => "iShares Semiconductor ETF",
        "ARKK" => "ARK Innovation ETF",
        "KWEB" => "KraneShares CSI China Internet ETF",
        "TLT" => "iShares 20+ Year Treasury Bond ETF",
        "IEF" => "iShares 7-10 Year Treasury Bond ETF",
        "SHY" => "iShares 1-3 Year Treasury Bond ETF",
        "BND" => "Vanguard Total Bond Market ETF",
        "AGG" => "iShares Core U.S. Aggregate Bond ETF",
        "LQD" => "iShares iBoxx $ Investment Grade Corporate Bond ETF",
        "HYG" => "iShares iBoxx $ High Yield Corporate Bond ETF",
        "TIP" => "iShares TIPS Bond ETF",
        "MBB" => "iShares MBS ETF",
        "BIL" => "SPDR Bloomberg 1-3 Month T-Bill ETF",
        "SGOV" => "iShares 0-3 Month Treasury Bond ETF",
        "VGIT" => "Vanguard Intermediate-Term Treasury ETF",
        "GOVT" => "iShares U.S. Treasury Bond ETF",
        "EDV" => "Vanguard Extended Duration Treasury ETF",
        "JNK" => "SPDR Bloomberg High Yield Bond ETF",
        _ => "",
    }
}

pub fn us_stock_sector(symbol: &str) -> &'static str {
    match symbol {
        "AAPL" | "MSFT" | "GOOGL" | "NVDA" | "META" | "AVGO" | "AMD" | "ORCL" | "CRM" | "ADBE"
        | "CSCO" | "ACN" | "TXN" | "INTC" | "QCOM" | "INTU" | "AMAT" | "ADP" | "PANW" | "LRCX"
        | "MU" | "KLAC" | "SNPS" => "Technology",
        "AMZN" | "TSLA" | "WMT" | "PG" | "HD" | "KO" | "PEP" | "COST" | "BKNG" | "MDLZ" => {
            "Consumer"
        }
        "BRK-B" | "JPM" | "V" | "MA" => "Financial",
        "JNJ" | "UNH" | "LLY" | "ABBV" | "PFE" | "MRK" | "ISRG" | "REGN" | "VRTX" | "GILD" => {
            "Healthcare"
        }
        "XOM" | "CVX" => "Energy",
        "SPY" | "VOO" | "VTI" | "DIA" | "IWM" | "QQQ" => "ETF",
        "XLK" | "SMH" | "SOXX" => "Technology ETF",
        "XLF" => "Financial ETF",
        "XLE" => "Energy ETF",
        "XLI" => "Industrial ETF",
        "XLV" => "Healthcare ETF",
        "XLP" => "Consumer Staples ETF",
        "XLY" => "Consumer Discretionary ETF",
        "XLU" => "Utilities ETF",
        "XLB" => "Materials ETF",
        "XLRE" => "Real Estate ETF",
        "ARKK" => "Growth ETF",
        "KWEB" => "China Internet ETF",
        "TLT" | "IEF" | "SHY" | "TIP" | "BIL" | "SGOV" | "VGIT" | "GOVT" | "EDV" => "Treasury Bond",
        "BND" | "AGG" => "Aggregate Bond",
        "LQD" => "Investment Grade Bond",
        "HYG" | "JNK" => "High Yield Bond",
        "MBB" => "Mortgage Bond",
        _ => "",
    }
}

pub fn kr_stock_name(symbol: &str) -> &'static str {
    match symbol {
        "005930" => "Samsung Electronics",
        "000660" => "SK Hynix",
        "373220" => "LG Energy Solution",
        "005380" => "Hyundai Motor",
        "000270" => "Kia",
        "207940" => "Samsung Biologics",
        "006400" => "Samsung SDI",
        "035420" => "NAVER",
        "035720" => "Kakao",
        "005490" => "POSCO Holdings",
        "068270" => "Celltrion",
        "028260" => "Samsung C&T",
        "105560" => "KB Financial",
        "055550" => "Shinhan Financial",
        "012330" => "Hyundai Mobis",
        "066570" => "LG Electronics",
        "003670" => "POSCO Future M",
        "051910" => "LG Chem",
        "096770" => "SK Innovation",
        "034730" => "SK",
        "000810" => "Samsung Fire",
        "003550" => "LG",
        "032830" => "Samsung Life",
        "009150" => "Samsung Electro",
        "086790" => "Hana Financial",
        "010130" => "Korea Zinc",
        "033780" => "KT&G",
        "011200" => "HMM",
        "247540" => "Ecopro BM",
        "377300" => "Kakao Pay",
        "030200" => "KT",
        "017670" => "SK Telecom",
        "018260" => "Samsung SDS",
        "036570" => "NCsoft",
        "316140" => "Woori Financial",
        "003490" => "Korea Shipbuilding",
        "034020" => "Doosan Enerbility",
        "011170" => "Lotte Chemical",
        "024110" => "Industrial Bank of Korea",
        "010950" => "S-Oil",
        "006800" => "Mirae Asset Securities",
        "004020" => "Hyundai Steel",
        "000720" => "Hyundai E&C",
        "002790" => "Amore Pacific",
        "138040" => "Meritz Financial",
        "259960" => "Krafton",
        "326030" => "SK Biopharm",
        "323410" => "Kakao Bank",
        "361610" => "SK IE Technology",
        "352820" => "Hive",
        _ => "",
    }
}

pub fn kr_stock_sector(symbol: &str) -> &'static str {
    match symbol {
        "005930" | "000660" => "Semiconductor",
        "373220" | "006400" | "247540" | "361610" => "Battery",
        "005380" | "000270" => "Auto",
        "207940" | "068270" | "326030" => "Bio",
        "035420" | "035720" => "Internet",
        "005490" | "004020" => "Steel",
        "028260" | "034730" | "003550" => "Holding",
        "105560" | "055550" | "086790" | "316140" | "024110" | "138040" => "Financial",
        "012330" => "Auto Parts",
        "066570" => "Electronics",
        "003670" => "Materials",
        "051910" | "011170" => "Chemical",
        "096770" | "010950" => "Energy",
        "000810" | "032830" => "Insurance",
        "009150" => "Components",
        "010130" => "Non-Ferrous",
        "033780" => "Tobacco",
        "011200" => "Shipping",
        "377300" | "323410" => "Fintech",
        "030200" | "017670" => "Telecom",
        "018260" => "IT Services",
        "036570" | "259960" => "Gaming",
        "003490" => "Shipbuilding",
        "034020" => "Industrial",
        "006800" => "Securities",
        "000720" => "Construction",
        "002790" => "Cosmetics",
        "352820" => "Entertainment",
        _ => "",
    }
}

pub fn instrument_tags(symbol: &str, sector: &str) -> Vec<String> {
    let mut tags = vec![sector.to_lowercase().replace(' ', "-")];
    match symbol {
        "NVDA" | "AMD" | "AVGO" | "SMH" | "SOXX" | "QQQ" | "SNPS" | "AMAT" | "KLAC" | "LRCX"
        | "MU" => {
            tags.extend(["ai", "semiconductor"].into_iter().map(str::to_string));
        }
        "MSFT" | "GOOGL" | "AMZN" | "META" | "ARKK" => {
            tags.extend(["ai", "cloud", "platform"].into_iter().map(str::to_string));
        }
        "XLE" | "CVX" | "XOM" | "010950" | "096770" => {
            tags.extend(["energy", "resource"].into_iter().map(str::to_string));
        }
        "XLI" | "005380" | "000270" | "012330" | "003490" => {
            tags.extend(["industrial", "mobility"].into_iter().map(str::to_string));
        }
        "TLT" | "IEF" | "SHY" | "VGIT" | "GOVT" | "EDV" | "BIL" | "SGOV" => {
            tags.extend(["bond", "treasury", "rate"].into_iter().map(str::to_string));
        }
        "BND" | "AGG" => {
            tags.extend(
                ["bond", "aggregate", "rate"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        "HYG" | "JNK" => {
            tags.extend(
                ["bond", "credit", "high-yield"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        "TIP" => {
            tags.extend(
                ["bond", "inflation", "tips"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        "DBA" => {
            tags.extend(
                ["food", "agriculture", "commodity"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        "CORN" | "WEAT" | "SOYB" => {
            tags.extend(
                ["food", "grain", "agriculture"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        "005930" | "000660" | "373220" | "006400" | "247540" | "361610" => {
            tags.extend(
                ["korea", "ai", "electronics"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        "035420" | "035720" | "323410" | "377300" => {
            tags.extend(
                ["korea", "platform", "internet"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        _ => {}
    }
    if sector.eq_ignore_ascii_case("Utilities ETF") {
        tags.extend(["power", "defensive"].into_iter().map(str::to_string));
    }
    if sector.eq_ignore_ascii_case("Energy") || sector.eq_ignore_ascii_case("Energy ETF") {
        tags.push("power".to_string());
    }
    tags.retain(|tag| !tag.trim().is_empty());
    tags.sort();
    tags.dedup();
    tags
}

pub fn instrument_profile(symbol: &str) -> Option<InstrumentProfile> {
    match symbol {
        "SPY" => Some(etf_profile(
            symbol,
            "SPDR S&P 500 ETF",
            "State Street",
            "Large Cap Blend ETF",
            "Tracks the S&P 500 Index.",
            Some("S&P 500 Index"),
            Some(0.09),
            Some(1.3),
            None,
            vec![
                holding("Microsoft", 7.0),
                holding("Apple", 6.0),
                holding("NVIDIA", 5.5),
                holding("Amazon", 3.8),
                holding("Meta Platforms", 2.7),
            ],
            vec![
                "가장 대표적인 미국 대형주 지수 ETF입니다.",
                "시장 전체 방향성을 보기 위한 코어 포지션 성격이 강합니다.",
            ],
        )),
        "QQQ" => Some(etf_profile(
            symbol,
            "Invesco QQQ Trust",
            "Invesco",
            "Large Cap Growth ETF",
            "Tracks the Nasdaq-100 Index with tech-heavy growth exposure.",
            Some("Nasdaq-100 Index"),
            Some(0.20),
            Some(0.6),
            None,
            vec![
                holding("Microsoft", 8.5),
                holding("Apple", 7.5),
                holding("NVIDIA", 7.0),
                holding("Amazon", 5.5),
                holding("Broadcom", 4.5),
            ],
            vec![
                "기술주와 대형 성장주 비중이 높아 변동성이 SPY보다 큽니다.",
                "AI/반도체 흐름을 볼 때 선행지표처럼 참고하기 좋습니다.",
            ],
        )),
        "SMH" => Some(etf_profile(
            symbol,
            "VanEck Semiconductor ETF",
            "VanEck",
            "Semiconductor ETF",
            "Provides concentrated exposure to semiconductor designers, foundries, and equipment makers.",
            Some("MVIS US Listed Semiconductor 25 Index"),
            Some(0.35),
            Some(0.5),
            None,
            vec![
                holding("NVIDIA", 20.0),
                holding("TSMC", 12.0),
                holding("Broadcom", 7.0),
                holding("AMD", 6.0),
                holding("ASML", 5.5),
            ],
            vec![
                "반도체 공급망 전체를 보되 일부 종목 집중도가 높은 편입니다.",
                "장비·설계·파운드리 사이클 해석에 유용합니다.",
            ],
        )),
        "SOXX" => Some(etf_profile(
            symbol,
            "iShares Semiconductor ETF",
            "iShares",
            "Semiconductor ETF",
            "Tracks a broad semiconductor index with designers and equipment names.",
            Some("ICE Semiconductor Index"),
            Some(0.35),
            Some(0.7),
            None,
            vec![
                holding("NVIDIA", 9.0),
                holding("Broadcom", 8.0),
                holding("AMD", 7.0),
                holding("Qualcomm", 6.0),
                holding("Texas Instruments", 6.0),
            ],
            vec![
                "SMH보다 분산도가 높은 편이라 반도체 업종 전반 흐름 보기 좋습니다.",
            ],
        )),
        "ARKK" => Some(etf_profile(
            symbol,
            "ARK Innovation ETF",
            "ARK Invest",
            "Disruptive Innovation ETF",
            "Actively managed portfolio focused on disruptive innovation themes.",
            None,
            Some(0.75),
            None,
            None,
            vec![
                holding("Tesla", 10.0),
                holding("Roku", 7.0),
                holding("Coinbase", 6.5),
                holding("Roblox", 6.0),
                holding("CRISPR Therapeutics", 5.0),
            ],
            vec![
                "테마형·액티브 ETF라 종목 교체와 변동성이 큽니다.",
            ],
        )),
        "KWEB" => Some(etf_profile(
            symbol,
            "KraneShares CSI China Internet ETF",
            "KraneShares",
            "China Internet ETF",
            "Tracks large Chinese internet and e-commerce companies.",
            Some("CSI Overseas China Internet Index"),
            Some(0.69),
            None,
            None,
            vec![
                holding("Tencent", 10.0),
                holding("Alibaba", 9.5),
                holding("Meituan", 8.0),
                holding("PDD Holdings", 7.0),
                holding("JD.com", 5.0),
            ],
            vec![
                "중국 플랫폼 규제/소비/매크로 영향을 크게 받는 ETF입니다.",
            ],
        )),
        "TLT" => Some(bond_profile(
            symbol,
            "iShares 20+ Year Treasury Bond ETF",
            "iShares",
            "Long Treasury ETF",
            "Holds long-duration U.S. Treasury bonds.",
            Some("ICE U.S. Treasury 20+ Year Bond Index"),
            Some(0.15),
            Some(4.0),
            Some(16.5),
            vec![
                holding("U.S. Treasury 20Y+", 98.0),
                holding("Cash", 2.0),
            ],
            vec![
                "금리 하락 수혜가 큰 장기채 성격입니다.",
                "duration이 길어 금리 민감도가 매우 큽니다.",
            ],
        )),
        "IEF" => Some(bond_profile(
            symbol,
            "iShares 7-10 Year Treasury Bond ETF",
            "iShares",
            "Intermediate Treasury ETF",
            "Holds intermediate-duration U.S. Treasury bonds.",
            Some("ICE U.S. Treasury 7-10 Year Bond Index"),
            Some(0.15),
            Some(3.8),
            Some(7.2),
            vec![
                holding("U.S. Treasury 7-10Y", 97.0),
                holding("Cash", 3.0),
            ],
            vec![
                "TLT보다 duration이 짧아 금리 변동성이 덜합니다.",
            ],
        )),
        "BND" => Some(bond_profile(
            symbol,
            "Vanguard Total Bond Market ETF",
            "Vanguard",
            "Broad Bond Market ETF",
            "Broad exposure to U.S. investment-grade taxable bonds.",
            Some("Bloomberg U.S. Aggregate Float Adjusted Index"),
            Some(0.03),
            Some(4.2),
            Some(6.2),
            vec![
                holding("U.S. Treasuries", 45.0),
                holding("Agency MBS", 25.0),
                holding("Investment Grade Credit", 20.0),
                holding("ABS/CMBS", 10.0),
            ],
            vec![
                "미국 투자적격 채권 전반을 보는 코어 bond ETF입니다.",
            ],
        )),
        "AGG" => Some(bond_profile(
            symbol,
            "iShares Core U.S. Aggregate Bond ETF",
            "iShares",
            "Aggregate Bond ETF",
            "Tracks the broad U.S. investment-grade bond market.",
            Some("Bloomberg U.S. Aggregate Bond Index"),
            Some(0.03),
            Some(4.0),
            Some(6.0),
            vec![
                holding("U.S. Treasuries", 43.0),
                holding("Agency MBS", 27.0),
                holding("Investment Grade Credit", 19.0),
                holding("Securitized", 11.0),
            ],
            vec![
                "BND와 비슷한 broad bond benchmark 역할을 합니다.",
            ],
        )),
        "HYG" => Some(bond_profile(
            symbol,
            "iShares iBoxx $ High Yield Corporate Bond ETF",
            "iShares",
            "High Yield Bond ETF",
            "Tracks U.S. dollar-denominated high-yield corporate bonds.",
            Some("Markit iBoxx USD Liquid High Yield Index"),
            Some(0.49),
            Some(5.8),
            Some(3.0),
            vec![
                holding("Consumer Cyclical HY", 18.0),
                holding("Energy HY", 12.0),
                holding("Communications HY", 11.0),
                holding("Healthcare HY", 10.0),
            ],
            vec![
                "금리보다 credit spread와 경기 민감도가 더 큽니다.",
            ],
        )),
        _ => None,
    }
}

fn holding(name: &str, weight_pct: f64) -> HoldingInfo {
    HoldingInfo {
        name: name.to_string(),
        weight_pct,
    }
}

#[allow(clippy::too_many_arguments)]
fn etf_profile(
    symbol: &str,
    display_name: &str,
    issuer: &str,
    category: &str,
    objective: &str,
    benchmark: Option<&str>,
    expense_ratio_pct: Option<f64>,
    distribution_yield_pct: Option<f64>,
    duration_years: Option<f64>,
    top_holdings: Vec<HoldingInfo>,
    notes: Vec<&str>,
) -> InstrumentProfile {
    InstrumentProfile {
        symbol: symbol.to_string(),
        display_name: display_name.to_string(),
        kind: InstrumentKind::Etf,
        issuer: issuer.to_string(),
        category: category.to_string(),
        objective: objective.to_string(),
        benchmark: benchmark.map(str::to_string),
        expense_ratio_pct,
        distribution_yield_pct,
        duration_years,
        top_holdings,
        notes: notes.into_iter().map(str::to_string).collect(),
    }
}

#[allow(clippy::too_many_arguments)]
fn bond_profile(
    symbol: &str,
    display_name: &str,
    issuer: &str,
    category: &str,
    objective: &str,
    benchmark: Option<&str>,
    expense_ratio_pct: Option<f64>,
    distribution_yield_pct: Option<f64>,
    duration_years: Option<f64>,
    top_holdings: Vec<HoldingInfo>,
    notes: Vec<&str>,
) -> InstrumentProfile {
    InstrumentProfile {
        symbol: symbol.to_string(),
        display_name: display_name.to_string(),
        kind: InstrumentKind::BondFund,
        issuer: issuer.to_string(),
        category: category.to_string(),
        objective: objective.to_string(),
        benchmark: benchmark.map(str::to_string),
        expense_ratio_pct,
        distribution_yield_pct,
        duration_years,
        top_holdings,
        notes: notes.into_iter().map(str::to_string).collect(),
    }
}

pub fn related_indicators(sector: &str) -> &'static [&'static str] {
    match sector {
        "Technology" | "Semiconductor" | "Internet" | "IT Services" | "Communication"
        | "Gaming" => &["^IXIC", "BTC-USD"],
        "Energy" | "Chemical" | "Materials" | "Industrial" => &["CL=F", "HG=F"],
        "Financial" | "Insurance" | "Securities" => &["^TNX", "EURUSD=X"],
        "Fintech" => &["^TNX", "BTC-USD"],
        "Consumer" | "Entertainment" => &["GC=F", "EURUSD=X"],
        "Cosmetics" => &["GC=F", "JPY=X"],
        "Auto" | "Auto Parts" => &["KRW=X", "CL=F"],
        "Healthcare" | "Bio" => &["GC=F", "^TNX"],
        "Battery" => &["HG=F", "SI=F"],
        "Steel" => &["HG=F", "CNY=X"],
        "Non-Ferrous" => &["HG=F", "GC=F"],
        "Shipbuilding" | "Shipping" => &["CL=F", "KRW=X"],
        "Construction" => &["HG=F", "^TNX"],
        "Telecom" | "Holding" => &["^TNX", "KRW=X"],
        "Tobacco" => &["GC=F", "KRW=X"],
        "Electronics" | "Components" => &["^IXIC", "KRW=X"],
        _ => &[],
    }
}

pub fn kr_yahoo_candidates(symbol: &str) -> [&str; 2] {
    match symbol {
        "247540" | "036570" | "259960" | "326030" => [".KQ", ".KS"],
        _ => [".KS", ".KQ"],
    }
}

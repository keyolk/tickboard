use std::{env, process::Stdio};

use serde_json::{json, Value};

use crate::models::{
    format_market_cap, format_number, format_volume, Currency, InvestorRow, OrderBookEntry,
    StockDetail, StockQuote,
};

pub const DISABLED_MSG: &str = "⚠ 사용 가능한 AI provider가 없습니다.\n  - Bedrock: us.anthropic.claude-sonnet-4-6 inference profile 또는 AWS credentials 필요\n  - Claude CLI: claude -p 사용 가능해야 함\n  - Codex CLI: codex exec 사용 가능해야 함";

pub fn is_bedrock_available() -> bool {
    env::var("BEDROCK_API_KEY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || env::var("AWS_ACCESS_KEY_ID")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        || env::var("AWS_PROFILE")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}

pub fn is_ai_available() -> bool {
    is_bedrock_available() || command_exists("claude") || command_exists("codex")
}

pub async fn analyze_article(title: &str, content: &str, is_korean: bool) -> String {
    if !is_ai_available() {
        return DISABLED_MSG.to_string();
    }
    let body = truncate_chars(content, 6000);
    let prompt = if is_korean {
        format!("다음 경제/금융 뉴스 기사를 분석해 주세요.\n\n제목: {title}\n\n본문:\n{body}\n\n다음 형식으로 한국어로 작성해 주세요:\n\n## 요약\n(기사의 핵심 내용을 3-5문장으로 요약)\n\n## 분석\n(이 뉴스가 시장에 미치는 영향, 관련 산업/기업에 대한 분석)\n\n## 투자 인사이트\n(투자자 관점에서의 시사점, 주목할 포인트)\n\n## 관련 종목\n(이 뉴스와 관련된 주요 종목들)")
    } else {
        format!("다음 영문 경제/금융 뉴스 기사를 한국어로 번역하고 분석해 주세요.\n\nTitle: {title}\n\nContent:\n{body}\n\n다음 형식으로 한국어로 작성해 주세요:\n\n## 한국어 번역\n(기사 핵심 내용의 한국어 번역, 3-5문장)\n\n## 요약\n(기사의 핵심 내용을 3-5문장으로 요약)\n\n## 분석\n(이 뉴스가 글로벌 시장 및 한국 시장에 미치는 영향 분석)\n\n## 투자 인사이트\n(투자자 관점에서의 시사점, 주목할 포인트)\n\n## 관련 종목\n(이 뉴스와 관련된 주요 종목들 - 미국/한국)")
    };
    invoke_ai(&prompt, 2048)
        .await
        .unwrap_or_else(|error| format!("(AI 분석을 불러올 수 없습니다: {error})"))
}

/// Build the AI stock analysis from the full detail snapshot plus the
/// supporting flows we already collect (investor trends, order-book depth) and
/// a same-sector peer set for relative valuation.
pub async fn analyze_stock(
    detail: &StockDetail,
    investor_rows: &[InvestorRow],
    order_book: &[OrderBookEntry],
    peers: &[StockQuote],
    news_titles: &[String],
) -> String {
    if !is_ai_available() {
        return DISABLED_MSG.to_string();
    }

    let market_name = if detail.market.as_str() == "US" {
        "미국"
    } else {
        "한국"
    };
    let sector = if detail.sector.is_empty() {
        "N/A"
    } else {
        &detail.sector
    };

    let header = format!(
        "종목: {} ({})\n시장: {market_name}\n섹터: {sector}",
        detail.symbol, detail.name
    );
    let prompt = format!(
        "다음 종목을 아래 정량 데이터를 근거로 분석해 주세요. 추측보다 제공된 수치를 인용하며 설명하세요.\n\n\
{header}\n\n\
[밸류에이션 / 펀더멘털]\n{valuation}\n\n\
[가격 / 기술적]\n{technical}\n\n\
[거래량 / 수급]\n{flow}\n\n\
[호가 (매수/매도 잔량)]\n{depth}\n\n\
[동종 섹터 비교]\n{peers}\n\n\
[최근 뉴스]\n{news}\n\n\
다음 형식으로 한국어로 간결하게(각 항목 2-4문장) 작성해 주세요:\n\n\
## 기술적 분석\n(가격 위치, 추세, 모멘텀 — 52주 위치와 기간 수익률 인용)\n\n\
## 수급 분석\n(거래량 추세와 투자자별 순매수, 호가 잔량의 매수/매도 우위 해석)\n\n\
## 밸류에이션 / 동종업계 비교\n(PER·EPS·시총을 동종 섹터 평균과 비교해 고평가/저평가 판단)\n\n\
## 투자 포인트\n(매력 포인트 2-3개)\n\n\
## 리스크 요인\n(주의할 리스크 2-3개)",
        valuation = valuation_section(detail),
        technical = technical_section(detail),
        flow = flow_section(detail, investor_rows),
        depth = depth_section(order_book, detail.currency),
        peers = peers_section(detail, peers),
        news = news_section(news_titles),
    );

    invoke_ai(&prompt, 1536)
        .await
        .unwrap_or_else(|error| format!("(AI 종목 분석을 불러올 수 없습니다: {error})"))
}

fn money(value: f64, currency: Currency) -> String {
    match currency {
        Currency::Usd => format!("${}", format_number(value, 2)),
        Currency::Krw => format_number(value, 0),
    }
}

fn opt_num(value: Option<f64>, suffix: &str) -> String {
    value
        .map(|v| format!("{v:.2}{suffix}"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn valuation_section(d: &StockDetail) -> String {
    format!(
        "시가총액: {}\nPER: {}\nEPS: {}\n배당수익률: {}\n베타: {}",
        if d.market_cap > 0.0 {
            format_market_cap(d.market_cap, d.currency)
        } else {
            "N/A".to_string()
        },
        opt_num(d.pe_ratio, ""),
        opt_num(d.eps, ""),
        opt_num(d.dividend_yield, "%"),
        opt_num(d.beta, ""),
    )
}

fn technical_section(d: &StockDetail) -> String {
    let week52_pct = if d.week52_high > d.week52_low {
        (d.price - d.week52_low) / (d.week52_high - d.week52_low) * 100.0
    } else {
        0.0
    };
    let ret = |hist: &[f64]| -> String {
        match hist.first() {
            Some(first) if *first != 0.0 => format!("{:+.1}%", (d.price - first) / first * 100.0),
            _ => "N/A".to_string(),
        }
    };
    format!(
        "현재가: {} ({:+.2}%)\n당일 시/고/저/전일종가: {} / {} / {} / {}\n52주 범위: {} ~ {} (현재 위치 {:.0}%)\n기간 수익률: 1W {} · 1M {} · 3M {} · 1Y {}",
        money(d.price, d.currency),
        d.change_pct,
        money(d.open_price, d.currency),
        money(d.high, d.currency),
        money(d.low, d.currency),
        money(d.prev_close, d.currency),
        money(d.week52_low, d.currency),
        money(d.week52_high, d.currency),
        week52_pct,
        ret(&d.history_7d),
        ret(&d.history_30d),
        ret(&d.history_90d),
        ret(&d.history_1y),
    )
}

fn flow_section(d: &StockDetail, investor_rows: &[InvestorRow]) -> String {
    let vol_ratio = if d.avg_volume > 0 {
        format!("{:.1}x", d.volume as f64 / d.avg_volume as f64)
    } else {
        "N/A".to_string()
    };
    let mut out = format!(
        "거래량: {} (평균 {} 대비 {})",
        format_volume(d.volume),
        format_volume(d.avg_volume),
        vol_ratio,
    );
    if investor_rows.is_empty() {
        out.push_str("\n투자자별 순매수: 데이터 없음");
    } else {
        // Sum the recent investor-trend rows into a net direction per group.
        let (mut ind, mut frn, mut inst) = (0i64, 0i64, 0i64);
        for row in investor_rows.iter().take(5) {
            ind += row.individual;
            frn += row.foreign;
            inst += row.institution;
        }
        out.push_str(&format!(
            "\n최근 순매수 합계(개인/외국인/기관): {ind:+} / {frn:+} / {inst:+}"
        ));
    }
    out
}

fn depth_section(order_book: &[OrderBookEntry], currency: Currency) -> String {
    if order_book.is_empty() {
        return "데이터 없음".to_string();
    }
    let bid_vol: u64 = order_book
        .iter()
        .filter(|e| e.is_bid)
        .map(|e| e.volume)
        .sum();
    let ask_vol: u64 = order_book
        .iter()
        .filter(|e| !e.is_bid)
        .map(|e| e.volume)
        .sum();
    let best_bid = order_book
        .iter()
        .filter(|e| e.is_bid)
        .map(|e| e.price)
        .fold(f64::MIN, f64::max);
    let best_ask = order_book
        .iter()
        .filter(|e| !e.is_bid)
        .map(|e| e.price)
        .fold(f64::MAX, f64::min);
    let pressure = match (bid_vol, ask_vol) {
        (b, a) if b > a => "매수 우위",
        (b, a) if a > b => "매도 우위",
        _ => "중립",
    };
    format!(
        "매수잔량 합계: {} / 매도잔량 합계: {} ({})\n최우선 매수/매도호가: {} / {}",
        format_volume(bid_vol),
        format_volume(ask_vol),
        pressure,
        if best_bid > f64::MIN {
            money(best_bid, currency)
        } else {
            "N/A".to_string()
        },
        if best_ask < f64::MAX {
            money(best_ask, currency)
        } else {
            "N/A".to_string()
        },
    )
}

fn peers_section(d: &StockDetail, peers: &[StockQuote]) -> String {
    // Exclude the subject itself; peers come pre-filtered to the same sector.
    let others: Vec<&StockQuote> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(&d.symbol))
        .collect();
    if others.is_empty() {
        return "동일 섹터 비교 데이터 없음".to_string();
    }
    let avg_change = others.iter().map(|p| p.change_pct).sum::<f64>() / others.len() as f64;
    let mut lines = vec![format!(
        "동일 섹터({}) {}개 종목 · 평균 등락률 {:+.2}% (본 종목 {:+.2}%)",
        if d.sector.is_empty() {
            "N/A"
        } else {
            &d.sector
        },
        others.len(),
        avg_change,
        d.change_pct,
    )];
    // List up to 6 peers by market cap so the model has concrete comparables.
    let mut sorted = others;
    sorted.sort_by(|a, b| b.market_cap.total_cmp(&a.market_cap));
    for p in sorted.into_iter().take(6) {
        lines.push(format!(
            "- {} {} | 등락 {:+.2}% | 시총 {}",
            p.symbol,
            p.name,
            p.change_pct,
            if p.market_cap > 0.0 {
                format_market_cap(p.market_cap, p.currency)
            } else {
                "N/A".to_string()
            },
        ));
    }
    lines.join("\n")
}

fn news_section(news_titles: &[String]) -> String {
    if news_titles.is_empty() {
        "(없음)".to_string()
    } else {
        news_titles
            .iter()
            .take(5)
            .map(|t| format!("- {t}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

async fn invoke_ai(prompt: &str, max_tokens: u32) -> anyhow::Result<String> {
    let mut errors = Vec::new();
    if is_bedrock_available() {
        match invoke_bedrock(prompt, max_tokens).await {
            Ok(text) if !text.trim().is_empty() => return Ok(text),
            Ok(_) => errors.push("Bedrock returned an empty response".to_string()),
            Err(error) => errors.push(format!("Bedrock: {error}")),
        }
    }
    if command_exists("claude") {
        match invoke_claude_cli(prompt).await {
            Ok(text) if !text.trim().is_empty() => return Ok(text),
            Ok(_) => errors.push("Claude CLI returned an empty response".to_string()),
            Err(error) => errors.push(format!("Claude CLI: {error}")),
        }
    }
    if command_exists("codex") {
        match invoke_codex_cli(prompt).await {
            Ok(text) if !text.trim().is_empty() => return Ok(text),
            Ok(_) => errors.push("Codex CLI returned an empty response".to_string()),
            Err(error) => errors.push(format!("Codex CLI: {error}")),
        }
    }
    Err(anyhow::anyhow!(errors.join(" | ")))
}

async fn invoke_bedrock(prompt: &str, max_tokens: u32) -> anyhow::Result<String> {
    let region = env::var("BEDROCK_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let model_id = env::var("BEDROCK_MODEL_ID")
        .unwrap_or_else(|_| "us.anthropic.claude-sonnet-4-6".to_string());
    let body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": max_tokens,
        "messages": [{"role": "user", "content": prompt}],
    });
    let body_file = std::env::temp_dir().join(format!(
        "tickboard-bedrock-body-{}-{}.json",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let output_file = std::env::temp_dir().join(format!(
        "tickboard-bedrock-output-{}-{}.json",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    tokio::fs::write(&body_file, serde_json::to_vec(&body)?).await?;
    let body_arg = format!("fileb://{}", body_file.display());
    let output_arg = output_file.to_string_lossy().to_string();
    let output = {
        let mut command = tokio::process::Command::new("aws");
        command
            .args([
                "bedrock-runtime",
                "invoke-model",
                "--region",
                &region,
                "--model-id",
                &model_id,
                "--content-type",
                "application/json",
                "--accept",
                "application/json",
                "--body",
                &body_arg,
                &output_arg,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Ok(api_key) = env::var("BEDROCK_API_KEY") {
            if !api_key.is_empty() {
                command.env("AWS_ACCESS_KEY_ID", &api_key);
                command.env("AWS_SECRET_ACCESS_KEY", &api_key);
            }
        }
        command.output().await?
    };
    let _ = tokio::fs::remove_file(&body_file).await;
    if !output.status.success() {
        let _ = tokio::fs::remove_file(&output_file).await;
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }
    let response_bytes = tokio::fs::read(&output_file).await?;
    let _ = tokio::fs::remove_file(&output_file).await;
    let value: Value = serde_json::from_slice(&response_bytes)?;
    Ok(value
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string())
}

async fn invoke_claude_cli(prompt: &str) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("claude")
        .args(["-p", prompt, "--output-format", "text"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn invoke_codex_cli(prompt: &str) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("codex")
        .args(["exec", "--sandbox", "read-only", prompt])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_exists(command: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{InvestorRow, Market, OrderBookEntry, StockQuote};

    fn sample_detail() -> StockDetail {
        StockDetail {
            symbol: "AAPL".to_string(),
            name: "Apple".to_string(),
            market: Market::Us,
            currency: Currency::Usd,
            price: 150.0,
            change: 1.5,
            change_pct: 1.0,
            open_price: 148.0,
            high: 151.0,
            low: 147.0,
            prev_close: 148.5,
            volume: 90_000_000,
            avg_volume: 60_000_000,
            market_cap: 2_500_000_000_000.0,
            pe_ratio: Some(28.0),
            week52_high: 200.0,
            week52_low: 100.0,
            history_7d: vec![145.0, 150.0],
            history_30d: vec![140.0, 150.0],
            history_90d: vec![120.0, 150.0],
            history_1y: vec![100.0, 150.0],
            intraday_times: vec![],
            intraday_volumes: vec![],
            intraday_up_bars: vec![],
            volume_history_7d: vec![],
            volume_history_30d: vec![],
            volume_history_90d: vec![],
            volume_history_1y: vec![],
            history_dates_7d: vec![],
            history_dates_30d: vec![],
            history_dates_90d: vec![],
            history_dates_1y: vec![],
            day_change: 1.5,
            day_change_pct: 1.0,
            eps: Some(6.0),
            dividend_yield: Some(0.5),
            beta: Some(1.2),
            sector: "Technology".to_string(),
            last_updated: None,
        }
    }

    fn peer(symbol: &str, change_pct: f64, market_cap: f64) -> StockQuote {
        StockQuote {
            symbol: symbol.to_string(),
            name: format!("{symbol} Inc"),
            price: 100.0,
            change: 0.0,
            change_pct,
            volume: 1_000_000,
            market: Market::Us,
            currency: Currency::Usd,
            sector: "Technology".to_string(),
            tags: vec![],
            market_cap,
            history_7d: vec![],
            last_updated: None,
        }
    }

    #[test]
    fn valuation_section_includes_fundamentals() {
        let s = valuation_section(&sample_detail());
        assert!(s.contains("PER: 28.00"));
        assert!(s.contains("EPS: 6.00"));
        assert!(s.contains("배당수익률: 0.50%"));
        assert!(s.contains("베타: 1.20"));
        assert!(s.contains("$2.50T"));
    }

    #[test]
    fn flow_section_sums_investor_rows() {
        let rows = vec![
            InvestorRow {
                date: "1".into(),
                individual: -100,
                foreign: 60,
                institution: 40,
            },
            InvestorRow {
                date: "2".into(),
                individual: -50,
                foreign: 30,
                institution: 20,
            },
        ];
        let s = flow_section(&sample_detail(), &rows);
        assert!(s.contains("1.5x")); // 90M / 60M
        assert!(s.contains("-150 / +90 / +60"));
    }

    #[test]
    fn depth_section_reports_pressure() {
        let book = vec![
            OrderBookEntry {
                price: 149.0,
                volume: 5000,
                is_bid: true,
            },
            OrderBookEntry {
                price: 151.0,
                volume: 2000,
                is_bid: false,
            },
        ];
        let s = depth_section(&book, Currency::Usd);
        assert!(s.contains("매수 우위"));
    }

    #[test]
    fn peers_section_excludes_self_and_averages() {
        let peers = vec![
            peer("AAPL", 1.0, 2.5e12),
            peer("MSFT", 3.0, 3.0e12),
            peer("NVDA", 5.0, 2.0e12),
        ];
        let s = peers_section(&sample_detail(), &peers);
        // Self excluded → 2 peers, average of 3.0 and 5.0 = 4.0.
        assert!(s.contains("2개 종목"));
        assert!(s.contains("+4.00%"));
        assert!(s.contains("MSFT"));
        assert!(!s.lines().any(|l| l.starts_with("- AAPL")));
    }
}

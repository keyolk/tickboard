use std::{env, process::Stdio};

use serde_json::{json, Value};

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

#[allow(clippy::too_many_arguments)]
pub async fn analyze_stock(
    symbol: &str,
    name: &str,
    price: f64,
    change_pct: f64,
    pe_ratio: Option<f64>,
    week52_high: f64,
    week52_low: f64,
    sector: &str,
    market: &str,
    news_titles: &[String],
) -> String {
    if !is_ai_available() {
        return DISABLED_MSG.to_string();
    }
    let news = if news_titles.is_empty() {
        "(없음)".to_string()
    } else {
        news_titles
            .iter()
            .take(5)
            .map(|t| format!("- {t}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let per = pe_ratio
        .map(|v| format!("{v:.2}"))
        .unwrap_or_else(|| "N/A".to_string());
    let week52_pct = if week52_high > week52_low {
        (price - week52_low) / (week52_high - week52_low) * 100.0
    } else {
        0.0
    };
    let market_name = if market == "US" { "미국" } else { "한국" };
    let sector = if sector.is_empty() { "N/A" } else { sector };
    let prompt = format!("다음 종목을 간결하게 분석해 주세요. 각 항목을 2-3문장으로 작성하세요.\n\n종목: {symbol} ({name})\n시장: {market_name}\n섹터: {sector}\n현재가: {price:.2} ({change_pct:+.2}%)\nPER: {per}\n52주 범위: {week52_low:.2} ~ {week52_high:.2} (현재 위치: {week52_pct:.0}%)\n\n최근 뉴스:\n{news}\n\n다음 형식으로 한국어로 간결하게 작성해 주세요:\n\n## 기술적 분석\n(가격 위치, 추세, 모멘텀에 대한 간단 분석)\n\n## 투자 포인트\n(이 종목의 매력 포인트 2-3개)\n\n## 리스크 요인\n(주의할 리스크 2-3개)");
    invoke_ai(&prompt, 1024)
        .await
        .unwrap_or_else(|error| format!("(AI 종목 분석을 불러올 수 없습니다: {error})"))
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

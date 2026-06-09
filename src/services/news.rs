use regex::Regex;

use crate::{
    config,
    models::{Market, NewsItem},
};

const EN_KO_TERMS: &[(&str, &str)] = &[
    ("Federal Reserve", "연준"),
    ("interest rates", "금리"),
    ("interest rate", "금리"),
    ("record high", "사상 최고치"),
    ("record low", "사상 최저치"),
    ("Wall Street", "월스트리트"),
    ("semiconductors", "반도체"),
    ("semiconductor", "반도체"),
    ("cryptocurrency", "암호화폐"),
    ("Treasury", "국채"),
    ("inflation", "인플레이션"),
    ("recession", "경기침체"),
    ("earnings", "실적"),
    ("revenue", "매출"),
    ("profit", "이익"),
    ("market", "시장"),
    ("markets", "시장"),
    ("stocks", "주식"),
    ("stock", "주식"),
    ("rally", "랠리"),
    ("crash", "폭락"),
    ("bull", "강세"),
    ("bear", "약세"),
    ("Fed", "연준"),
    ("bond", "채권"),
    ("bonds", "채권"),
    ("yield", "수익률"),
    ("yields", "수익률"),
    ("loss", "손실"),
    ("Nasdaq", "나스닥"),
    ("Dow", "다우"),
    ("investors", "투자자"),
    ("investor", "투자자"),
    ("trade", "무역"),
    ("tariffs", "관세"),
    ("tariff", "관세"),
    ("oil", "유가"),
    ("gold", "금"),
    ("Bitcoin", "비트코인"),
    ("tech", "기술주"),
    ("chip", "반도체"),
    ("chips", "반도체"),
    ("banks", "은행"),
    ("bank", "은행"),
    ("economy", "경제"),
    ("rises", "상승"),
    ("rise", "상승"),
    ("falls", "하락"),
    ("fall", "하락"),
    ("drops", "하락"),
    ("drop", "하락"),
    ("surges", "급등"),
    ("surge", "급등"),
    ("plunges", "급락"),
    ("plunge", "급락"),
    ("jumps", "급등"),
    ("jump", "급등"),
    ("gains", "상승"),
    ("gain", "상승"),
    ("declines", "하락"),
    ("decline", "하락"),
    ("China", "중국"),
    ("Japan", "일본"),
    ("Europe", "유럽"),
    ("Korea", "한국"),
    ("Trump", "트럼프"),
    ("Biden", "바이든"),
    ("Apple", "애플"),
    ("Microsoft", "마이크로소프트"),
    ("Google", "구글"),
    ("Amazon", "아마존"),
    ("Tesla", "테슬라"),
    ("Nvidia", "엔비디아"),
    ("Meta", "메타"),
    ("Netflix", "넷플릭스"),
    ("Samsung", "삼성"),
    ("analysts", "애널리스트"),
    ("analyst", "애널리스트"),
    ("report", "보고서"),
    ("quarterly", "분기"),
    ("quarter", "분기"),
    ("annual", "연간"),
    ("growth", "성장"),
    ("forecast", "전망"),
    ("prices", "가격"),
    ("price", "가격"),
    ("trading", "거래"),
    ("session", "세션"),
    ("higher", "더 높은"),
    ("lower", "더 낮은"),
    ("billion", "십억"),
    ("trillion", "조"),
    ("million", "백만"),
    ("percent", "퍼센트"),
    ("index", "지수"),
    ("crude", "원유"),
    ("crypto", "암호화폐"),
    ("Ethereum", "이더리움"),
    ("dollar", "달러"),
    ("yen", "엔"),
    ("yuan", "위안"),
    ("won", "원"),
    ("euro", "유로"),
];

pub async fn fetch_news(max_per_source: usize) -> Vec<NewsItem> {
    let client = super::http_client();
    let mut all_news = Vec::new();
    for (source_key, url) in config::NEWS_FEEDS {
        let Ok(response) = client.get(*url).send().await else {
            continue;
        };
        let Ok(text) = response.text().await else {
            continue;
        };
        let is_korean = matches!(*source_key, "hankyung" | "mk");
        let source_label = match *source_key {
            "yahoo" | "yahoo_markets" => "Yahoo",
            "hankyung" => "한경",
            "mk" => "매경",
            other => other,
        };
        all_news.extend(
            parse_rss(&text, source_label, is_korean)
                .into_iter()
                .take(max_per_source),
        );
    }
    all_news
}

pub async fn fetch_company_news(
    symbol: &str,
    name: &str,
    market: Market,
    max_items: usize,
) -> Vec<NewsItem> {
    let client = super::http_client();
    let (url, source, is_korean) = match market {
        Market::Us => (
            format!(
                "https://feeds.finance.yahoo.com/rss/2.0/headline?s={symbol}&region=US&lang=en-US"
            ),
            "Yahoo",
            false,
        ),
        Market::Kr => (
            format!(
                "https://news.google.com/rss/search?q={}&hl=ko&gl=KR&ceid=KR:ko",
                urlencoding::encode(&build_kr_company_query(symbol, name))
            ),
            "Google",
            true,
        ),
    };
    let Ok(response) = client.get(url).send().await else {
        return Vec::new();
    };
    let Ok(text) = response.text().await else {
        return Vec::new();
    };
    let items = parse_rss(&text, source, is_korean);
    let filtered = match market {
        Market::Us => items,
        Market::Kr => filter_kr_company_news(items, symbol, name),
    };
    filtered.into_iter().take(max_items).collect()
}

pub async fn fetch_article_content(url: &str) -> String {
    let client = super::http_client();
    let Ok(response) = client.get(url).send().await else {
        return "(기사를 불러올 수 없습니다)".to_string();
    };
    if !response.status().is_success() {
        return "(기사를 불러올 수 없습니다)".to_string();
    }
    let Ok(html) = response.text().await else {
        return "(기사 본문을 추출할 수 없습니다. 브라우저에서 확인해 주세요.)".to_string();
    };
    extract_article_text(&html)
}

fn build_kr_company_query(symbol: &str, name: &str) -> String {
    let aliases = kr_company_aliases(symbol, name);
    let query = aliases
        .iter()
        .map(|alias| format!("\"{alias}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    format!("({query}) (주식 OR 실적 OR 목표가 OR 증권)")
}

fn filter_kr_company_news(items: Vec<NewsItem>, symbol: &str, name: &str) -> Vec<NewsItem> {
    let aliases = kr_company_aliases(symbol, name)
        .into_iter()
        .map(|alias| alias.to_lowercase())
        .collect::<Vec<_>>();
    let generic_noise = [
        "네이버 증권",
        "naver pay",
        "pay naver",
        "news.naver.com",
        "finance.naver.com",
    ];
    items
        .into_iter()
        .filter(|item| {
            let haystack =
                format!("{} {} {}", item.source, item.title, item.description).to_lowercase();
            let has_alias = aliases.iter().any(|alias| haystack.contains(alias));
            if !has_alias {
                return false;
            }
            let is_noise_only = generic_noise.iter().any(|noise| haystack.contains(noise))
                && !aliases
                    .iter()
                    .any(|alias| alias != "naver" && haystack.contains(alias));
            !is_noise_only
        })
        .collect()
}

fn kr_company_aliases(symbol: &str, name: &str) -> Vec<String> {
    let mut aliases = vec![name.to_string(), symbol.to_string()];
    match symbol {
        "035420" => aliases.extend([
            "NAVER".to_string(),
            "네이버".to_string(),
            "네이버 주식".to_string(),
        ]),
        "035720" => aliases.extend([
            "Kakao".to_string(),
            "카카오".to_string(),
            "카카오 주식".to_string(),
        ]),
        _ => {}
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn parse_rss(xml_text: &str, source: &str, is_korean: bool) -> Vec<NewsItem> {
    let item_re = Regex::new(r"(?is)<item\b[^>]*>(.*?)</item>").unwrap();
    item_re
        .captures_iter(xml_text)
        .filter_map(|caps| {
            let item = caps.get(1)?.as_str();
            let mut title = extract_tag(item, "title")?;
            if title.trim().is_empty() {
                return None;
            }
            let link = extract_tag(item, "link").unwrap_or_default();
            let published = extract_tag(item, "pubDate").unwrap_or_default();
            let mut description = extract_tag(item, "description").unwrap_or_default();
            description = clean_html(&description);
            title = clean_html(&title);
            if !is_korean {
                title = format!("[EN] {}", translate_text(&title));
                description = translate_text(&description);
            }
            Some(NewsItem {
                title,
                source: source.to_string(),
                url: clean_html(&link),
                published: published.chars().take(16).collect(),
                description,
                is_korean,
            })
        })
        .collect()
}

fn extract_tag(text: &str, tag: &str) -> Option<String> {
    let regex = Regex::new(&format!(r"(?is)<{tag}\b[^>]*>(.*?)</{tag}>")).ok()?;
    let value = regex.captures(text)?.get(1)?.as_str();
    Some(strip_cdata(value).trim().to_string())
}

fn strip_cdata(text: &str) -> String {
    text.trim()
        .strip_prefix("<![CDATA[")
        .and_then(|s| s.strip_suffix("]]>"))
        .unwrap_or(text)
        .to_string()
}

fn translate_text(text: &str) -> String {
    let mut result = text.to_string();
    let mut terms = EN_KO_TERMS.to_vec();
    terms.sort_by_key(|(en, _)| std::cmp::Reverse(en.len()));
    for (en, ko) in terms {
        if let Ok(regex) = Regex::new(&format!("(?i){}", regex::escape(en))) {
            result = regex.replace_all(&result, ko).to_string();
        }
    }
    result
}

fn clean_html(text: &str) -> String {
    let without_tags = Regex::new(r"(?is)<[^>]+>")
        .unwrap()
        .replace_all(text, " ")
        .to_string();
    decode_entities(&Regex::new(r"\s+").unwrap().replace_all(&without_tags, " "))
        .trim()
        .to_string()
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn extract_article_text(html: &str) -> String {
    let mut parts = Vec::new();
    if let Some(article) = capture_block(html, "article", None) {
        parts.extend(extract_paragraphs(&article, 20));
    }
    if parts.is_empty() {
        for class in [
            "article-body",
            "article_body",
            "article-content",
            "newsct_article",
            "news_end",
            "view_con",
        ] {
            if let Some(block) = capture_block(html, "div", Some(class)) {
                parts.extend(extract_paragraphs(&block, 20));
                if !parts.is_empty() {
                    break;
                }
            }
        }
    }
    if parts.is_empty() {
        parts.extend(extract_paragraphs(html, 50).into_iter().filter(|part| {
            let lower = part.to_lowercase();
            part.len() > 50
                && ![
                    "cookie",
                    "javascript",
                    "subscribe",
                    "sign up",
                    "login",
                    "copyright",
                    "privacy policy",
                    "terms of",
                ]
                .iter()
                .any(|skip| lower.contains(skip))
        }));
    }
    let content = parts.into_iter().take(25).collect::<Vec<_>>().join("\n\n");
    if content.is_empty() {
        "(기사 본문을 추출할 수 없습니다. 브라우저에서 확인해 주세요.)".to_string()
    } else {
        content
    }
}

fn capture_block(html: &str, tag: &str, class: Option<&str>) -> Option<String> {
    let pattern = if let Some(class) = class {
        format!(
            r#"(?is)<{tag}[^>]*class="[^"]*{}[^"]*"[^>]*>(.*?)</{tag}>"#,
            regex::escape(class)
        )
    } else {
        format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>")
    };
    Regex::new(&pattern)
        .ok()?
        .captures(html)?
        .get(1)
        .map(|m| m.as_str().to_string())
}

fn extract_paragraphs(html: &str, min_len: usize) -> Vec<String> {
    Regex::new(r"(?is)<p[^>]*>(.*?)</p>")
        .unwrap()
        .captures_iter(html)
        .filter_map(|caps| caps.get(1).map(|m| clean_html(m.as_str())))
        .filter(|text| text.len() > min_len)
        .collect()
}

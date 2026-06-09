use tokio::sync::mpsc::UnboundedSender;

use crate::{models::Market, tasks::AppMsg};

pub fn spawn_dashboard_refresh(
    tx: UnboundedSender<AppMsg>,
    generation: u64,
    us_symbols: Vec<String>,
    kr_symbols: Vec<String>,
    etf_symbols: Vec<String>,
    bond_symbols: Vec<String>,
) {
    tokio::spawn(async move {
        let us_refs = us_symbols.iter().map(String::as_str).collect::<Vec<_>>();
        let kr_refs = kr_symbols.iter().map(String::as_str).collect::<Vec<_>>();
        let etf_refs = etf_symbols.iter().map(String::as_str).collect::<Vec<_>>();
        let bond_refs = bond_symbols.iter().map(String::as_str).collect::<Vec<_>>();
        let (us, kr) = tokio::join!(
            crate::services::yahoo::fetch_us_indices(),
            crate::services::yahoo::fetch_kr_indices()
        );
        let _ = tx.send(AppMsg::DashboardIndicesLoaded { generation, us, kr });

        let (indicators, fx_rates) = tokio::join!(
            crate::services::indicators::fetch_indicators(),
            crate::services::indicators::fetch_fx_rates(),
        );
        let _ = tx.send(AppMsg::DashboardIndicatorsLoaded {
            generation,
            indicators,
            fx_rates,
        });

        let (us_quotes, kr_quotes, etf_quotes, bond_quotes) = tokio::join!(
            crate::services::yahoo::fetch_us_quotes(&us_refs),
            crate::services::kr::fetch_kr_quotes(&kr_refs),
            crate::services::yahoo::fetch_us_quotes(&etf_refs),
            crate::services::yahoo::fetch_us_quotes(&bond_refs),
        );
        let _ = tx.send(AppMsg::DashboardQuotesLoaded {
            generation,
            us: us_quotes,
            kr: kr_quotes,
            etfs: etf_quotes,
            bonds: bond_quotes,
        });

        let (us_caps, kr_caps, etf_caps, bond_caps) = tokio::join!(
            crate::services::yahoo::fetch_us_market_caps(&us_refs),
            crate::services::kr::fetch_kr_market_caps(&kr_refs),
            crate::services::yahoo::fetch_us_market_caps(&etf_refs),
            crate::services::yahoo::fetch_us_market_caps(&bond_refs),
        );
        let _ = tx.send(AppMsg::DashboardMarketCapsLoaded {
            generation,
            us: us_caps,
            kr: kr_caps,
            etfs: etf_caps,
            bonds: bond_caps,
        });
    });
}

pub fn spawn_news_refresh(tx: UnboundedSender<AppMsg>, generation: u64) {
    tokio::spawn(async move {
        let news = crate::services::news::fetch_news(10).await;
        let _ = tx.send(AppMsg::NewsLoaded { generation, news });
    });
}

pub fn spawn_detail_refresh(
    tx: UnboundedSender<AppMsg>,
    generation: u64,
    symbol: String,
    market: Market,
) {
    tokio::spawn(async move {
        let detail = match market {
            Market::Us => crate::services::yahoo::fetch_us_stock_detail(&symbol).await,
            Market::Kr => crate::services::kr::fetch_kr_stock_detail(&symbol).await,
        };
        let _ = tx.send(AppMsg::DetailLoaded {
            generation,
            detail,
            error: None,
        });

        let symbol_for_fundamentals = symbol.clone();
        let tx_fundamentals = tx.clone();
        tokio::spawn(async move {
            let mut data = std::collections::HashMap::new();
            for attempt in 0..4 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(5 * attempt)).await;
                }
                data = match market {
                    Market::Us => {
                        crate::services::yahoo::fetch_us_fundamentals(&symbol_for_fundamentals)
                            .await
                    }
                    Market::Kr => {
                        crate::services::kr::fetch_kr_fundamentals(&symbol_for_fundamentals).await
                    }
                };
                if !data.is_empty() {
                    break;
                }
            }
            let _ = tx_fundamentals.send(AppMsg::FundamentalsLoaded { generation, data });
        });

        let name = crate::config::stock_name(market, &symbol).to_string();
        let (news, entries, rows) = tokio::join!(
            crate::services::news::fetch_company_news(&symbol, &name, market, 8),
            crate::services::simulation::fetch_order_book(&symbol, market),
            crate::services::simulation::fetch_investor_trends(&symbol, market, 10),
        );
        let _ = tx.send(AppMsg::CompanyNewsLoaded { generation, news });
        let _ = tx.send(AppMsg::OrderBookLoaded {
            generation,
            entries,
        });
        let _ = tx.send(AppMsg::InvestorTrendsLoaded { generation, rows });
    });
}

pub fn spawn_stock_ai(
    tx: UnboundedSender<AppMsg>,
    generation: u64,
    detail: crate::models::StockDetail,
    news_titles: Vec<String>,
) {
    tokio::spawn(async move {
        let result = crate::services::bedrock::analyze_stock(
            &detail.symbol,
            &detail.name,
            detail.price,
            detail.change_pct,
            detail.pe_ratio,
            detail.week52_high,
            detail.week52_low,
            &detail.sector,
            detail.market.as_str(),
            &news_titles,
        )
        .await;
        let _ = tx.send(AppMsg::StockAiLoaded { generation, result });
    });
}

pub fn spawn_article_analysis(
    tx: UnboundedSender<AppMsg>,
    generation: u64,
    item: crate::models::NewsItem,
) {
    tokio::spawn(async move {
        let content = crate::services::news::fetch_article_content(&item.url).await;
        let analysis =
            crate::services::bedrock::analyze_article(&item.title, &content, item.is_korean).await;
        let _ = tx.send(AppMsg::ArticleLoaded {
            generation,
            content,
            analysis,
        });
    });
}

use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::{self, instrument_profile},
    models::{
        format_number, format_volume, ChartPeriod, Currency, InstrumentKind, InvestorRow, Market,
        NewsItem, OrderBookEntry, StockDetail, StockQuote,
    },
    tasks::{scheduler, AppMsg},
    ui::{format, layout, theme, widgets},
};

#[derive(Debug)]
pub struct DetailState {
    pub generation: u64,
    pub symbol: String,
    pub market: Market,
    pub detail: Option<StockDetail>,
    pub news: Vec<NewsItem>,
    pub order_book: Vec<OrderBookEntry>,
    pub investor_rows: Vec<InvestorRow>,
    pub period: ChartPeriod,
    pub news_selected: usize,
    pub news_search: String,
    pub news_search_mode: bool,
    pub ai_result: String,
    pub ai_loading: bool,
    pub ai_visible: bool,
    pub status: String,
}

impl DetailState {
    pub fn new(symbol: String, market: Market) -> Self {
        Self {
            generation: 0,
            symbol,
            market,
            detail: None,
            news: Vec::new(),
            order_book: Vec::new(),
            investor_rows: Vec::new(),
            period: ChartPeriod::OneWeek,
            news_selected: 0,
            news_search: String::new(),
            news_search_mode: false,
            ai_result: String::new(),
            ai_loading: false,
            ai_visible: false,
            status: "Loading...".to_string(),
        }
    }

    pub fn refresh(&mut self, tx: UnboundedSender<AppMsg>) {
        self.generation = self.generation.wrapping_add(1);
        self.status = "Refreshing...".to_string();
        scheduler::spawn_detail_refresh(tx, self.generation, self.symbol.clone(), self.market);
    }

    pub fn toggle_ai(&mut self, tx: UnboundedSender<AppMsg>, peers: Vec<StockQuote>) {
        if !crate::services::bedrock::is_ai_available() {
            self.ai_result = crate::services::bedrock::DISABLED_MSG.to_string();
            self.ai_visible = true;
            return;
        }
        if self.ai_loading {
            return;
        }
        if self.ai_visible && !self.ai_result.is_empty() {
            self.ai_visible = false;
            return;
        }
        if !self.ai_result.is_empty() {
            self.ai_visible = true;
            return;
        }
        if let Some(detail) = self.detail.clone() {
            self.ai_loading = true;
            self.ai_visible = true;
            self.ai_result = "AI 종목 분석 로딩중...".to_string();
            let news_titles = self.news.iter().take(5).map(|n| n.title.clone()).collect();
            scheduler::spawn_stock_ai(
                tx,
                self.generation,
                detail,
                self.investor_rows.clone(),
                self.order_book.clone(),
                peers,
                news_titles,
            );
        }
    }

    pub fn apply_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::DetailLoaded {
                generation,
                detail,
                error,
            } if *generation == self.generation => {
                self.detail = detail.clone();
                self.status = if detail.is_some() {
                    "Updated detail | loading fundamentals...".to_string()
                } else {
                    error
                        .clone()
                        .unwrap_or_else(|| "Failed to load. Press R to retry.".to_string())
                };
            }
            AppMsg::FundamentalsLoaded { generation, data } if *generation == self.generation => {
                if let Some(detail) = &mut self.detail {
                    crate::services::kr::merge_fundamentals(detail, data);
                }
                self.status = "Updated: [1]1W [2]1M [3]3M [4]1Y [A]AI [B/Esc]Back".to_string();
            }
            AppMsg::CompanyNewsLoaded { generation, news } if *generation == self.generation => {
                self.news = news.clone();
                self.news_selected = self
                    .news_selected
                    .min(self.visible_news().len().saturating_sub(1));
            }
            AppMsg::OrderBookLoaded {
                generation,
                entries,
            } if *generation == self.generation => {
                self.order_book = entries.clone();
            }
            AppMsg::InvestorTrendsLoaded { generation, rows } if *generation == self.generation => {
                self.investor_rows = rows.clone();
            }
            AppMsg::StockAiLoaded { generation, result } if *generation == self.generation => {
                self.ai_result = result.clone();
                self.ai_loading = false;
                self.ai_visible = true;
            }
            _ => {}
        }
    }

    pub fn begin_news_search(&mut self) {
        self.news_search_mode = true;
    }

    pub fn push_news_search_char(&mut self, character: char) {
        self.news_search.push(character);
        self.news_selected = self
            .news_selected
            .min(self.visible_news().len().saturating_sub(1));
    }

    pub fn pop_news_search_char(&mut self) {
        self.news_search.pop();
        self.news_selected = self
            .news_selected
            .min(self.visible_news().len().saturating_sub(1));
    }

    pub fn end_news_search(&mut self) {
        self.news_search_mode = false;
        self.news_selected = self
            .news_selected
            .min(self.visible_news().len().saturating_sub(1));
    }

    pub fn visible_news(&self) -> Vec<NewsItem> {
        let query = self.news_search.trim().to_lowercase();
        if query.is_empty() {
            return self.news.clone();
        }
        self.news
            .iter()
            .filter(|item| {
                let haystack =
                    format!("{} {} {}", item.source, item.title, item.description).to_lowercase();
                query.split_whitespace().all(|term| haystack.contains(term))
            })
            .cloned()
            .collect()
    }

    pub fn has_active_news_filter(&self) -> bool {
        !self.news_search.is_empty()
    }

    pub fn clear_news_filter(&mut self) {
        self.news_search.clear();
        self.news_search_mode = false;
        self.news_selected = self
            .news_selected
            .min(self.visible_news().len().saturating_sub(1));
    }

    pub fn jump_news_start(&mut self) {
        self.news_selected = 0;
    }

    pub fn jump_news_end(&mut self) {
        self.news_selected = self.visible_news().len().saturating_sub(1);
    }

    pub fn page_news(&mut self, delta: isize) {
        self.news_selected = move_index(self.news_selected, self.visible_news().len(), delta);
    }

    pub fn move_news(&mut self, delta: isize) {
        let visible_len = self.visible_news().len();
        if visible_len == 0 {
            self.news_selected = 0;
            return;
        }
        self.news_selected =
            (self.news_selected as isize + delta).clamp(0, visible_len as isize - 1) as usize;
    }

    pub fn selected_news(&self) -> Option<NewsItem> {
        self.visible_news().get(self.news_selected).cloned()
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &DetailState,
    indicators: &[crate::models::EconomicIndicator],
) {
    let show_ai = state.ai_visible && !state.ai_result.is_empty();
    let mut constraints = vec![
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Length(9),
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Min(6),
    ];
    if show_ai {
        constraints.push(Constraint::Min(8));
    }
    constraints.push(Constraint::Length(1));
    let rows = layout::vertical(area, &constraints);
    render_header(
        frame,
        rows[0],
        state.detail.as_ref(),
        &state.symbol,
        state.market,
    );
    render_metrics(frame, rows[1], state.detail.as_ref());
    widgets::rich_chart::render(frame, rows[2], state.detail.as_ref(), state.period);
    widgets::volume_chart::render(frame, rows[3], state.detail.as_ref(), state.period);
    widgets::range_gauge::render(frame, rows[4], state.detail.as_ref());
    let mid = layout::horizontal(
        rows[5],
        &[Constraint::Percentage(42), Constraint::Percentage(58)],
    );
    render_order_book(
        frame,
        mid[0],
        &state.order_book,
        state.market,
        state.detail.as_ref().map(|d| d.symbol.as_str()),
    );
    render_info_and_investors(frame, mid[1], state.detail.as_ref(), &state.investor_rows);
    render_related_and_ai(frame, rows[6], state.detail.as_ref(), indicators, state);
    let visible_news = state.visible_news();
    widgets::news_feed::render(
        frame,
        rows[7],
        &visible_news,
        state.news_selected,
        true,
        &format!(
            " Company News [Enter] AI Analysis [A] AI 종목분석 [/ {}] ",
            if state.news_search.is_empty() {
                "search".to_string()
            } else {
                state.news_search.clone()
            }
        ),
    );
    if show_ai {
        frame.render_widget(
            Paragraph::new(state.ai_result.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" AI 종목 분석 ")
                        .border_style(Style::default().fg(theme::ACCENT)),
                )
                .wrap(ratatui::widgets::Wrap { trim: false }),
            rows[8],
        );
    }
    let status_row = if show_ai { rows[9] } else { rows[8] };
    frame.render_widget(
        Paragraph::new(state.status.clone()).style(Style::default().fg(theme::DIM)),
        status_row,
    );
}

fn render_header(
    frame: &mut Frame,
    area: Rect,
    detail: Option<&StockDetail>,
    symbol: &str,
    market: Market,
) {
    let lines = if let Some(d) = detail {
        let color = theme::change_color(d.change_pct);
        vec![
            Line::from(vec![
                Span::styled(
                    format!(" {}  {}", d.symbol, d.name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  |  Market: {}", d.market.display())),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(" {} ", format::money(d.price, d.currency)),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "{} {} ({})",
                        if d.change >= 0.0 { "▲" } else { "▼" },
                        format::money(d.change, d.currency),
                        d.change_pct
                    ),
                    Style::default().fg(color),
                ),
                Span::raw(format!("  Vol: {}", format_volume(d.volume))),
            ]),
        ]
    } else {
        vec![
            Line::from(format!(" {symbol}  {}", market.display())),
            Line::from(" Loading..."),
        ]
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        area,
    );
}

fn render_metrics(frame: &mut Frame, area: Rect, detail: Option<&StockDetail>) {
    let chunks = layout::horizontal(
        area,
        &[
            Constraint::Percentage(15),
            Constraint::Percentage(13),
            Constraint::Percentage(13),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(15),
            Constraint::Percentage(16),
        ],
    );
    let profile = detail.and_then(|d| instrument_profile(&d.symbol));
    let cards = if let Some(d) = detail {
        vec![
            (
                if profile.is_some() {
                    "AUM/Cap"
                } else {
                    "Market Cap"
                },
                d.formatted_market_cap(),
            ),
            (
                if matches!(
                    profile.as_ref().map(|p| p.kind),
                    Some(InstrumentKind::Etf | InstrumentKind::BondFund)
                ) {
                    "Expense"
                } else {
                    "PER"
                },
                profile
                    .as_ref()
                    .and_then(|p| p.expense_ratio_pct)
                    .map(|v| format!("{v:.2}%"))
                    .or_else(|| d.pe_ratio.map(|v| format!("{v:.2}")))
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            (
                if matches!(
                    profile.as_ref().map(|p| p.kind),
                    Some(InstrumentKind::Etf | InstrumentKind::BondFund)
                ) {
                    "Yield"
                } else {
                    "EPS"
                },
                profile
                    .as_ref()
                    .and_then(|p| p.distribution_yield_pct)
                    .map(|v| format!("{v:.2}%"))
                    .or_else(|| d.eps.map(|v| format!("{v:.2}")))
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            (
                "Div Yield",
                d.dividend_yield
                    .map(|v| format!("{v:.2}%"))
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            (
                if let Some(profile) = &profile {
                    match profile.kind {
                        InstrumentKind::BondFund => "Duration",
                        InstrumentKind::Etf => "Benchmark",
                        InstrumentKind::Stock => {
                            if d.market == Market::Kr {
                                "PBR"
                            } else {
                                "Beta"
                            }
                        }
                    }
                } else if d.market == Market::Kr {
                    "PBR"
                } else {
                    "Beta"
                },
                if let Some(profile) = &profile {
                    match profile.kind {
                        InstrumentKind::BondFund => profile
                            .duration_years
                            .map(|v| format!("{v:.1}y"))
                            .unwrap_or_else(|| "N/A".to_string()),
                        InstrumentKind::Etf => profile
                            .benchmark
                            .clone()
                            .unwrap_or_else(|| "N/A".to_string()),
                        InstrumentKind::Stock => d
                            .beta
                            .map(|v| format!("{v:.2}"))
                            .unwrap_or_else(|| "N/A".to_string()),
                    }
                } else {
                    d.beta
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "N/A".to_string())
                },
            ),
            ("Volume", format_volume(d.volume)),
            ("Avg Vol", format_volume(d.avg_volume)),
        ]
    } else {
        vec![
            ("Market Cap", "N/A".to_string()),
            ("PER", "N/A".to_string()),
            ("EPS", "N/A".to_string()),
            ("Div Yield", "N/A".to_string()),
            ("Beta/PBR", "N/A".to_string()),
            ("Volume", "N/A".to_string()),
            ("Avg Vol", "N/A".to_string()),
        ]
    };
    for (idx, (label, value)) in cards.into_iter().enumerate() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{label}: "), Style::default().fg(theme::DIM)),
                Span::styled(value, Style::default().add_modifier(Modifier::BOLD)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::BORDER)),
            ),
            chunks[idx],
        );
    }
}

fn render_order_book(
    frame: &mut Frame,
    area: Rect,
    entries: &[OrderBookEntry],
    market: Market,
    symbol: Option<&str>,
) {
    let fmt_price = |price: f64| match market {
        Market::Kr => format_number(price, 0),
        Market::Us => format_number(price, 2),
    };
    let mut asks = entries.iter().filter(|e| !e.is_bid).collect::<Vec<_>>();
    asks.sort_by(|a, b| b.price.total_cmp(&a.price));
    let mut bids = entries.iter().filter(|e| e.is_bid).collect::<Vec<_>>();
    bids.sort_by(|a, b| b.price.total_cmp(&a.price));
    let rows = asks.into_iter().chain(bids).map(|entry| {
        let bar = "█".repeat(((entry.volume / 100) as usize).clamp(1, 20));
        if entry.is_bid {
            Row::new([
                Cell::from(""),
                Cell::from(fmt_price(entry.price)).style(Style::default().fg(theme::UP)),
                Cell::from(format!("{} {bar}", entry.volume)).style(Style::default().fg(theme::UP)),
            ])
        } else {
            Row::new([
                Cell::from(format!("{bar} {}", entry.volume))
                    .style(Style::default().fg(theme::DOWN)),
                Cell::from(fmt_price(entry.price)).style(Style::default().fg(theme::DOWN)),
                Cell::from(""),
            ])
        }
    });
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ],
    )
    .header(Row::new(["매도잔량", "가격", "매수잔량"]).style(Style::default().fg(theme::ACCENT)))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(if symbol.and_then(instrument_profile).is_some() {
                " Flow Depth (simulated) "
            } else {
                " 호가 "
            })
            .border_style(Style::default().fg(theme::BORDER)),
    );
    frame.render_widget(table, area);
}

fn render_info_and_investors(
    frame: &mut Frame,
    area: Rect,
    detail: Option<&StockDetail>,
    rows: &[InvestorRow],
) {
    let chunks = layout::vertical(area, &[Constraint::Length(4), Constraint::Min(5)]);
    let text = if let Some(d) = detail {
        let fmt = |v| match d.currency {
            Currency::Krw => format_number(v, 0),
            Currency::Usd => format!("${}", format_number(v, 2)),
        };
        vec![
            Line::from(format!(
                " Open {}  High {}  Low {}  Prev {}",
                fmt(d.open_price),
                fmt(d.high),
                fmt(d.low),
                fmt(d.prev_close)
            )),
            Line::from(format!(
                " Day Range {} - {}  Sector {}",
                fmt(d.low),
                fmt(d.high),
                if d.sector.is_empty() {
                    "N/A"
                } else {
                    &d.sector
                }
            )),
            Line::from(format!(
                " 52W Low {}  High {}  Position {:.0}%",
                fmt(d.week52_low),
                fmt(d.week52_high),
                d.week52_position() * 100.0
            )),
        ]
    } else {
        vec![Line::from("Loading price info...")]
    };
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Price Info ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        chunks[0],
    );
    let table_rows = rows.iter().map(|r| {
        Row::new([
            Cell::from(r.date.clone()),
            color_cell(r.individual),
            color_cell(r.foreign),
            color_cell(r.institution),
        ])
    });
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Length(8),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ],
        )
        .header(
            Row::new(["날짜", "개인", "외국인", "기관"]).style(Style::default().fg(theme::ACCENT)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 투자자 동향 ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        chunks[1],
    );
}

fn render_related_and_ai(
    frame: &mut Frame,
    area: Rect,
    detail: Option<&StockDetail>,
    indicators: &[crate::models::EconomicIndicator],
    state: &DetailState,
) {
    let mut lines = Vec::new();
    if let Some(detail) = detail {
        let symbols = config::related_indicators(&detail.sector);
        if !symbols.is_empty() {
            let mut spans = vec![Span::styled(
                "관련 지표  ",
                Style::default().add_modifier(Modifier::BOLD),
            )];
            for sym in symbols {
                if let Some(ind) = indicators.iter().find(|i| i.symbol == *sym) {
                    spans.push(Span::raw(format!(
                        "{} {} ",
                        ind.name,
                        ind.formatted_value()
                    )));
                    spans.push(Span::styled(
                        format!("{}  ", ind.formatted_change_pct()),
                        Style::default().fg(theme::change_color(ind.change_pct)),
                    ));
                }
            }
            lines.push(Line::from(spans));
        }
        lines.push(period_returns(detail));
    }
    if state.ai_loading {
        lines.push(Line::from(Span::styled(
            "AI 종목 분석 로딩중...",
            Style::default().fg(theme::ACCENT),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Insight ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        area,
    );
}

fn period_returns(detail: &StockDetail) -> Line<'static> {
    let periods = [
        ("1W", &detail.history_7d),
        ("1M", &detail.history_30d),
        ("3M", &detail.history_90d),
        ("1Y", &detail.history_1y),
    ];
    let mut spans = vec![Span::styled(
        "수익률: ",
        Style::default().add_modifier(Modifier::BOLD),
    )];
    for (label, hist) in periods {
        spans.push(Span::raw(format!("{label} ")));
        if let Some(first) = hist.first() {
            let ret = if *first != 0.0 {
                (detail.price - first) / first * 100.0
            } else {
                0.0
            };
            spans.push(Span::styled(
                format!("{ret:+.1}%  |  "),
                Style::default().fg(theme::change_color(ret)),
            ));
        } else {
            spans.push(Span::raw("N/A  |  "));
        }
    }
    Line::from(spans)
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let next = current as isize + delta;
    next.clamp(0, len as isize - 1) as usize
}

fn color_cell(value: i64) -> Cell<'static> {
    let text = if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    };
    Cell::from(text).style(Style::default().fg(if value >= 0 { theme::UP } else { theme::DOWN }))
}

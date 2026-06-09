use std::cmp::Ordering;

use chrono::Local;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    config::{self, CustomWatchlists},
    models::{ChartPeriod, EconomicIndicator, FxRate, MarketIndex, NewsItem, StockQuote},
    tasks::{scheduler, AppMsg},
    ui::{layout, theme, widgets},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardFocus {
    MarketList,
    News,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketTab {
    Us,
    Kr,
    Etf,
    Bond,
    Fx,
}

impl MarketTab {
    pub const ALL: [Self; 5] = [Self::Us, Self::Kr, Self::Etf, Self::Bond, Self::Fx];

    pub fn label(self) -> &'static str {
        match self {
            Self::Us => "US",
            Self::Kr => "KR",
            Self::Etf => "ETF",
            Self::Bond => "Bond",
            Self::Fx => "FX",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn index(self) -> usize {
        match self {
            Self::Us => 0,
            Self::Kr => 1,
            Self::Etf => 2,
            Self::Bond => 3,
            Self::Fx => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Symbol,
    Name,
    Price,
    ChangePct,
    Volume,
    MarketCap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxSortField {
    Name,
    Value,
    ChangePct,
}

impl SortField {
    pub const ALL: [Self; 6] = [
        Self::Symbol,
        Self::Name,
        Self::Price,
        Self::ChangePct,
        Self::Volume,
        Self::MarketCap,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Symbol => "Symbol",
            Self::Name => "Name",
            Self::Price => "Price",
            Self::ChangePct => "Change%",
            Self::Volume => "Volume",
            Self::MarketCap => "MktCap",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Asc => "↑",
            Self::Desc => "↓",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortCriterion {
    pub field: SortField,
    pub direction: SortDirection,
}

#[derive(Debug)]
pub struct DashboardState {
    pub generation: u64,
    pub news_generation: u64,
    pub watchlists: CustomWatchlists,
    pub us_indices: Vec<MarketIndex>,
    pub kr_indices: Vec<MarketIndex>,
    pub indicators: Vec<EconomicIndicator>,
    pub fx_rates: Vec<FxRate>,
    pub us_quotes: Vec<StockQuote>,
    pub kr_quotes: Vec<StockQuote>,
    pub etf_quotes: Vec<StockQuote>,
    pub bond_quotes: Vec<StockQuote>,
    pub news: Vec<NewsItem>,
    pub us_selected: usize,
    pub kr_selected: usize,
    pub etf_selected: usize,
    pub bond_selected: usize,
    pub fx_selected: usize,
    pub news_selected: usize,
    pub focus: DashboardFocus,
    pub active_tab: MarketTab,
    pub status: String,
    pub search_mode: bool,
    pub add_symbol_mode: bool,
    pub add_symbol_input: String,
    pub market_searches: [String; 4],
    pub news_search: String,
    pub market_sorts: [Vec<SortCriterion>; 4],
    pub fx_period: ChartPeriod,
    pub fx_base_currency: String,
    pub fx_sort_field: FxSortField,
    pub fx_sort_direction: SortDirection,
    pub heatmap_view: bool,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            generation: 0,
            news_generation: 0,
            watchlists: config::load_custom_watchlists(),
            us_indices: Vec::new(),
            kr_indices: Vec::new(),
            indicators: Vec::new(),
            fx_rates: Vec::new(),
            us_quotes: Vec::new(),
            kr_quotes: Vec::new(),
            etf_quotes: Vec::new(),
            bond_quotes: Vec::new(),
            news: Vec::new(),
            us_selected: 0,
            kr_selected: 0,
            etf_selected: 0,
            bond_selected: 0,
            fx_selected: 0,
            news_selected: 0,
            focus: DashboardFocus::MarketList,
            active_tab: MarketTab::Us,
            status: "Loading data...".to_string(),
            search_mode: false,
            add_symbol_mode: false,
            add_symbol_input: String::new(),
            market_searches: [String::new(), String::new(), String::new(), String::new()],
            news_search: String::new(),
            market_sorts: default_market_sorts(),
            fx_period: ChartPeriod::ThreeMonths,
            fx_base_currency: "KRW".to_string(),
            fx_sort_field: FxSortField::ChangePct,
            fx_sort_direction: SortDirection::Desc,
            heatmap_view: false,
        }
    }

    pub fn refresh(&mut self, tx: UnboundedSender<AppMsg>) {
        self.generation = self.generation.wrapping_add(1);
        self.status = "Refreshing market data...".to_string();
        scheduler::spawn_dashboard_refresh(
            tx,
            self.generation,
            config::merged_us_symbols(&self.watchlists),
            config::merged_kr_symbols(&self.watchlists),
            config::merged_etf_symbols(&self.watchlists),
            config::merged_bond_symbols(&self.watchlists),
        );
    }

    pub fn refresh_news(&mut self, tx: UnboundedSender<AppMsg>) {
        self.news_generation = self.news_generation.wrapping_add(1);
        scheduler::spawn_news_refresh(tx, self.news_generation);
    }

    pub fn apply_msg(&mut self, msg: &AppMsg) {
        match msg {
            AppMsg::DashboardIndicesLoaded { generation, us, kr }
                if *generation == self.generation =>
            {
                self.us_indices = us.clone();
                self.kr_indices = kr.clone();
                self.status = "Wave 1/4: indices loaded".to_string();
            }
            AppMsg::DashboardIndicatorsLoaded {
                generation,
                indicators,
                fx_rates,
            } if *generation == self.generation => {
                self.indicators = indicators.clone();
                self.fx_rates = fx_rates.clone();
                self.status = "Wave 2/4: indicators loaded".to_string();
            }
            AppMsg::DashboardQuotesLoaded {
                generation,
                us,
                kr,
                etfs,
                bonds,
            } if *generation == self.generation => {
                self.us_quotes = us.clone();
                self.kr_quotes = kr.clone();
                self.etf_quotes = etfs.clone();
                self.bond_quotes = bonds.clone();
                self.status = format!(
                    "Updated: {} | loading market caps...",
                    Local::now().format("%H:%M:%S")
                );
            }
            AppMsg::DashboardMarketCapsLoaded {
                generation,
                us,
                kr,
                etfs,
                bonds,
            } if *generation == self.generation => {
                for quote in &mut self.us_quotes {
                    if let Some(cap) = us.get(&quote.symbol) {
                        quote.market_cap = *cap;
                    }
                }
                for quote in &mut self.kr_quotes {
                    if let Some(cap) = kr.get(&quote.symbol) {
                        quote.market_cap = *cap;
                    }
                }
                for quote in &mut self.etf_quotes {
                    if let Some(cap) = etfs.get(&quote.symbol) {
                        quote.market_cap = *cap;
                    }
                }
                for quote in &mut self.bond_quotes {
                    if let Some(cap) = bonds.get(&quote.symbol) {
                        quote.market_cap = *cap;
                    }
                }
                self.status = format!(
                    "Updated: {} | Next refresh in {}s",
                    Local::now().format("%H:%M:%S"),
                    crate::config::REFRESH_INTERVAL_SECS
                );
            }
            AppMsg::NewsLoaded { generation, news } if *generation == self.news_generation => {
                self.news = news.clone();
                self.clamp_selection();
            }
            _ => {}
        }
        self.clamp_selection();
    }

    pub fn selected_stock(&self) -> Option<StockQuote> {
        if self.active_tab == MarketTab::Fx {
            return None;
        }
        self.current_visible_quotes()
            .get(self.current_selected_index())
            .cloned()
    }

    pub fn selected_news(&self) -> Option<NewsItem> {
        self.visible_news().get(self.news_selected).cloned()
    }

    pub fn next_fx_base_currency(&mut self) {
        self.fx_base_currency = match self.fx_base_currency.as_str() {
            "KRW" => "USD".to_string(),
            "USD" => "EUR".to_string(),
            "EUR" => "JPY".to_string(),
            _ => "KRW".to_string(),
        };
        self.fx_selected = 0;
        self.clamp_selection();
    }

    pub fn cycle_fx_sort(&mut self) {
        self.fx_sort_field = match self.fx_sort_field {
            FxSortField::Name => FxSortField::Value,
            FxSortField::Value => FxSortField::ChangePct,
            FxSortField::ChangePct => FxSortField::Name,
        };
    }

    pub fn toggle_fx_sort_direction(&mut self) {
        self.fx_sort_direction = self.fx_sort_direction.toggle();
    }

    pub fn visible_fx_rates(&self) -> Vec<FxRate> {
        self.fx_rates
            .iter()
            .filter(|rate| rate.quote_currency == self.fx_base_currency)
            .cloned()
            .collect()
    }

    pub fn sorted_fx_rates(&self) -> Vec<FxRate> {
        let mut rates = self.visible_fx_rates();
        rates.sort_by(|left, right| {
            let ordering = match self.fx_sort_field {
                FxSortField::Name => left.base_currency.cmp(&right.base_currency),
                FxSortField::Value => left.value.total_cmp(&right.value),
                FxSortField::ChangePct => left.change_pct.total_cmp(&right.change_pct),
            };
            match self.fx_sort_direction {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
            }
        });
        rates
    }

    pub fn selected_fx_rate(&self) -> Option<FxRate> {
        self.sorted_fx_rates().get(self.fx_selected).cloned()
    }

    pub fn fx_sort_label(&self) -> String {
        let field = match self.fx_sort_field {
            FxSortField::Name => "Name",
            FxSortField::Value => self.fx_base_currency.as_str(),
            FxSortField::ChangePct => "Move",
        };
        format!("{field} {}", self.fx_sort_direction.symbol())
    }

    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            DashboardFocus::MarketList => DashboardFocus::News,
            DashboardFocus::News => DashboardFocus::MarketList,
        };
        self.clamp_selection();
    }

    pub fn focus_prev(&mut self) {
        self.focus_next();
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.clamp_selection();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.clamp_selection();
    }

    pub fn toggle_heatmap(&mut self) {
        self.heatmap_view = !self.heatmap_view;
    }

    pub fn move_selection(&mut self, delta: isize) {
        match self.focus {
            DashboardFocus::MarketList => match self.active_tab {
                MarketTab::Us => {
                    self.us_selected =
                        move_index(self.us_selected, self.current_visible_quotes().len(), delta)
                }
                MarketTab::Kr => {
                    self.kr_selected =
                        move_index(self.kr_selected, self.current_visible_quotes().len(), delta)
                }
                MarketTab::Etf => {
                    self.etf_selected = move_index(
                        self.etf_selected,
                        self.current_visible_quotes().len(),
                        delta,
                    )
                }
                MarketTab::Bond => {
                    self.bond_selected = move_index(
                        self.bond_selected,
                        self.current_visible_quotes().len(),
                        delta,
                    )
                }
                MarketTab::Fx => {
                    self.fx_selected =
                        move_index(self.fx_selected, self.sorted_fx_rates().len(), delta)
                }
            },
            DashboardFocus::News => {
                self.news_selected =
                    move_index(self.news_selected, self.visible_news().len(), delta)
            }
        }
    }

    pub fn jump_selection_start(&mut self) {
        match self.focus {
            DashboardFocus::MarketList => match self.active_tab {
                MarketTab::Us => self.us_selected = 0,
                MarketTab::Kr => self.kr_selected = 0,
                MarketTab::Etf => self.etf_selected = 0,
                MarketTab::Bond => self.bond_selected = 0,
                MarketTab::Fx => self.fx_selected = 0,
            },
            DashboardFocus::News => self.news_selected = 0,
        }
    }

    pub fn jump_selection_end(&mut self) {
        match self.focus {
            DashboardFocus::MarketList => {
                let last = self.current_visible_quotes().len().saturating_sub(1);
                match self.active_tab {
                    MarketTab::Us => self.us_selected = last,
                    MarketTab::Kr => self.kr_selected = last,
                    MarketTab::Etf => self.etf_selected = last,
                    MarketTab::Bond => self.bond_selected = last,
                    MarketTab::Fx => {
                        self.fx_selected = self.sorted_fx_rates().len().saturating_sub(1)
                    }
                }
            }
            DashboardFocus::News => {
                self.news_selected = self.visible_news().len().saturating_sub(1)
            }
        }
    }

    pub fn page_selection(&mut self, delta: isize) {
        self.move_selection(delta);
    }

    pub fn begin_search(&mut self) {
        self.search_mode = true;
    }

    pub fn push_search_char(&mut self, character: char) {
        match self.focus {
            DashboardFocus::MarketList => {
                if let Some(query) = self.active_market_search_mut() {
                    query.push(character);
                }
            }
            DashboardFocus::News => self.news_search.push(character),
        }
        self.clamp_selection();
    }

    pub fn pop_search_char(&mut self) {
        match self.focus {
            DashboardFocus::MarketList => {
                if let Some(query) = self.active_market_search_mut() {
                    query.pop();
                }
            }
            DashboardFocus::News => {
                self.news_search.pop();
            }
        }
        self.clamp_selection();
    }

    pub fn end_search(&mut self) {
        self.search_mode = false;
        self.clamp_selection();
    }

    pub fn begin_add_symbol(&mut self) {
        self.add_symbol_mode = true;
        self.add_symbol_input.clear();
    }

    pub fn push_add_symbol_char(&mut self, character: char) {
        self.add_symbol_input.push(character);
    }

    pub fn pop_add_symbol_char(&mut self) {
        self.add_symbol_input.pop();
    }

    pub fn cancel_add_symbol(&mut self) {
        self.add_symbol_mode = false;
        self.add_symbol_input.clear();
    }

    pub fn commit_add_symbol(&mut self) -> Option<String> {
        let symbol = self.add_symbol_input.trim().to_uppercase();
        self.add_symbol_mode = false;
        self.add_symbol_input.clear();
        if symbol.is_empty() {
            return None;
        }
        let target = match self.active_tab {
            MarketTab::Us => &mut self.watchlists.us,
            MarketTab::Kr => &mut self.watchlists.kr,
            MarketTab::Etf => &mut self.watchlists.etf,
            MarketTab::Bond => &mut self.watchlists.bond,
            MarketTab::Fx => return None,
        };
        if !target
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&symbol))
        {
            target.push(symbol.clone());
            target.sort();
            let _ = config::save_custom_watchlists(&self.watchlists);
        }
        Some(symbol)
    }

    pub fn active_focus_has_filter(&self) -> bool {
        match self.focus {
            DashboardFocus::MarketList => self
                .active_market_search_ref()
                .map(|query| !query.is_empty())
                .unwrap_or(false),
            DashboardFocus::News => !self.news_search.is_empty(),
        }
    }

    pub fn clear_active_filter(&mut self) {
        match self.focus {
            DashboardFocus::MarketList => {
                if let Some(query) = self.active_market_search_mut() {
                    query.clear();
                }
            }
            DashboardFocus::News => self.news_search.clear(),
        }
        self.search_mode = false;
        self.clamp_selection();
    }

    pub fn cycle_sort_field(&mut self) {
        let Some(current_sort) = self.active_market_sort_ref() else {
            return;
        };
        let next = current_sort
            .first()
            .map(|criterion| criterion.field.next())
            .unwrap_or(SortField::ChangePct);
        let direction = current_sort
            .first()
            .map(|criterion| criterion.direction)
            .unwrap_or(SortDirection::Desc);
        if let Some(sort) = self.active_market_sort_mut() {
            *sort = vec![
                SortCriterion {
                    field: next,
                    direction,
                },
                SortCriterion {
                    field: SortField::MarketCap,
                    direction: SortDirection::Desc,
                },
                SortCriterion {
                    field: SortField::Symbol,
                    direction: SortDirection::Asc,
                },
            ];
        }
        self.clamp_selection();
    }

    pub fn toggle_sort_direction(&mut self) {
        if let Some(first) = self
            .active_market_sort_mut()
            .and_then(|sort| sort.first_mut())
        {
            first.direction = first.direction.toggle();
        }
        self.clamp_selection();
    }

    pub fn current_sort_label(&self) -> String {
        self.active_market_sort_ref()
            .and_then(|sort| sort.first())
            .map(|criterion| {
                format!(
                    "{} {}",
                    criterion.field.label(),
                    criterion.direction.symbol()
                )
            })
            .unwrap_or_else(|| "Default".to_string())
    }

    pub fn market_search_label(&self) -> String {
        let Some(query) = self.active_market_search_ref() else {
            return "-".to_string();
        };
        if query.is_empty() {
            "-".to_string()
        } else {
            query.clone()
        }
    }

    pub fn news_search_label(&self) -> String {
        if self.news_search.is_empty() {
            "-".to_string()
        } else {
            self.news_search.clone()
        }
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

    pub fn current_visible_quotes(&self) -> Vec<StockQuote> {
        let raw = match self.active_tab {
            MarketTab::Us => &self.us_quotes,
            MarketTab::Kr => &self.kr_quotes,
            MarketTab::Etf => &self.etf_quotes,
            MarketTab::Bond => &self.bond_quotes,
            MarketTab::Fx => return Vec::new(),
        };
        let query = self.active_market_search();
        let terms = query.split_whitespace().collect::<Vec<_>>();
        let mut filtered = raw
            .iter()
            .filter(|quote| {
                if terms.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {} {}",
                    quote.symbol,
                    quote.name,
                    quote.sector,
                    quote.tags.join(" ")
                )
                .to_lowercase();
                terms.iter().all(|term| {
                    if let Some(tag) = term.strip_prefix("tag:") {
                        quote
                            .tags
                            .iter()
                            .any(|existing| existing.eq_ignore_ascii_case(tag))
                    } else {
                        haystack.contains(term)
                    }
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        filtered.sort_by(|left, right| {
            compare_quotes(left, right, self.active_market_sort_ref().unwrap_or(&[]))
        });
        filtered
    }

    pub fn tab_titles(&self) -> Vec<Line<'static>> {
        MarketTab::ALL
            .iter()
            .map(|tab| {
                let count = self.quote_count(*tab);
                Line::from(format!("{} ({count})", tab.label()))
            })
            .collect()
    }

    pub fn active_tab_title(&self) -> &'static str {
        match self.active_tab {
            MarketTab::Us => " US Stocks ",
            MarketTab::Kr => " KR Stocks ",
            MarketTab::Etf => " Major ETFs ",
            MarketTab::Bond => " Bonds ",
            MarketTab::Fx => " FX ",
        }
    }

    fn active_market_search_ref(&self) -> Option<&String> {
        match self.active_tab {
            MarketTab::Us => Some(&self.market_searches[0]),
            MarketTab::Kr => Some(&self.market_searches[1]),
            MarketTab::Etf => Some(&self.market_searches[2]),
            MarketTab::Bond => Some(&self.market_searches[3]),
            MarketTab::Fx => None,
        }
    }

    fn active_market_search_mut(&mut self) -> Option<&mut String> {
        match self.active_tab {
            MarketTab::Us => Some(&mut self.market_searches[0]),
            MarketTab::Kr => Some(&mut self.market_searches[1]),
            MarketTab::Etf => Some(&mut self.market_searches[2]),
            MarketTab::Bond => Some(&mut self.market_searches[3]),
            MarketTab::Fx => None,
        }
    }

    fn active_market_search(&self) -> String {
        self.active_market_search_ref()
            .map(|query| query.trim().to_lowercase())
            .unwrap_or_default()
    }

    fn active_market_sort_ref(&self) -> Option<&[SortCriterion]> {
        match self.active_tab {
            MarketTab::Us => Some(&self.market_sorts[0]),
            MarketTab::Kr => Some(&self.market_sorts[1]),
            MarketTab::Etf => Some(&self.market_sorts[2]),
            MarketTab::Bond => Some(&self.market_sorts[3]),
            MarketTab::Fx => None,
        }
        .map(Vec::as_slice)
    }

    fn active_market_sort_mut(&mut self) -> Option<&mut Vec<SortCriterion>> {
        match self.active_tab {
            MarketTab::Us => Some(&mut self.market_sorts[0]),
            MarketTab::Kr => Some(&mut self.market_sorts[1]),
            MarketTab::Etf => Some(&mut self.market_sorts[2]),
            MarketTab::Bond => Some(&mut self.market_sorts[3]),
            MarketTab::Fx => None,
        }
    }

    fn quote_count(&self, tab: MarketTab) -> usize {
        match tab {
            MarketTab::Us => self.us_quotes.len(),
            MarketTab::Kr => self.kr_quotes.len(),
            MarketTab::Etf => self.etf_quotes.len(),
            MarketTab::Bond => self.bond_quotes.len(),
            MarketTab::Fx => self.fx_rates.len(),
        }
    }

    fn current_selected_index(&self) -> usize {
        match self.active_tab {
            MarketTab::Us => self.us_selected,
            MarketTab::Kr => self.kr_selected,
            MarketTab::Etf => self.etf_selected,
            MarketTab::Bond => self.bond_selected,
            MarketTab::Fx => self.fx_selected,
        }
    }

    fn clamp_selection(&mut self) {
        let visible_market_len = self.current_visible_quotes().len();
        match self.active_tab {
            MarketTab::Us => {
                self.us_selected = self.us_selected.min(visible_market_len.saturating_sub(1))
            }
            MarketTab::Kr => {
                self.kr_selected = self.kr_selected.min(visible_market_len.saturating_sub(1))
            }
            MarketTab::Etf => {
                self.etf_selected = self.etf_selected.min(visible_market_len.saturating_sub(1))
            }
            MarketTab::Bond => {
                self.bond_selected = self.bond_selected.min(visible_market_len.saturating_sub(1))
            }
            MarketTab::Fx => {
                self.fx_selected = self
                    .fx_selected
                    .min(self.sorted_fx_rates().len().saturating_sub(1))
            }
        }
        self.news_selected = self
            .news_selected
            .min(self.visible_news().len().saturating_sub(1));
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &DashboardState) {
    // Heights of the stacked panels above the market table. Some are gated on
    // terminal height so cramped terminals drop the optional context rows.
    let summary_h = if area.height >= 28 { 3 } else { 0 };
    let breadth_h = if area.height >= 31 { 4 } else { 0 };
    let sector_h = if area.height >= 35 { 3 } else { 0 };
    // title 1 + indicators 3 + fx 5 + indices 4 + tabs 3 + status 1 = 17 fixed.
    let fixed_h = 17 + summary_h + breadth_h + sector_h;
    let body = area.height.saturating_sub(fixed_h).max(15);
    let market_height = (body.saturating_mul(3) / 5).max(10);
    let news_height = body.saturating_sub(market_height).max(5);
    let outer = layout::vertical(
        area,
        &[
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Length(summary_h),
            Constraint::Length(breadth_h),
            Constraint::Length(sector_h),
            Constraint::Length(3),
            Constraint::Length(market_height),
            Constraint::Min(news_height),
            Constraint::Length(1),
        ],
    );
    frame.render_widget(title(), outer[0]);
    widgets::indicator_bar::render(frame, outer[1], &state.indicators);
    widgets::fx_board::render(
        frame,
        outer[2],
        &state.sorted_fx_rates(),
        state.fx_period,
        &fx_context_lines(state),
        &state.fx_sort_label(),
    );
    render_indices(frame, outer[3], state);
    widgets::market_summary::render(frame, outer[4], &state.us_quotes, &state.kr_quotes);
    widgets::breadth_bar::render(frame, outer[5], &state.us_quotes, &state.kr_quotes);
    widgets::sector_bar::render(frame, outer[6], &state.us_quotes, &state.kr_quotes);
    render_market_tabs(frame, outer[7], state);
    if state.active_tab == MarketTab::Fx {
        let fx_split = layout::horizontal(
            outer[8],
            &[Constraint::Percentage(38), Constraint::Percentage(62)],
        );
        let fx_rates = state.sorted_fx_rates();
        widgets::fx_table::render(
            frame,
            fx_split[0],
            state.active_tab_title(),
            &fx_rates,
            state.fx_selected,
            state.focus == DashboardFocus::MarketList,
            state.fx_period,
        );
        widgets::fx_detail::render(
            frame,
            fx_split[1],
            state.selected_fx_rate().as_ref(),
            state.fx_period,
        );
    } else {
        let visible_quotes = state.current_visible_quotes();
        if state.heatmap_view {
            widgets::heatmap::render(
                frame,
                outer[8],
                state.active_tab_title(),
                &visible_quotes,
                state.focus == DashboardFocus::MarketList,
            );
        } else {
            widgets::stock_table::render(
                frame,
                outer[8],
                state.active_tab_title(),
                &visible_quotes,
                state.current_selected_index(),
                state.focus == DashboardFocus::MarketList,
            );
        }
    }
    widgets::news_feed::render(
        frame,
        outer[9],
        &state.visible_news(),
        state.news_selected,
        state.focus == DashboardFocus::News,
        " News Feed ",
    );
    frame.render_widget(
        Paragraph::new(state.status.clone()).style(Style::default().fg(theme::DIM)),
        outer[10],
    );
}

fn render_indices(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let chunks = layout::horizontal(
        area,
        &[
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    );
    let find = |symbol: &str| {
        state
            .us_indices
            .iter()
            .chain(state.kr_indices.iter())
            .find(|idx| idx.symbol == symbol)
    };
    widgets::market_card::render(frame, chunks[0], "S&P 500", find("^GSPC"));
    widgets::market_card::render(frame, chunks[1], "NASDAQ", find("^IXIC"));
    widgets::market_card::render(frame, chunks[2], "DOW", find("^DJI"));
    widgets::market_card::render(frame, chunks[3], "KOSPI", find("^KS11"));
    widgets::market_card::render(frame, chunks[4], "KOSDAQ", find("^KQ11"));
}

fn render_market_tabs(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let chunks = layout::horizontal(
        area,
        &[Constraint::Percentage(60), Constraint::Percentage(40)],
    );
    let border = if state.focus == DashboardFocus::MarketList {
        theme::ACCENT
    } else {
        theme::BORDER
    };
    let tabs = Tabs::new(state.tab_titles())
        .select(state.active_tab.index())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Markets ")
                .border_style(Style::default().fg(border)),
        )
        .highlight_style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);
    let search_target = match state.focus {
        DashboardFocus::MarketList => format!("Filter /{}", state.market_search_label()),
        DashboardFocus::News => format!("News /{}", state.news_search_label()),
    };
    let hint = if state.add_symbol_mode {
        format!(
            "Add {} symbol: {}",
            state.active_tab.label(),
            state.add_symbol_input
        )
    } else {
        format!(
            "Sort {}  |  {}  |  / search  |  + add {}  |  [m] {}",
            state.current_sort_label(),
            search_target,
            state.active_tab.label(),
            if state.heatmap_view {
                "table"
            } else {
                "heatmap"
            },
        )
    };
    frame.render_widget(
        Paragraph::new(hint).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" View ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        chunks[1],
    );
}

fn fx_context_lines(state: &DashboardState) -> Vec<String> {
    let mut lines = vec![format!(
        "Period {}  |  Sort {}  |  Base {}  |  [1]1W [2]1M [3]3M [4]1Y  [B] base  [V] sort  [Shift+V] reverse",
        state.fx_period.label(),
        state.fx_sort_label(),
        state.fx_base_currency
    )];

    let exporters = ["Electronics", "Components", "Auto", "Shipbuilding"];
    let importers = ["Energy", "Airline", "Travel", "Utilities"];
    let exporter_count = state
        .kr_quotes
        .iter()
        .filter(|quote| exporters.contains(&quote.sector.as_str()))
        .count();
    let importer_count = state
        .kr_quotes
        .iter()
        .filter(|quote| importers.contains(&quote.sector.as_str()))
        .count();

    lines.push(format!(
        "KRW 약세(USD/EUR↑) 수혜 가능: 전기전자·부품·자동차·조선 ({exporter_count}개 종목)"
    ));
    lines.push(format!(
        "KRW 강세 / 원자재·해외결제 민감: 에너지·항공·여행·유틸리티 ({importer_count}개 종목)"
    ));
    lines
}

fn title() -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled(
            " tickboard ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("US, KR, ETF & Bond Markets  "),
        Span::styled(
            "[Q] Quit [?] Help [R] Refresh [Tab] Pane [H/L] Tabs [/] Search [S] Sort",
            Style::default().fg(theme::DIM),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme::BORDER)),
    )
}

fn default_market_sorts() -> [Vec<SortCriterion>; 4] {
    std::array::from_fn(|_| {
        vec![
            SortCriterion {
                field: SortField::ChangePct,
                direction: SortDirection::Desc,
            },
            SortCriterion {
                field: SortField::MarketCap,
                direction: SortDirection::Desc,
            },
            SortCriterion {
                field: SortField::Symbol,
                direction: SortDirection::Asc,
            },
        ]
    })
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let next = current as isize + delta;
    next.clamp(0, len as isize - 1) as usize
}

fn compare_quotes(left: &StockQuote, right: &StockQuote, criteria: &[SortCriterion]) -> Ordering {
    for criterion in criteria {
        let ordering = match criterion.field {
            SortField::Symbol => left.symbol.cmp(&right.symbol),
            SortField::Name => left.name.cmp(&right.name),
            SortField::Price => left.price.total_cmp(&right.price),
            SortField::ChangePct => left.change_pct.total_cmp(&right.change_pct),
            SortField::Volume => left.volume.cmp(&right.volume),
            SortField::MarketCap => left.market_cap.total_cmp(&right.market_cap),
        };
        let ordering = match criterion.direction {
            SortDirection::Asc => ordering,
            SortDirection::Desc => ordering.reverse(),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

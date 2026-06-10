use std::{io, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    config,
    models::{InstrumentKind, InstrumentProfile, NewsItem, StockQuote},
    tasks::AppMsg,
    ui::{
        screens::{self, dashboard::DashboardFocus, ArticleState, DashboardState, DetailState},
        theme,
    },
};

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Screen {
    Dashboard(DashboardState),
    Detail(DetailState),
    Article(ArticleState),
}

pub struct App {
    screens: Vec<Screen>,
    running: bool,
    tx: UnboundedSender<AppMsg>,
    rx: UnboundedReceiver<AppMsg>,
    last_indicators: Vec<crate::models::EconomicIndicator>,
    show_shortcuts: bool,
    inspector_profile: Option<InstrumentProfile>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut dashboard = DashboardState::new();
        dashboard.refresh(tx.clone());
        dashboard.refresh_news(tx.clone());
        Self {
            screens: vec![Screen::Dashboard(dashboard)],
            running: true,
            tx,
            rx,
            last_indicators: Vec::new(),
            show_shortcuts: false,
            inspector_profile: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut terminal = setup_terminal()?;
        let result = self.event_loop(&mut terminal).await;
        restore_terminal(&mut terminal)?;
        result
    }

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        let mut events = EventStream::new();
        let mut stock_interval =
            tokio::time::interval(Duration::from_secs(config::REFRESH_INTERVAL_SECS));
        let mut news_interval =
            tokio::time::interval(Duration::from_secs(config::NEWS_REFRESH_INTERVAL_SECS));
        stock_interval.tick().await;
        news_interval.tick().await;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            tokio::select! {
                _ = stock_interval.tick() => self.refresh_dashboard(),
                _ = news_interval.tick() => self.refresh_news(),
                Some(msg) = self.rx.recv() => self.apply_msg(msg),
                Some(Ok(event)) = events.next() => {
                    if let Event::Key(key) = event {
                        self.handle_key(key);
                    }
                }
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        match self.screens.last() {
            Some(Screen::Dashboard(state)) => screens::dashboard::render(frame, area, state),
            Some(Screen::Detail(state)) => {
                screens::detail::render(frame, area, state, &self.last_indicators)
            }
            Some(Screen::Article(state)) => screens::article::render(frame, area, state),
            None => {}
        }
        if self.show_shortcuts {
            render_shortcuts_modal(frame, area, self.screens.last());
        }
        if let Some(profile) = &self.inspector_profile {
            render_inspector_modal(frame, area, profile);
        }
    }

    fn apply_msg(&mut self, msg: AppMsg) {
        if let AppMsg::DashboardIndicatorsLoaded {
            generation: _,
            indicators,
            ..
        } = &msg
        {
            if !indicators.is_empty() {
                self.last_indicators = indicators.clone();
            }
        }
        for screen in &mut self.screens {
            match screen {
                Screen::Dashboard(state) => state.apply_msg(&msg),
                Screen::Detail(state) => state.apply_msg(&msg),
                Screen::Article(state) => state.apply_msg(&msg),
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.show_shortcuts {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => self.show_shortcuts = false,
                _ => {}
            }
            return;
        }
        if self.inspector_profile.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('i') => self.inspector_profile = None,
                _ => {}
            }
            return;
        }
        if matches!(key.code, KeyCode::Char('?')) {
            self.show_shortcuts = true;
            return;
        }
        let mut push_detail_stock = None;
        let mut push_article_news = None;
        let mut request_inspector_symbol: Option<String> = None;
        let mut request_ai_toggle = false;
        let mut pop_screen = false;
        match self.screens.last_mut() {
            Some(Screen::Dashboard(state)) => {
                if state.add_symbol_mode {
                    match key.code {
                        KeyCode::Esc => {
                            state.cancel_add_symbol();
                            return;
                        }
                        KeyCode::Enter => {
                            let _ = state.commit_add_symbol();
                            self.refresh_dashboard();
                            return;
                        }
                        KeyCode::Backspace => {
                            state.pop_add_symbol_char();
                            return;
                        }
                        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            state.push_add_symbol_char(ch);
                            return;
                        }
                        _ => return,
                    }
                }
                if state.search_mode {
                    match key.code {
                        KeyCode::Esc => state.clear_active_filter(),
                        KeyCode::Enter => state.end_search(),
                        KeyCode::Backspace => state.pop_search_char(),
                        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            state.push_search_char(ch)
                        }
                        _ => {}
                    }
                    return;
                }
                match key.code {
                    KeyCode::Esc if state.active_focus_has_filter() => state.clear_active_filter(),
                    KeyCode::Char('r') => {
                        state.refresh(self.tx.clone());
                        state.refresh_news(self.tx.clone());
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => state.begin_add_symbol(),
                    KeyCode::Char('/') => state.begin_search(),
                    KeyCode::Char('m') => state.toggle_heatmap(),
                    KeyCode::Char('s') => state.cycle_sort_field(),
                    KeyCode::Char('S') => state.toggle_sort_direction(),
                    KeyCode::Char('1') => state.fx_period = crate::models::ChartPeriod::OneWeek,
                    KeyCode::Char('2') => state.fx_period = crate::models::ChartPeriod::OneMonth,
                    KeyCode::Char('3') => state.fx_period = crate::models::ChartPeriod::ThreeMonths,
                    KeyCode::Char('4') => state.fx_period = crate::models::ChartPeriod::OneYear,
                    KeyCode::Char('b') => state.next_fx_base_currency(),
                    KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        state.toggle_fx_sort_direction()
                    }
                    KeyCode::Char('v') => state.cycle_fx_sort(),
                    KeyCode::Char('i') => {
                        request_inspector_symbol = state.selected_stock().map(|stock| stock.symbol);
                    }
                    KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        state.jump_selection_end()
                    }
                    KeyCode::Char('g') if key.modifiers.is_empty() => state.jump_selection_start(),
                    KeyCode::Home => state.jump_selection_start(),
                    KeyCode::End => state.jump_selection_end(),
                    KeyCode::PageUp => state.page_selection(-8),
                    KeyCode::PageDown => state.page_selection(8),
                    KeyCode::BackTab => state.focus_prev(),
                    KeyCode::Tab => state.focus_next(),
                    KeyCode::Down | KeyCode::Char('j') => state.move_selection(1),
                    KeyCode::Up | KeyCode::Char('k') => state.move_selection(-1),
                    KeyCode::Left | KeyCode::Char('h') => match state.focus {
                        DashboardFocus::MarketList => state.prev_tab(),
                        DashboardFocus::News => state.focus_prev(),
                    },
                    KeyCode::Right | KeyCode::Char('l') => match state.focus {
                        DashboardFocus::MarketList => state.next_tab(),
                        DashboardFocus::News => state.focus_next(),
                    },
                    KeyCode::Enter => match state.focus {
                        DashboardFocus::MarketList => {
                            push_detail_stock = state.selected_stock();
                        }
                        DashboardFocus::News => {
                            push_article_news = state.selected_news();
                        }
                    },
                    _ => {}
                }
            }
            Some(Screen::Detail(state)) => {
                if state.news_search_mode {
                    match key.code {
                        KeyCode::Esc => state.clear_news_filter(),
                        KeyCode::Enter => state.end_news_search(),
                        KeyCode::Backspace => state.pop_news_search_char(),
                        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            state.push_news_search_char(ch)
                        }
                        _ => {}
                    }
                    return;
                }
                match key.code {
                    KeyCode::Esc if state.has_active_news_filter() => state.clear_news_filter(),
                    KeyCode::Esc | KeyCode::Char('b') => pop_screen = true,
                    KeyCode::Char('/') => state.begin_news_search(),
                    KeyCode::Char('r') => state.refresh(self.tx.clone()),
                    KeyCode::Char('p') | KeyCode::Right => state.period = state.period.next(),
                    KeyCode::Left => state.period = state.period.prev(),
                    KeyCode::Char('1') => state.period = crate::models::ChartPeriod::OneWeek,
                    KeyCode::Char('2') => state.period = crate::models::ChartPeriod::OneMonth,
                    KeyCode::Char('3') => state.period = crate::models::ChartPeriod::ThreeMonths,
                    KeyCode::Char('4') => state.period = crate::models::ChartPeriod::OneYear,
                    KeyCode::Char('a') => request_ai_toggle = true,
                    KeyCode::Char('g') if key.modifiers.is_empty() => state.jump_news_start(),
                    KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        state.jump_news_end()
                    }
                    KeyCode::Home => state.jump_news_start(),
                    KeyCode::End => state.jump_news_end(),
                    KeyCode::PageUp => state.page_news(-8),
                    KeyCode::PageDown => state.page_news(8),
                    KeyCode::Down | KeyCode::Char('j') => state.move_news(1),
                    KeyCode::Up | KeyCode::Char('k') => state.move_news(-1),
                    KeyCode::Enter => push_article_news = state.selected_news(),
                    _ => {}
                }
            }
            Some(Screen::Article(state)) => match key.code {
                KeyCode::Esc | KeyCode::Char('b') => pop_screen = true,
                KeyCode::Char('r') => state.refresh(self.tx.clone()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    pop_screen = true
                }
                _ => {}
            },
            None => self.running = false,
        }
        if key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
            self.running = false;
            return;
        }
        if pop_screen && self.screens.len() > 1 {
            self.screens.pop();
        }
        if let Some(symbol) = request_inspector_symbol {
            self.inspector_profile = config::instrument_profile(&symbol).or_else(|| {
                self.screens.last().and_then(|screen| match screen {
                    Screen::Dashboard(state) => state.selected_stock().map(|stock| {
                        config::generic_instrument_profile(&stock.symbol, stock.market)
                    }),
                    _ => None,
                })
            });
        }
        if let Some(stock) = push_detail_stock {
            push_detail(&mut self.screens, &self.tx, stock);
        }
        if let Some(news) = push_article_news {
            push_article(&mut self.screens, &self.tx, news);
        }
        if request_ai_toggle {
            let peers = self.detail_peers();
            if let Some(Screen::Detail(state)) = self.screens.last_mut() {
                state.toggle_ai(self.tx.clone(), peers);
            }
        }
    }

    fn refresh_dashboard(&mut self) {
        if let Some(Screen::Dashboard(state)) = self
            .screens
            .iter_mut()
            .find(|screen| matches!(screen, Screen::Dashboard(_)))
        {
            state.refresh(self.tx.clone());
        }
    }

    fn refresh_news(&mut self) {
        if let Some(Screen::Dashboard(state)) = self
            .screens
            .iter_mut()
            .find(|screen| matches!(screen, Screen::Dashboard(_)))
        {
            state.refresh_news(self.tx.clone());
        }
    }

    /// Same-sector quotes from the dashboard for the stock currently shown in
    /// the detail screen, used as the AI analysis peer comparison set.
    fn detail_peers(&self) -> Vec<StockQuote> {
        let Some(Screen::Detail(detail_state)) = self
            .screens
            .iter()
            .rev()
            .find(|screen| matches!(screen, Screen::Detail(_)))
        else {
            return Vec::new();
        };
        let Some(detail) = detail_state.detail.as_ref() else {
            return Vec::new();
        };
        if detail.sector.is_empty() {
            return Vec::new();
        }
        let Some(Screen::Dashboard(dash)) = self
            .screens
            .iter()
            .find(|screen| matches!(screen, Screen::Dashboard(_)))
        else {
            return Vec::new();
        };
        let universe = match detail.market {
            crate::models::Market::Us => &dash.us_quotes,
            crate::models::Market::Kr => &dash.kr_quotes,
        };
        universe
            .iter()
            .filter(|quote| quote.sector.eq_ignore_ascii_case(&detail.sector))
            .cloned()
            .collect()
    }
}

fn push_detail(screens: &mut Vec<Screen>, tx: &UnboundedSender<AppMsg>, stock: StockQuote) {
    let market = stock.market;
    let mut state = DetailState::new(stock.symbol, market);
    state.refresh(tx.clone());
    screens.push(Screen::Detail(state));
}

fn push_article(screens: &mut Vec<Screen>, tx: &UnboundedSender<AppMsg>, news: NewsItem) {
    let mut state = ArticleState::new(news);
    state.refresh(tx.clone());
    screens.push(Screen::Article(state));
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_shortcuts_modal(frame: &mut ratatui::Frame, area: Rect, screen: Option<&Screen>) {
    let horizontal_margin = (area.width / 8).max(6);
    let vertical_margin = (area.height / 8).max(3);
    let modal = Rect {
        x: area.x + horizontal_margin,
        y: area.y + vertical_margin,
        width: area.width.saturating_sub(horizontal_margin * 2),
        height: area.height.saturating_sub(vertical_margin * 2),
    };
    let mut lines = vec![
        Line::from(Span::styled(
            "Global",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  q        Quit"),
        Line::from("  ?        Toggle this shortcuts modal"),
        Line::from("  Esc      Close modal / back out of current context"),
        Line::from(""),
    ];
    match screen {
        Some(Screen::Dashboard(_)) => {
            lines.extend([
                Line::from(Span::styled(
                    "Dashboard",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("  Tab      Switch focus between market list and news"),
                Line::from("  h/l      Switch active market tab when list is focused"),
                Line::from("  j/k      Move selection"),
                Line::from("  /        Search/filter focused list"),
                Line::from(
                    "  tag:ai   Search by generated theme tag (also energy/bond/power/etc.)",
                ),
                Line::from("  m        Toggle market heatmap view (sector-grouped colored cells)"),
                Line::from("  +        Add custom symbol/code to current tab"),
                Line::from("  s        Cycle primary sort field"),
                Line::from("  S        Toggle sort direction"),
                Line::from("  i        Open inspector for selected ETF/bond"),
                Line::from("  ETF/Bond note: funds use inspector instead of stock detail because holdings/yield/duration are more meaningful than PER/EPS/order-book fields"),
                Line::from("  Signal column: GC!/GCn/GC~ and DC!/DCn/DC~ reflect cross type + recency; UP/DN show ongoing trend"),
                Line::from("  Flow column: B/S/Q approximates buying, selling, or quiet volume pressure"),
                Line::from("  r        Refresh market data and news"),
                Line::from("  Enter    Open selected stock or article"),
                Line::from(""),
                Line::from("Sections"),
                Line::from("  Economic Indicators: oil, metals, FX, rates, and crypto snapshots"),
                Line::from("  Market Summary: advancers/decliners, top movers, and volume leaders"),
                Line::from("  Sector Performance: average change by sector/theme"),
                Line::from("  Search tips: type plain text or tag:ai / tag:energy / tag:bond"),
            ]);
        }
        Some(Screen::Detail(_)) => {
            lines.extend([
                Line::from(Span::styled(
                    "Detail",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("  1/2/3/4  Switch chart period (1W/1M/3M/1Y)"),
                Line::from("  Left/Right or p  Cycle chart period"),
                Line::from("  a        Toggle AI stock analysis"),
                Line::from("  /        Search/filter company news"),
                Line::from("  j/k      Move company news selection"),
                Line::from("  Enter    Open selected company news article"),
                Line::from("  r        Refresh detail data"),
                Line::from("  b / Esc  Back to dashboard"),
            ]);
        }
        Some(Screen::Article(_)) => {
            lines.extend([
                Line::from(Span::styled(
                    "Article",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("  r        Reload article content and AI analysis"),
                Line::from("  b / Esc  Back to previous screen"),
            ]);
        }
        None => {}
    }
    frame.render_widget(Clear, modal);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Shortcuts ")
                .border_style(Style::default().fg(theme::ACCENT)),
        ),
        modal,
    );
}

fn render_inspector_modal(frame: &mut ratatui::Frame, area: Rect, profile: &InstrumentProfile) {
    let horizontal_margin = (area.width / 10).max(8);
    let vertical_margin = (area.height / 8).max(3);
    let modal = Rect {
        x: area.x + horizontal_margin,
        y: area.y + vertical_margin,
        width: area.width.saturating_sub(horizontal_margin * 2),
        height: area.height.saturating_sub(vertical_margin * 2),
    };
    let kind = match profile.kind {
        InstrumentKind::Stock => "Stock",
        InstrumentKind::Etf => "ETF",
        InstrumentKind::BondFund => "Bond",
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} ({})", profile.display_name, profile.symbol),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("Type: {}", kind)),
        Line::from(format!("Issuer: {}", profile.issuer)),
        Line::from(format!("Category: {}", profile.category)),
        Line::from(format!("Objective: {}", profile.objective)),
    ];
    if let Some(benchmark) = &profile.benchmark {
        lines.push(Line::from(format!("Benchmark: {}", benchmark)));
    }
    if let Some(expense_ratio) = profile.expense_ratio_pct {
        lines.push(Line::from(format!("Expense Ratio: {:.2}%", expense_ratio)));
    }
    if let Some(distribution_yield) = profile.distribution_yield_pct {
        lines.push(Line::from(format!(
            "Distribution Yield: {:.2}%",
            distribution_yield
        )));
    }
    if let Some(duration) = profile.duration_years {
        lines.push(Line::from(format!("Duration: {:.1} years", duration)));
    }
    if !profile.top_holdings.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Top Holdings",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        for holding in &profile.top_holdings {
            lines.push(Line::from(format!(
                "  • {} — {:.1}%",
                holding.name, holding.weight_pct
            )));
        }
    }
    if !profile.notes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Notes",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        for note in &profile.notes {
            lines.push(Line::from(format!("  • {}", note)));
        }
    }
    if let Some(tags) = config::instrument_tags(&profile.symbol, &profile.category)
        .into_iter()
        .reduce(|left, right| format!("{left}, {right}"))
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Tags: {}", tags),
            Style::default().fg(theme::DIM),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Esc / i 로 닫기"));
    frame.render_widget(Clear, modal);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Inspector ")
                .border_style(Style::default().fg(theme::ACCENT)),
        ),
        modal,
    );
}

use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::{
    models::{
        stock::{FlowKind, SignalKind},
        StockQuote,
    },
    ui::theme,
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    stocks: &[StockQuote],
    selected: usize,
    active: bool,
) {
    let header = Row::new([
        "Signal", "Flow", "Symbol", "Name", "Price", "Chg", "%", "Vol", "MktCap",
    ])
    .style(
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    );
    let rows = stocks.iter().map(|stock| {
        let style = Style::default().fg(theme::change_color(stock.change_pct));
        let change = format!("{} {}", stock.arrow(), stock.formatted_change());
        let signal_info = stock.signal_info();
        let signal_style = match signal_info.kind {
            SignalKind::GoldenCross => Style::default().fg(theme::UP).add_modifier(Modifier::BOLD),
            SignalKind::DeathCross => Style::default()
                .fg(theme::DOWN)
                .add_modifier(Modifier::BOLD),
            SignalKind::BullTrend => Style::default().fg(theme::ACCENT),
            SignalKind::BearTrend => Style::default().fg(theme::DIM),
            SignalKind::Neutral => Style::default().fg(theme::DIM),
        };
        let flow_info = stock.flow_info();
        let flow_style = match flow_info.kind {
            FlowKind::Buying => Style::default().fg(theme::UP),
            FlowKind::Selling => Style::default().fg(theme::DOWN),
            FlowKind::Quiet => Style::default().fg(theme::DIM),
        };
        Row::new(vec![
            Cell::from(stock.technical_signal_label()).style(signal_style),
            Cell::from(stock.flow_label()).style(flow_style),
            Cell::from(stock.symbol.clone()),
            Cell::from(stock.name.clone()),
            Cell::from(stock.formatted_price()),
            Cell::from(change).style(style),
            Cell::from(stock.formatted_change_pct()).style(style),
            Cell::from(crate::models::format_volume(stock.volume)),
            Cell::from(stock.formatted_market_cap()),
        ])
    });
    let border = if active { theme::ACCENT } else { theme::BORDER };
    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(16),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .row_highlight_style(
        Style::default()
            .bg(theme::SURFACE)
            .add_modifier(Modifier::BOLD),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border)),
    );
    let selected_row = if stocks.is_empty() {
        None
    } else {
        Some(selected.min(stocks.len() - 1))
    };
    let mut state = TableState::default().with_selected(selected_row);
    frame.render_stateful_widget(table, area, &mut state);
}

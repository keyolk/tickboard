use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::{
    models::{ChartPeriod, FxRate},
    ui::theme,
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    rates: &[FxRate],
    selected: usize,
    active: bool,
    period: ChartPeriod,
) {
    let quote_label = rates
        .first()
        .map(|rate| rate.quote_currency.as_str())
        .unwrap_or("Quote");
    let header = Row::new(["Pair", quote_label, "%", "Chart"]).style(
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    );
    let rows = rates.iter().map(|rate| {
        let color = theme::change_color(rate.change_pct);
        Row::new(vec![
            Cell::from(rate.pair.clone()),
            Cell::from(rate.formatted_value()).style(Style::default().fg(color)),
            Cell::from(rate.formatted_change_pct()).style(Style::default().fg(color)),
            Cell::from(trend_bar(rate, period)).style(Style::default().fg(color)),
        ])
    });
    let border = if active { theme::ACCENT } else { theme::BORDER };
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Min(18),
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

    let selected_row = if rates.is_empty() {
        None
    } else {
        Some(selected.min(rates.len() - 1))
    };
    let mut state = TableState::default().with_selected(selected_row);
    frame.render_stateful_widget(table, area, &mut state);
}

fn trend_bar(rate: &FxRate, period: ChartPeriod) -> String {
    let history = match period {
        ChartPeriod::OneWeek => &rate.history_7d,
        ChartPeriod::OneMonth => &rate.history_30d,
        ChartPeriod::ThreeMonths => &rate.history_90d,
        ChartPeriod::OneYear => &rate.history_1y,
    };
    if history.is_empty() {
        return String::new();
    }
    let min = history.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = history.iter().copied().reduce(f64::max).unwrap_or(1.0);
    let span = (max - min).max(0.0001);
    history
        .iter()
        .map(|value| {
            let level = (((*value - min) / span) * 7.0).round() as usize;
            ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][level.min(7)]
        })
        .collect()
}

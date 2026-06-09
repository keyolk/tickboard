use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Sparkline},
    Frame,
};

use crate::{
    models::{ChartPeriod, FxRate},
    ui::{layout, theme},
};

pub fn render(frame: &mut Frame, area: Rect, fx: Option<&FxRate>, period: ChartPeriod) {
    let rows = layout::vertical(
        area,
        &[
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(2),
        ],
    );

    let Some(fx) = fx else {
        frame.render_widget(
            Paragraph::new("환율 데이터를 불러오는 중...").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" FX ")
                    .border_style(Style::default().fg(theme::BORDER)),
            ),
            area,
        );
        return;
    };

    let color = theme::change_color(fx.change_pct);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", fx.pair),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(fx.formatted_value(), Style::default().fg(color)),
            ]),
            Line::from(vec![
                Span::styled(fx.formatted_change_pct(), Style::default().fg(color)),
                Span::raw(format!(
                    "  |  Base {}  Quote {}",
                    fx.base_currency, fx.quote_currency
                )),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" FX {} ", period.label()))
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        rows[0],
    );

    let history = history_for_period(fx, period);
    let sparkline_data = expand_series(
        &normalize(&history),
        rows[1].width.saturating_sub(2) as usize,
    );
    frame.render_widget(
        Sparkline::default()
            .data(&sparkline_data)
            .style(Style::default().fg(color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Chart ")
                    .border_style(Style::default().fg(theme::BORDER)),
            ),
        rows[1],
    );

    let start = history.first().copied().unwrap_or_default();
    let end = history.last().copied().unwrap_or_default();
    let min = history.iter().copied().reduce(f64::min).unwrap_or_default();
    let max = history.iter().copied().reduce(f64::max).unwrap_or_default();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("Start {:.2}", start),
                Style::default().fg(theme::DIM),
            ),
            Span::raw("  "),
            Span::styled(format!("Low {:.2}", min), Style::default().fg(theme::DIM)),
            Span::raw("  "),
            Span::styled(format!("High {:.2}", max), Style::default().fg(theme::DIM)),
            Span::raw("  "),
            Span::styled(format!("End {:.2}", end), Style::default().fg(theme::DIM)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        rows[2],
    );
}

fn history_for_period(fx: &FxRate, period: ChartPeriod) -> Vec<f64> {
    match period {
        ChartPeriod::OneWeek => fx.history_7d.clone(),
        ChartPeriod::OneMonth => fx.history_30d.clone(),
        ChartPeriod::ThreeMonths => fx.history_90d.clone(),
        ChartPeriod::OneYear => fx.history_1y.clone(),
    }
}

fn normalize(values: &[f64]) -> Vec<u64> {
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(1.0);
    let span = (max - min).max(0.0001);
    values
        .iter()
        .map(|value| (((*value - min) / span) * 100.0).round() as u64)
        .collect()
}

fn expand_series(values: &[u64], target_width: usize) -> Vec<u64> {
    if values.is_empty() || target_width == 0 {
        return Vec::new();
    }
    if values.len() >= target_width {
        return values[..target_width].to_vec();
    }
    let mut expanded = Vec::with_capacity(target_width);
    for column in 0..target_width {
        let source_index = column * values.len() / target_width;
        expanded.push(values[source_index]);
    }
    expanded
}

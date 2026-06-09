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

pub fn render(
    frame: &mut Frame,
    area: Rect,
    fx_rates: &[FxRate],
    period: ChartPeriod,
    context_lines: &[String],
    sort_label: &str,
) {
    let outer = layout::vertical(
        area,
        &[
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(2),
        ],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(
                    " {} FX ",
                    fx_rates
                        .first()
                        .map(|rate| rate.quote_currency.as_str())
                        .unwrap_or("FX")
                ),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" {} | {}", period.label(), sort_label)),
        ]))
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        outer[0],
    );

    let columns = layout::horizontal(
        outer[1],
        &[
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ],
    );

    for (index, fx) in fx_rates.iter().take(3).enumerate() {
        render_card(frame, columns[index], fx, period);
    }

    let context = context_lines
        .iter()
        .map(|line| Line::from(line.clone()))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(context).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        outer[2],
    );
}

fn render_card(frame: &mut Frame, area: Rect, fx: &FxRate, period: ChartPeriod) {
    let rows = layout::vertical(
        area,
        &[
            Constraint::Length(2),
            Constraint::Min(2),
            Constraint::Length(1),
        ],
    );
    let color = theme::change_color(fx.change_pct);
    let history = history_for_period(fx, period);
    let sparkline_data = expand_series(
        &normalize(&history),
        rows[1].width.saturating_sub(2) as usize,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", fx.pair),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(fx.formatted_value(), Style::default().fg(color)),
            ]),
            Line::from(Span::styled(
                fx.formatted_change_pct(),
                Style::default().fg(color),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        rows[0],
    );

    frame.render_widget(
        Sparkline::default()
            .data(&sparkline_data)
            .style(Style::default().fg(color))
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_style(Style::default().fg(theme::BORDER)),
            ),
        rows[1],
    );

    let start = history.first().copied().unwrap_or_default();
    let end = history.last().copied().unwrap_or_default();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{:>6.0}", start), Style::default().fg(theme::DIM)),
            Span::raw(" → "),
            Span::styled(format!("{:>6.0}", end), Style::default().fg(theme::DIM)),
        ]))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        rows[2],
    );
}

fn history_for_period(fx: &FxRate, period: ChartPeriod) -> Vec<f64> {
    match period {
        ChartPeriod::OneWeek => fx.history_7d.clone(),
        ChartPeriod::OneMonth => fx.history_30d.clone(),
        ChartPeriod::ThreeMonths | ChartPeriod::OneYear => fx.history_30d.clone(),
    }
}

fn normalize(values: &[f64]) -> Vec<u64> {
    let min = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .reduce(f64::min)
        .unwrap_or(0.0);
    let max = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .reduce(f64::max)
        .unwrap_or(1.0);
    let span = (max - min).max(0.0001);
    values
        .iter()
        .map(|v| (((*v - min) / span) * 100.0).round() as u64)
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

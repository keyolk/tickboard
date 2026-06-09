use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    models::{Currency, StockDetail},
    ui::{format, theme},
};

/// Render 52-week and intraday range gauges with a marker showing where the
/// current price sits inside each band.
pub fn render(frame: &mut Frame, area: Rect, detail: Option<&StockDetail>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Range Position ")
        .border_style(Style::default().fg(theme::BORDER));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(d) = detail else {
        frame.render_widget(Paragraph::new(" Loading range..."), inner);
        return;
    };

    let track = (inner.width as usize).saturating_sub(34).clamp(8, 60);
    let lines = vec![
        gauge_line(
            "52W",
            d.week52_low,
            d.week52_high,
            d.price,
            d.currency,
            track,
        ),
        gauge_line("Day", d.low, d.high, d.price, d.currency, track),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn gauge_line(
    label: &str,
    low: f64,
    high: f64,
    price: f64,
    currency: Currency,
    track: usize,
) -> Line<'static> {
    let pos = if (high - low).abs() < f64::EPSILON {
        0.5
    } else {
        ((price - low) / (high - low)).clamp(0.0, 1.0)
    };
    let marker = ((pos * track as f64).round() as usize).min(track.saturating_sub(1));
    let pct = pos * 100.0;

    let mut bar = String::with_capacity(track + 2);
    bar.push('├');
    for cell in 0..track {
        if cell == marker {
            bar.push('●');
        } else {
            bar.push('─');
        }
    }
    bar.push('┤');

    let zone_color = if pct >= 85.0 {
        theme::UP
    } else if pct <= 15.0 {
        theme::DOWN
    } else {
        theme::ACCENT
    };

    Line::from(vec![
        Span::styled(
            format!(" {label} "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>9} ", short_money(low, currency)),
            Style::default().fg(theme::DIM),
        ),
        Span::styled(bar, Style::default().fg(zone_color)),
        Span::styled(
            format!(" {:<9}", short_money(high, currency)),
            Style::default().fg(theme::DIM),
        ),
        Span::styled(
            format!(" {pct:>3.0}% {}", zone_label(pct)),
            Style::default().fg(zone_color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn zone_label(pct: f64) -> &'static str {
    if pct >= 85.0 {
        "고점권"
    } else if pct <= 15.0 {
        "저점권"
    } else {
        "중립"
    }
}

fn short_money(value: f64, currency: Currency) -> String {
    match currency {
        Currency::Usd => format::money(value, currency),
        Currency::Krw => crate::models::format_number(value, 0),
    }
}

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    models::{format_volume, StockQuote},
    ui::theme,
};

pub fn render(frame: &mut Frame, area: Rect, us: &[StockQuote], kr: &[StockQuote]) {
    let lines = vec![market_line("US", us), market_line("KR", kr)];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Market Summary ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        area,
    );
}

fn market_line<'a>(label: &'a str, stocks: &'a [StockQuote]) -> Line<'a> {
    if stocks.is_empty() {
        return Line::from(vec![
            Span::styled(
                format!("{label}: "),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("Loading..."),
        ]);
    }
    let up = stocks.iter().filter(|s| s.change_pct >= 0.0).count();
    let down = stocks.len().saturating_sub(up);
    let best = stocks
        .iter()
        .max_by(|a, b| a.change_pct.total_cmp(&b.change_pct));
    let worst = stocks
        .iter()
        .min_by(|a, b| a.change_pct.total_cmp(&b.change_pct));
    let volume = stocks.iter().max_by_key(|s| s.volume);
    let mut spans = vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("▲{up} "), Style::default().fg(theme::UP)),
        Span::styled(format!("▼{down}  "), Style::default().fg(theme::DOWN)),
    ];
    if let Some(best) = best {
        spans.push(Span::raw("Top "));
        spans.push(Span::styled(
            format!("{} {}  ", best.symbol, best.formatted_change_pct()),
            Style::default().fg(theme::UP),
        ));
    }
    if let Some(worst) = worst {
        spans.push(Span::raw("Low "));
        spans.push(Span::styled(
            format!("{} {}  ", worst.symbol, worst.formatted_change_pct()),
            Style::default().fg(theme::DOWN),
        ));
    }
    if let Some(volume) = volume {
        spans.push(Span::raw(format!(
            "Vol {} {}",
            volume.symbol,
            format_volume(volume.volume)
        )));
    }
    Line::from(spans)
}

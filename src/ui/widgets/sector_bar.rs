use std::collections::BTreeMap;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{models::StockQuote, ui::theme};

pub fn render(frame: &mut Frame, area: Rect, us: &[StockQuote], kr: &[StockQuote]) {
    let lines = vec![sector_line("US", us), sector_line("KR", kr)];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Sector Performance ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        area,
    );
}

fn sector_line<'a>(label: &'a str, stocks: &'a [StockQuote]) -> Line<'a> {
    if stocks.is_empty() {
        return Line::from(format!("{label}: Loading..."));
    }
    let mut grouped: BTreeMap<&str, (f64, usize)> = BTreeMap::new();
    for stock in stocks {
        if stock.sector.is_empty() {
            continue;
        }
        let entry = grouped.entry(stock.sector.as_str()).or_default();
        entry.0 += stock.change_pct;
        entry.1 += 1;
    }
    let mut averages = grouped
        .into_iter()
        .map(|(sector, (sum, count))| (sector, sum / count as f64))
        .collect::<Vec<_>>();
    averages.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    let mut spans = vec![Span::styled(
        format!("{label}: "),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    for (sector, value) in averages.into_iter().take(6) {
        let bar = "█".repeat(((value.abs() * 1.5).round() as usize).clamp(1, 8));
        spans.push(Span::raw(format!("{sector} ")));
        spans.push(Span::styled(
            format!("{bar} {value:+.1}%  "),
            Style::default().fg(theme::change_color(value)),
        ));
    }
    Line::from(spans)
}

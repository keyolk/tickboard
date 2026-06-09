use std::collections::BTreeMap;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{models::StockQuote, ui::theme};

const CELL_WIDTH: usize = 9;

/// Render a grass-style heatmap: each stock is a coloured cell whose background
/// intensity tracks its percent change (red up / blue down), grouped into rows
/// by sector. Sectors are ordered by average move, cells within a sector by
/// individual move.
pub fn render(frame: &mut Frame, area: Rect, title: &str, stocks: &[StockQuote], active: bool) {
    let border = if active { theme::ACCENT } else { theme::BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if stocks.is_empty() {
        frame.render_widget(Paragraph::new(" Loading heatmap..."), inner);
        return;
    }

    let label_width = stocks
        .iter()
        .map(|s| display_sector(&s.sector).chars().count())
        .max()
        .unwrap_or(8)
        .clamp(6, 14);
    let cols = ((inner.width as usize).saturating_sub(label_width + 1) / CELL_WIDTH).max(1);

    // Group by sector, accumulating average change for ordering.
    let mut grouped: BTreeMap<String, Vec<&StockQuote>> = BTreeMap::new();
    for stock in stocks {
        grouped
            .entry(display_sector(&stock.sector))
            .or_default()
            .push(stock);
    }
    let mut sectors: Vec<(String, Vec<&StockQuote>, f64)> = grouped
        .into_iter()
        .map(|(name, mut members)| {
            members.sort_by(|a, b| b.change_pct.total_cmp(&a.change_pct));
            let avg = members.iter().map(|m| m.change_pct).sum::<f64>() / members.len() as f64;
            (name, members, avg)
        })
        .collect();
    sectors.sort_by(|a, b| b.2.total_cmp(&a.2));

    let mut lines = Vec::new();
    let max_rows = inner.height as usize;
    for (name, members, avg) in &sectors {
        if lines.len() >= max_rows {
            break;
        }
        // Each sector occupies one or more wrapped rows of cells.
        for (chunk_idx, chunk) in members.chunks(cols).enumerate() {
            if lines.len() >= max_rows {
                break;
            }
            let mut spans = Vec::with_capacity(chunk.len() + 1);
            if chunk_idx == 0 {
                spans.push(Span::styled(
                    format!("{:>width$} ", name, width = label_width),
                    Style::default()
                        .fg(theme::change_color(*avg))
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw(" ".repeat(label_width + 1)));
            }
            for stock in chunk {
                spans.push(cell_span(stock));
            }
            lines.push(Line::from(spans));
        }
    }
    if lines.len() < max_rows {
        lines.push(Line::from(Span::styled(
            " 진하게 빨강=강한 상승 · 파랑=강한 하락 · [m] 테이블로 전환",
            Style::default().fg(theme::DIM),
        )));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn cell_span(stock: &StockQuote) -> Span<'static> {
    let symbol = truncate(&stock.symbol, 5);
    // e.g. " AAPL +1.8" packed into CELL_WIDTH.
    let text = format!(" {symbol:<5}{:>+4.0}", stock.change_pct);
    let text = format!("{text:<width$}", width = CELL_WIDTH);
    Span::styled(
        text,
        Style::default()
            .bg(theme::heat_bg(stock.change_pct))
            .fg(theme::heat_fg(stock.change_pct)),
    )
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        text.chars().take(max).collect()
    }
}

fn display_sector(sector: &str) -> String {
    if sector.is_empty() {
        "기타".to_string()
    } else {
        sector.to_string()
    }
}

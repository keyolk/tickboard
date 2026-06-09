use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{models::StockQuote, ui::theme};

/// Render market breadth (advancers vs decliners gauge) plus a change-percent
/// distribution histogram for the combined US + KR universe.
pub fn render(frame: &mut Frame, area: Rect, us: &[StockQuote], kr: &[StockQuote]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Market Breadth & Distribution ")
        .border_style(Style::default().fg(theme::BORDER));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let all: Vec<&StockQuote> = us.iter().chain(kr.iter()).collect();
    if all.is_empty() {
        frame.render_widget(Paragraph::new(" Loading breadth..."), inner);
        return;
    }

    let track = (inner.width as usize).saturating_sub(28).clamp(10, 60);
    let up = all.iter().filter(|q| q.change_pct >= 0.0).count();
    let down = all.len() - up;
    let total = all.len().max(1);

    let mut lines = vec![breadth_line(up, down, total, track)];
    lines.push(distribution_line(&all, inner.width as usize));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn breadth_line(up: usize, down: usize, total: usize, track: usize) -> Line<'static> {
    let up_cells = (up * track / total).min(track);
    let down_cells = track - up_cells;
    let up_pct = up as f64 / total as f64 * 100.0;

    Line::from(vec![
        Span::styled(" Adv/Dec ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("█".repeat(up_cells), Style::default().fg(theme::UP)),
        Span::styled("█".repeat(down_cells), Style::default().fg(theme::DOWN)),
        Span::styled(
            format!(" {up_pct:>3.0}% "),
            Style::default().fg(theme::change_color(up_pct - 50.0)),
        ),
        Span::styled(format!("▲{up} "), Style::default().fg(theme::UP)),
        Span::styled(format!("▼{down}"), Style::default().fg(theme::DOWN)),
    ])
}

/// 11-bucket histogram spanning roughly [-5%, +5%], using vertical block glyphs
/// for density. Each bucket is coloured by its sign.
fn distribution_line(all: &[&StockQuote], width: usize) -> Line<'static> {
    const BUCKETS: usize = 11;
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mut counts = [0usize; BUCKETS];
    for quote in all {
        // Map -5%..+5% onto 0..10, clamping the tails into the end buckets.
        let bucket = (((quote.change_pct + 5.0) / 10.0) * (BUCKETS - 1) as f64)
            .round()
            .clamp(0.0, (BUCKETS - 1) as f64) as usize;
        counts[bucket] += 1;
    }
    let peak = counts.iter().copied().max().unwrap_or(1).max(1);

    let mut median: Vec<f64> = all.iter().map(|q| q.change_pct).collect();
    median.sort_by(f64::total_cmp);
    let med = median[median.len() / 2];

    let mut spans = vec![Span::styled(
        " Dist ",
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if width >= 40 {
        spans.push(Span::styled("-5% ", Style::default().fg(theme::DIM)));
    }
    for (i, &count) in counts.iter().enumerate() {
        let level = if count == 0 {
            0
        } else {
            1 + (count * (BLOCKS.len() - 2) / peak).min(BLOCKS.len() - 2)
        };
        let center = (BUCKETS - 1) as f64 / 2.0;
        let color = if (i as f64) > center {
            theme::UP
        } else if (i as f64) < center {
            theme::DOWN
        } else {
            theme::DIM
        };
        spans.push(Span::styled(
            BLOCKS[level].to_string(),
            Style::default().fg(color),
        ));
    }
    if width >= 40 {
        spans.push(Span::styled(" +5%", Style::default().fg(theme::DIM)));
    }
    spans.push(Span::styled(
        format!("  median {med:+.1}%"),
        Style::default()
            .fg(theme::change_color(med))
            .add_modifier(Modifier::BOLD),
    ));
    Line::from(spans)
}

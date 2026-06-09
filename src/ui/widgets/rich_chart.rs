use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    models::{format_number, ChartPeriod, Currency, StockDetail},
    ui::{layout, theme},
};

pub fn render(frame: &mut Frame, area: Rect, detail: Option<&StockDetail>, period: ChartPeriod) {
    let chunks = layout::vertical(
        area,
        &[
            ratatui::layout::Constraint::Min(5),
            ratatui::layout::Constraint::Length(2),
        ],
    );
    let values = detail
        .map(|d| history_for_period(d, period))
        .unwrap_or_default();
    let dates = detail
        .map(|d| dates_for_period(d, period))
        .unwrap_or_default();
    let currency = detail.map(|d| d.currency).unwrap_or(Currency::Usd);
    let title = format!(" Price Chart {} ", period.label());

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme::BORDER));
    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);

    let body = if values.len() >= 2 {
        braille_chart(&values, &dates, currency, inner.width, inner.height)
    } else {
        vec![Line::from("차트 데이터를 불러오는 중...")]
    };
    frame.render_widget(Paragraph::new(body), inner);

    let info = if values.len() >= 2 {
        let first = values[0];
        let last = *values.last().unwrap_or(&first);
        let ret = if first != 0.0 {
            (last - first) / first * 100.0
        } else {
            0.0
        };
        let (ma5, ma20) = (moving_average(&values, 5), moving_average(&values, 20));
        Line::from(vec![
            Span::raw(" Return "),
            Span::styled(
                format!("{ret:+.2}% "),
                Style::default()
                    .fg(theme::change_color(ret))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " MA5 {}  MA20 {} ",
                format_opt(ma5),
                format_opt(ma20)
            )),
            Span::styled(cross_signal(ma5, ma20), Style::default().fg(theme::ACCENT)),
        ])
    } else {
        Line::from("")
    };
    frame.render_widget(Paragraph::new(info), chunks[1]);
}

pub fn history_for_period(detail: &StockDetail, period: ChartPeriod) -> Vec<f64> {
    match period {
        ChartPeriod::OneWeek => detail.history_7d.clone(),
        ChartPeriod::OneMonth => detail.history_30d.clone(),
        ChartPeriod::ThreeMonths => detail.history_90d.clone(),
        ChartPeriod::OneYear => detail.history_1y.clone(),
    }
}

fn dates_for_period(detail: &StockDetail, period: ChartPeriod) -> Vec<String> {
    match period {
        ChartPeriod::OneWeek => detail.history_dates_7d.clone(),
        ChartPeriod::OneMonth => detail.history_dates_30d.clone(),
        ChartPeriod::ThreeMonths => detail.history_dates_90d.clone(),
        ChartPeriod::OneYear => detail.history_dates_1y.clone(),
    }
}

/// Render an axis-labelled braille line chart as `Line`s. The left gutter shows
/// price ticks, the body plots the series at 2×4 braille resolution coloured by
/// local direction (red rising / blue falling), and the bottom row shows date
/// ticks.
fn braille_chart(
    values: &[f64],
    dates: &[String],
    currency: Currency,
    width: u16,
    height: u16,
) -> Vec<Line<'static>> {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.len() < 2 || width < 8 || height < 2 {
        return vec![Line::from("")];
    }
    let min = finite.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = finite.iter().copied().reduce(f64::max).unwrap_or(1.0);
    let span = (max - min).max(f64::EPSILON);

    // Reserve the bottom text row for date ticks.
    let plot_rows = (height as usize).saturating_sub(1).max(1);
    // Gutter holds the widest price tick plus the axis glyph.
    let gutter = axis_label(max, currency)
        .len()
        .max(axis_label(min, currency).len());
    let plot_cols = (width as usize).saturating_sub(gutter + 1).max(4);

    let sub_w = plot_cols * 2;
    let sub_h = plot_rows * 4;

    // Sample the series across the sub-pixel columns and project to dot rows.
    let project = |i: usize| -> usize {
        let idx = if sub_w <= 1 {
            0
        } else {
            i * (values.len() - 1) / (sub_w - 1)
        };
        let v = values[idx.min(values.len() - 1)];
        let norm = ((v - min) / span).clamp(0.0, 1.0);
        // Invert: high price sits near the top.
        (sub_h - 1) - (norm * (sub_h - 1) as f64).round() as usize
    };

    let mut dots = vec![0u8; plot_cols * plot_rows];
    let mut col_up = vec![true; plot_cols];
    let mut prev_y = project(0);
    for i in 0..sub_w {
        let y = project(i);
        // Connect to the previous sub-column so the line is continuous.
        let (lo, hi) = if y <= prev_y {
            (y, prev_y)
        } else {
            (prev_y, y)
        };
        for yy in lo..=hi {
            set_dot(&mut dots, plot_cols, i, yy);
        }
        if i % 2 == 1 {
            let cell = i / 2;
            // Falling (line moved down → larger y) is blue, rising is red.
            col_up[cell] = y <= prev_y;
        }
        prev_y = y;
    }

    let mut lines = Vec::with_capacity(plot_rows + 1);
    for row in 0..plot_rows {
        let mut spans = Vec::with_capacity(plot_cols + 1);
        spans.push(gutter_span(row, plot_rows, min, max, currency, gutter));
        for col in 0..plot_cols {
            let dot = dots[row * plot_cols + col];
            let ch = if dot == 0 {
                ' '
            } else {
                char::from_u32(0x2800 + dot as u32).unwrap_or(' ')
            };
            let color = if col_up[col] { theme::UP } else { theme::DOWN };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }
    lines.push(date_axis(dates, gutter, plot_cols));
    lines
}

fn set_dot(dots: &mut [u8], cols: usize, px: usize, py: usize) {
    let cell_x = px / 2;
    let cell_y = py / 4;
    if cell_x >= cols {
        return;
    }
    let bit = match (px % 2, py % 4) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    };
    if let Some(slot) = dots.get_mut(cell_y * cols + cell_x) {
        *slot |= bit;
    }
}

fn gutter_span(
    row: usize,
    plot_rows: usize,
    min: f64,
    max: f64,
    currency: Currency,
    width: usize,
) -> Span<'static> {
    // Label the top, middle, and bottom rows; the rest get a bare axis line.
    let label = if row == 0 {
        Some(max)
    } else if row + 1 == plot_rows {
        Some(min)
    } else if plot_rows >= 5 && row == plot_rows / 2 {
        Some((min + max) / 2.0)
    } else {
        None
    };
    let text = match label {
        Some(value) => format!("{:>width$}┤", axis_label(value, currency), width = width),
        None => format!("{:>width$}│", "", width = width),
    };
    Span::styled(text, Style::default().fg(theme::DIM))
}

fn date_axis(dates: &[String], gutter: usize, plot_cols: usize) -> Line<'static> {
    let mut text = format!("{:>width$}└", "", width = gutter);
    if dates.len() < 2 || plot_cols < 6 {
        text.push_str(&"─".repeat(plot_cols));
        return Line::from(Span::styled(text, Style::default().fg(theme::DIM)));
    }
    let pick = |frac: f64| {
        let idx = ((dates.len() - 1) as f64 * frac).round() as usize;
        short_date(&dates[idx.min(dates.len() - 1)])
    };
    let left = pick(0.0);
    let mid = pick(0.5);
    let right = pick(1.0);

    // Lay the three ticks across the plot width over a baseline of dashes.
    let mut baseline: Vec<char> = "─".repeat(plot_cols).chars().collect();
    place(&mut baseline, 0, &left);
    place(&mut baseline, plot_cols.saturating_sub(right.len()), &right);
    let mid_start = plot_cols / 2 - (mid.len() / 2).min(plot_cols / 2);
    place(&mut baseline, mid_start, &mid);

    text.push_str(&baseline.into_iter().collect::<String>());
    Line::from(Span::styled(text, Style::default().fg(theme::DIM)))
}

fn place(buf: &mut [char], start: usize, label: &str) {
    for (offset, ch) in label.chars().enumerate() {
        if let Some(slot) = buf.get_mut(start + offset) {
            *slot = ch;
        }
    }
}

/// Trim an ISO-like date down to a compact `M/D` (or last segment) tick.
fn short_date(date: &str) -> String {
    let parts: Vec<&str> = date.split(['-', '/', '.']).collect();
    match parts.as_slice() {
        [_, m, d] => format!(
            "{}/{}",
            m.trim_start_matches('0'),
            d.trim_start_matches('0')
        ),
        [m, d] => format!(
            "{}/{}",
            m.trim_start_matches('0'),
            d.trim_start_matches('0')
        ),
        _ => date.to_string(),
    }
}

fn axis_label(value: f64, currency: Currency) -> String {
    match currency {
        Currency::Krw => format_number(value, 0),
        Currency::Usd => {
            if value.abs() >= 100.0 {
                format_number(value, 0)
            } else {
                format!("{value:.2}")
            }
        }
    }
}

fn moving_average(values: &[f64], window: usize) -> Option<f64> {
    if values.len() < window {
        None
    } else {
        Some(values[values.len() - window..].iter().sum::<f64>() / window as f64)
    }
}

fn format_opt(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.2}"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn cross_signal(ma5: Option<f64>, ma20: Option<f64>) -> &'static str {
    match (ma5, ma20) {
        (Some(short), Some(long)) if short > long => "골든크로스 구간",
        (Some(short), Some(long)) if short < long => "데드크로스 구간",
        _ => "",
    }
}

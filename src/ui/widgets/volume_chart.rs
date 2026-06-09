use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    models::{format_volume, ChartPeriod, StockDetail},
    ui::{layout, theme},
};

pub fn render(frame: &mut Frame, area: Rect, detail: Option<&StockDetail>, period: ChartPeriod) {
    let chunks = layout::vertical(
        area,
        &[
            ratatui::layout::Constraint::Min(4),
            ratatui::layout::Constraint::Length(2),
        ],
    );

    let (title, labels, values, up_bars) = if let Some(detail) = detail {
        (
            format!(" Volume {} ", period.label()),
            period_labels(detail, period),
            volume_history_for_period(detail, period),
            period_up_bars(detail, period),
        )
    } else {
        (" Volume ".to_string(), Vec::new(), Vec::new(), Vec::new())
    };

    let body = if values.is_empty() {
        vec![Line::from("거래량 데이터를 불러오는 중...")]
    } else {
        build_pressure_lines(
            &labels,
            &values,
            &up_bars,
            chunks[0].width as usize,
            chunks[0].height.saturating_sub(2) as usize,
        )
    };

    frame.render_widget(
        Paragraph::new(body).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        chunks[0],
    );

    let info = if values.is_empty() {
        Line::from("")
    } else {
        let current = values.last().copied().unwrap_or_default();
        let average = values.iter().copied().sum::<u64>() / values.len() as u64;
        let ratio = if average == 0 {
            0.0
        } else {
            current as f64 / average as f64
        };
        let emphasis = if ratio >= 2.0 {
            theme::UP
        } else if ratio <= 0.6 {
            theme::DOWN
        } else {
            theme::ACCENT
        };
        Line::from(vec![
            Span::raw(format!(
                " Latest {}  Avg {}  ",
                format_volume(current),
                format_volume(average)
            )),
            Span::styled(
                format!("x{ratio:.1}"),
                Style::default().fg(emphasis).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  빨강=매수 우위 볼륨, 파랑=매도 우위 볼륨"),
        ])
    };
    frame.render_widget(Paragraph::new(info), chunks[1]);
}

fn volume_history_for_period(detail: &StockDetail, period: ChartPeriod) -> Vec<u64> {
    match period {
        ChartPeriod::OneWeek => detail.volume_history_7d.clone(),
        ChartPeriod::OneMonth => detail.volume_history_30d.clone(),
        ChartPeriod::ThreeMonths => detail.volume_history_90d.clone(),
        ChartPeriod::OneYear => detail.volume_history_1y.clone(),
    }
}

fn period_labels(detail: &StockDetail, period: ChartPeriod) -> Vec<String> {
    match period {
        ChartPeriod::OneWeek => detail.history_dates_7d.clone(),
        ChartPeriod::OneMonth => detail.history_dates_30d.clone(),
        ChartPeriod::ThreeMonths => detail.history_dates_90d.clone(),
        ChartPeriod::OneYear => detail.history_dates_1y.clone(),
    }
}

fn period_up_bars(detail: &StockDetail, period: ChartPeriod) -> Vec<bool> {
    let history = match period {
        ChartPeriod::OneWeek => &detail.history_7d,
        ChartPeriod::OneMonth => &detail.history_30d,
        ChartPeriod::ThreeMonths => &detail.history_90d,
        ChartPeriod::OneYear => &detail.history_1y,
    };
    history
        .iter()
        .enumerate()
        .map(|(index, close)| {
            if index == 0 {
                detail.change >= 0.0
            } else {
                *close >= history[index - 1]
            }
        })
        .collect()
}

fn build_pressure_lines(
    labels: &[String],
    values: &[u64],
    up_bars: &[bool],
    area_width: usize,
    area_height: usize,
) -> Vec<Line<'static>> {
    if area_height == 0 || values.is_empty() {
        return Vec::new();
    }

    let max_value = values.iter().copied().max().unwrap_or(1).max(1);
    let chart_width = area_width.saturating_sub(2).max(8);
    let buckets = expanded_buckets(&bucket_volumes(values, up_bars, values.len()), chart_width);
    let label_row = label_line(labels, buckets.len(), values.len());
    let bar_height = area_height.saturating_sub(1).max(1);
    let mut lines = Vec::with_capacity(bar_height + 1);

    for row in (0..bar_height).rev() {
        let mut spans = Vec::with_capacity(buckets.len());
        for bucket in &buckets {
            let level = bucket_level(bucket.volume, max_value, bar_height);
            let filled = level > row;
            let color = if bucket.up_count >= bucket.down_count {
                theme::UP
            } else {
                theme::DOWN
            };
            spans.push(Span::styled(
                if filled { "█" } else { " " },
                Style::default().fg(color),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines.push(label_row);
    lines
}

#[derive(Debug, Clone)]
struct VolumeBucket {
    volume: u64,
    up_count: usize,
    down_count: usize,
}

fn bucket_volumes(values: &[u64], up_bars: &[bool], bucket_count: usize) -> Vec<VolumeBucket> {
    if bucket_count == 0 || values.is_empty() {
        return Vec::new();
    }
    (0..bucket_count)
        .map(|bucket_index| {
            let start = bucket_index * values.len() / bucket_count;
            let end = ((bucket_index + 1) * values.len() / bucket_count).max(start + 1);
            let range = start..end.min(values.len());
            let mut up_count = 0;
            let mut down_count = 0;
            let volume = range
                .clone()
                .map(|index| {
                    if up_bars.get(index).copied().unwrap_or(true) {
                        up_count += 1;
                    } else {
                        down_count += 1;
                    }
                    values[index]
                })
                .max()
                .unwrap_or_default();
            VolumeBucket {
                volume,
                up_count,
                down_count,
            }
        })
        .collect()
}

fn expanded_buckets(buckets: &[VolumeBucket], target_width: usize) -> Vec<VolumeBucket> {
    if buckets.is_empty() || target_width == 0 {
        return Vec::new();
    }
    if buckets.len() >= target_width {
        return buckets[..target_width].to_vec();
    }

    let mut expanded = Vec::with_capacity(target_width);
    for column in 0..target_width {
        let source_index = column * buckets.len() / target_width;
        expanded.push(buckets[source_index].clone());
    }
    expanded
}

fn label_line(labels: &[String], bucket_len: usize, source_len: usize) -> Line<'static> {
    let mut spans = vec![Span::raw(" ".to_string()); bucket_len];
    let label_slots = if labels.len() >= 3 { 3 } else { labels.len() };

    for (index, span) in spans.iter_mut().enumerate().take(bucket_len) {
        if should_insert_label(index, bucket_len, label_slots) {
            let source_index = source_index_for_bucket(index, bucket_len, source_len);
            if let Some(label) = labels.get(source_index) {
                *span = Span::styled(label.clone(), Style::default().fg(theme::DIM));
            }
        }
    }

    Line::from(spans)
}

fn bucket_level(volume: u64, max_value: u64, bar_height: usize) -> usize {
    if max_value == 0 || bar_height == 0 {
        0
    } else {
        ((volume as f64 / max_value as f64) * bar_height as f64)
            .ceil()
            .clamp(0.0, bar_height as f64) as usize
    }
}

fn should_insert_label(index: usize, len: usize, label_slots: usize) -> bool {
    label_slots > 0
        && match index {
            0 => true,
            _ if label_slots >= 2 && index == len / 2 => true,
            _ if label_slots >= 3 && index == len.saturating_sub(1) => true,
            _ => false,
        }
}

fn source_index_for_bucket(bucket_index: usize, bucket_len: usize, source_len: usize) -> usize {
    if bucket_len <= 1 || source_len <= 1 {
        0
    } else {
        (bucket_index * (source_len - 1) / (bucket_len - 1)).min(source_len - 1)
    }
}

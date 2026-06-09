use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{models::EconomicIndicator, ui::theme};

pub fn render(frame: &mut Frame, area: Rect, indicators: &[EconomicIndicator]) {
    let mut spans = Vec::new();
    if indicators.is_empty() {
        spans.push(Span::raw("Loading indicators..."));
    } else {
        for indicator in indicators {
            let color = theme::change_color(indicator.change_pct);
            spans.push(Span::styled(
                format!("{} ", indicator.name),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(format!("{} ", indicator.formatted_value())));
            spans.push(Span::styled(
                format!("{}  ", indicator.formatted_change_pct()),
                Style::default().fg(color),
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Economic Indicators ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        area,
    );
}

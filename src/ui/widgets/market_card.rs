use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{models::MarketIndex, ui::theme};

pub fn render(frame: &mut Frame, area: Rect, title: &str, index: Option<&MarketIndex>) {
    let lines = if let Some(index) = index {
        let change_color = theme::change_color(index.change_pct);
        vec![
            Line::from(Span::styled(
                index.formatted_value(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    index.formatted_change_pct(),
                    Style::default()
                        .fg(change_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {:+.2}", index.change)),
            ]),
        ]
    } else {
        vec![Line::from("Loading..."), Line::from("")]
    };
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER)),
    );
    frame.render_widget(paragraph, area);
}

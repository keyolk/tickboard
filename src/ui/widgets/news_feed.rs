use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{models::NewsItem, ui::theme};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    news: &[NewsItem],
    selected: usize,
    active: bool,
    title: &str,
) {
    let items = if news.is_empty() {
        vec![ListItem::new("Loading news...")]
    } else {
        news.iter()
            .map(|item| {
                let color = if item.is_korean {
                    theme::DOWN
                } else {
                    theme::UP
                };
                ListItem::new(format!("[{}] {}", item.source, item.title))
                    .style(Style::default().fg(color))
            })
            .collect()
    };
    let border = if active { theme::ACCENT } else { theme::BORDER };
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(theme::SURFACE)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border)),
        );
    let selected_item = if news.is_empty() {
        None
    } else {
        Some(selected.min(news.len() - 1))
    };
    let mut state = ListState::default().with_selected(selected_item);
    frame.render_stateful_widget(list, area, &mut state);
}

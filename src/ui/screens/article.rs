use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    models::NewsItem,
    tasks::{scheduler, AppMsg},
    ui::{layout, theme},
};

#[derive(Debug)]
pub struct ArticleState {
    pub generation: u64,
    pub item: NewsItem,
    pub content: String,
    pub analysis: String,
    pub status: String,
}

impl ArticleState {
    pub fn new(item: NewsItem) -> Self {
        Self {
            generation: 0,
            item,
            content: String::new(),
            analysis: String::new(),
            status: "Fetching article & AI analysis (Claude Sonnet 4.6)...".to_string(),
        }
    }

    pub fn refresh(&mut self, tx: UnboundedSender<AppMsg>) {
        self.generation = self.generation.wrapping_add(1);
        self.content.clear();
        self.analysis.clear();
        self.status = "[1/2] Fetching article content...".to_string();
        scheduler::spawn_article_analysis(tx, self.generation, self.item.clone());
    }

    pub fn apply_msg(&mut self, msg: &AppMsg) {
        if let AppMsg::ArticleLoaded {
            generation,
            content,
            analysis,
        } = msg
        {
            if *generation == self.generation {
                self.content = content.clone();
                self.analysis = analysis.clone();
                self.status =
                    "AI Analysis (Powered by Claude Sonnet 4.6 via Amazon Bedrock)".to_string();
            }
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &ArticleState) {
    let rows = layout::vertical(
        area,
        &[
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(8),
        ],
    );
    let badge = if state.item.is_korean {
        theme::DOWN
    } else {
        theme::UP
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!(" {}", state.item.title),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    format!(" [{}]", state.item.source),
                    Style::default().fg(badge).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(
                    "  {}  |  {}",
                    state.item.published,
                    if state.item.is_korean {
                        "한국어"
                    } else {
                        "English -> 한국어 번역"
                    }
                )),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Article ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(state.status.clone()).style(Style::default().fg(theme::DIM)),
        rows[1],
    );
    let body = if state.analysis.is_empty() {
        if state.content.is_empty() {
            "Loading...".to_string()
        } else {
            state.content.clone()
        }
    } else {
        state.analysis.clone()
    };
    frame.render_widget(
        Paragraph::new(body).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" AI Analysis ")
                .border_style(Style::default().fg(theme::BORDER)),
        ),
        rows[2],
    );
}

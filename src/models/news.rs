use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewsItem {
    pub title: String,
    pub source: String,
    pub url: String,
    pub published: String,
    pub description: String,
    pub is_korean: bool,
}

use std::sync::OnceLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use roux::{Subreddit, Reddit};
use crate::models::card::{Source, Platform};

static RE_CARD_SET: OnceLock<regex::Regex> = OnceLock::new();
static RE_CARD_SPOILER: OnceLock<regex::Regex> = OnceLock::new();

pub struct RedditClient {
    subreddit: Subreddit,
    seen_ids: Vec<String>,
}

impl RedditClient {
    pub fn new() -> Self {
        Self::with_subreddit("magicTCG")
    }

    pub fn with_subreddit(subreddit: &str) -> Self {
        let subreddit = Subreddit::new(subreddit);
        Self {
            subreddit,
            seen_ids: Vec::new(),
        }
    }

    pub async fn fetch_recent(&mut self) -> Result<Vec<RedditPost>> {
        let hot = self.subreddit.hot(25, None).await?;

        let posts: Vec<RedditPost> = hot.data.children
            .into_iter()
            .filter_map(|post| {
                let data = post.data;

                // Skip if already seen
                if self.seen_ids.contains(&data.id) {
                    return None;
                }

                // Skip non-spoiler posts
                if !is_spoiler_post(&data.title, &data.link_flair_text) {
                    return None;
                }

                self.seen_ids.push(data.id.clone());

                Some(RedditPost {
                    id: data.id,
                    title: data.title,
                    url: data.url.unwrap_or_default(),
                    author: data.author,
                    created_utc: DateTime::from_timestamp(data.created_utc as i64, 0)
                        .unwrap_or_else(|| Utc::now()),
                    upvotes: data.ups as i64,
                    thumbnail: if data.thumbnail.is_empty() { None } else { Some(data.thumbnail) },
                    is_self: data.is_self,
                    selftext: data.selftext,
                    flair: data.link_flair_text,
                })
            })
            .collect();

        Ok(posts)
    }
}

fn is_spoiler_post(title: &str, flair: &Option<String>) -> bool {
    let title_lower = title.to_lowercase();
    let spoiler_keywords = ["spoiler", "spoilers", "leak", "leaked", "revealed"];

    // Check title
    if spoiler_keywords.iter().any(|kw| title_lower.contains(kw)) {
        return true;
    }

    // Check flair
    if let Some(f) = flair {
        let f_lower = f.to_lowercase();
        if f_lower.contains("spoiler") || f_lower.contains("leak") {
            return true;
        }
    }

    false
}

#[derive(Debug, Clone)]
pub struct RedditPost {
    pub id: String,
    pub title: String,
    pub url: String,
    pub author: String,
    pub created_utc: DateTime<Utc>,
    pub upvotes: i64,
    pub thumbnail: Option<String>,
    pub is_self: bool,
    pub selftext: String,
    pub flair: Option<String>,
}

impl RedditPost {
    /// Extract card name from Reddit post title using regex patterns
    pub fn extract_card_name(&self) -> Option<String> {
        // Pattern: [SET] Card Name (cost)
        let re1 = RE_CARD_SET.get_or_init(|| {
            regex::Regex::new(r"\[(\w{2,4})\]\s+(.+?)\s+\(\d")
                .expect("RE_CARD_SET regex is valid")
        });
        if let Some(caps) = re1.captures(&self.title) {
            return Some(caps.get(2)?.as_str().trim().to_string());
        }

        // Pattern: Spoiler: Card Name
        let re2 = RE_CARD_SPOILER.get_or_init(|| {
            regex::Regex::new(r"(?i)spoiler[:\s]+(.+?)(?:\s+from|\s+\[|$)")
                .expect("RE_CARD_SPOILER regex is valid")
        });
        if let Some(caps) = re2.captures(&self.title) {
            return Some(caps.get(1)?.as_str().trim().to_string());
        }

        None
    }

    pub fn to_source(&self, score: f32) -> Source {
        Source {
            platform: Platform::Reddit,
            url: format!("https://reddit.com{}", self.url),
            author: Some(self.author.clone()),
            posted_at: self.created_utc,
            raw_title: self.title.clone(),
            upvotes: Some(self.upvotes as i32),
            authenticity_score: score,
        }
    }
}

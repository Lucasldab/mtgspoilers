use anyhow::Result;
use chrono::Utc;
use std::sync::OnceLock;

use crate::api::reddit::RedditClient;
use crate::api::scryfall::ScryfallClient;
use crate::db::Database;
use crate::filter::{dedup::Deduplicator, verify::AuthenticityScorer};
use crate::models::card::{Card, Confidence};
use tracing::{info, warn, error};

static RE_SET_CODE: OnceLock<regex::Regex> = OnceLock::new();

pub struct Fetcher {
    db: Database,
    reddit: RedditClient,
    scryfall: ScryfallClient,
    dedup: Deduplicator,
}

impl Fetcher {
    pub async fn new(db: Database, subreddit: &str, scryfall_rate_limit_ms: u64) -> Result<Self> {
        let dedup = Deduplicator::from_db(&db).await?;
        Ok(Self {
            db,
            reddit: RedditClient::with_subreddit(subreddit),
            scryfall: ScryfallClient::with_config("https://api.scryfall.com", scryfall_rate_limit_ms),
            dedup,
        })
    }

    /// Fetches from Reddit, cross-references with Scryfall, and saves new cards to the DB.
    /// Returns the number of new cards saved this cycle.
    pub async fn fetch_once(&mut self) -> Result<usize> {
        info!("Starting on-demand fetch cycle");

        let posts = self.reddit.fetch_recent().await?;
        info!("Fetched {} posts from Reddit", posts.len());

        let mut new_count = 0usize;

        for post in posts {
            let score = AuthenticityScorer::score_reddit(&post, &[]);

            if score < 0.3 {
                warn!("Skipping low-score post: {} (score: {})", post.title, score);
                continue;
            }

            let card_name = post.extract_card_name()
                .unwrap_or_else(|| post.title.clone());

            // Check for duplicates
            if let Some(existing_id) = self.dedup.check_duplicate(&card_name, Some(&post.url), None) {
                info!("Duplicate detected: {} matches {}", card_name, existing_id);
                self.db.add_source(&existing_id, &post.to_source(score)).await?;
                continue;
            }

            // Try Scryfall lookup — errors are treated as Unverified (non-fatal)
            let scryfall_result = self.scryfall.fetch_by_name(&card_name).await;
            match scryfall_result {
                Ok(Some(mut scryfall_card)) => {
                    scryfall_card.confidence = Confidence::Verified;
                    scryfall_card.sources.push(post.to_source(score));
                    scryfall_card.first_seen = post.created_utc;
                    scryfall_card.last_updated = Utc::now();
                    self.dedup.register(&scryfall_card, None);
                    self.db.save_card(&scryfall_card).await?;
                    info!("Saved verified card: {}", scryfall_card.name);
                    new_count += 1;
                }
                Ok(None) => {
                    info!("Card '{}' not found on Scryfall — saving as Unverified", card_name);
                    let set_code = extract_set_from_title(&post.title)
                        .unwrap_or_else(|| "UNK".to_string());
                    let card = Card {
                        id: format!("reddit_{}", post.id),
                        name: card_name.clone(),
                        mana_cost: None,
                        set_code,
                        card_type: None,
                        text: None,
                        power_toughness: None,
                        loyalty: None,
                        image_url: None,
                        confidence: Confidence::Unverified,
                        sources: vec![post.to_source(score)],
                        first_seen: post.created_utc,
                        last_updated: Utc::now(),
                        is_fake: false,
                    };
                    self.dedup.register(&card, None);
                    self.db.save_card(&card).await?;
                    info!("Saved unverified card: {}", card.name);
                    new_count += 1;
                }
                Err(e) => {
                    warn!("Scryfall lookup failed for '{}': {} — saving as Unverified", card_name, e);
                    let set_code = extract_set_from_title(&post.title)
                        .unwrap_or_else(|| "UNK".to_string());
                    let card = Card {
                        id: format!("reddit_{}", post.id),
                        name: card_name.clone(),
                        mana_cost: None,
                        set_code,
                        card_type: None,
                        text: None,
                        power_toughness: None,
                        loyalty: None,
                        image_url: None,
                        confidence: Confidence::Unverified,
                        sources: vec![post.to_source(score)],
                        first_seen: post.created_utc,
                        last_updated: Utc::now(),
                        is_fake: false,
                    };
                    self.dedup.register(&card, None);
                    self.db.save_card(&card).await?;
                    info!("Saved unverified card: {}", card.name);
                    new_count += 1;
                }
            }
        }

        info!("Fetch cycle complete: {} new card(s)", new_count);
        Ok(new_count)
    }
}

fn extract_set_from_title(title: &str) -> Option<String> {
    let re = RE_SET_CODE.get_or_init(|| {
        regex::Regex::new(r"\[(\w{2,4})\]").expect("RE_SET_CODE regex is valid")
    });
    re.captures(title)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_uppercase())
}

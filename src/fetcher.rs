use tokio::time::{interval, Duration};
use crate::api::reddit::RedditClient;
use crate::db::Database;
use crate::filter::{dedup::Deduplicator, verify::AuthenticityScorer};
use crate::models::card::{Card, Confidence};
use tracing::{info, warn, error};

pub struct BackgroundFetcher {
    db: Database,
    reddit: RedditClient,
    dedup: Deduplicator,
    interval: Duration,
}

impl BackgroundFetcher {
    pub fn new(db: Database, tick_minutes: u64) -> Self {
        Self {
            db,
            reddit: RedditClient::new(),
            dedup: Deduplicator::new(),
            interval: Duration::from_secs(tick_minutes * 60),
        }
    }

    pub async fn run(mut self) {
        let mut ticker = interval(self.interval);

        info!("Background fetcher started");

        loop {
            ticker.tick().await;

            if let Err(e) = self.fetch_cycle().await {
                error!("Fetch cycle failed: {}", e);
            }
        }
    }

    async fn fetch_cycle(&mut self) -> anyhow::Result<()> {
        info!("Starting fetch cycle");

        // Fetch from Reddit
        let posts = self.reddit.fetch_recent().await?;
        info!("Fetched {} posts from Reddit", posts.len());

        for post in posts {
            // Score authenticity
            let score = AuthenticityScorer::score_reddit(&post, &[]); // Load trusted users from config

            if score < 0.3 {
                warn!("Skipping low-score post: {} (score: {})", post.title, score);
                continue;
            }

            // Extract card name
            let card_name = post.extract_card_name()
                .unwrap_or_else(|| post.title.clone());

            // Check for duplicates
            // In real impl: fetch image and hash it
            if let Some(existing_id) = self.dedup.check_duplicate(&card_name, Some(&post.url), None) {
                info!("Duplicate detected: {} matches {}", card_name, existing_id);
                // Add source to existing card
                self.db.add_source(&existing_id, &post.to_source(score)).await?;
                continue;
            }

            // Create new card
            let set_code = extract_set_from_title(&post.title)
                .unwrap_or_else(|| "UNKNOWN".to_string());

            let card = Card {
                id: format!("reddit_{}", post.id),
                name: card_name.clone(),
                mana_cost: None, // Would need OCR
                set_code,
                card_type: None,
                text: None,
                power_toughness: None,
                loyalty: None,
                image_url: Some(post.url.clone()),
                confidence: Confidence::Rumor,
                sources: vec![post.to_source(score)],
                first_seen: post.created_utc,
                last_updated: post.created_utc,
                is_fake: false,
            };

            // Register in dedup index
            self.dedup.register(&card, None);

            // Save to DB
            self.db.save_card(&card).await?;
            info!("Saved new card: {}", card.name);
        }

        // Also fetch from Scryfall for confirmations
        // self.check_scryfall_confirmations().await?;

        Ok(())
    }
}

fn extract_set_from_title(title: &str) -> Option<String> {
    use regex::Regex;
    let re = Regex::new(r"\[(\w{2,4})\]").ok()?;
    re.captures(title)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_uppercase())
}

use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::models::card::{Card, Confidence, Platform, Source};

pub struct ScryfallClient {
    client: Client,
    api_url: String,
    rate_limit_ms: u64,
}

/// Minimal Scryfall card response fields we care about
#[derive(Debug, Deserialize)]
struct ScryfallCard {
    id: String,
    name: String,
    set: String,
    mana_cost: Option<String>,
    type_line: Option<String>,
    oracle_text: Option<String>,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<String>,
    image_uris: Option<ScryfallImageUris>,
    scryfall_uri: String,
}

#[derive(Debug, Deserialize)]
struct ScryfallImageUris {
    normal: Option<String>,
    small: Option<String>,
}

/// Response from /cards/search
#[derive(Debug, Deserialize)]
struct ScryfallSearchResponse {
    data: Vec<ScryfallCard>,
    has_more: bool,
    next_page: Option<String>,
    total_cards: Option<u32>,
}

impl ScryfallClient {
    pub fn new() -> Self {
        Self::with_config("https://api.scryfall.com", 100)
    }

    pub fn with_config(api_url: &str, rate_limit_ms: u64) -> Self {
        let client = Client::builder()
            .user_agent("mtg-spoiler-tui/0.1 (github.com/yourusername/mtg-spoiler-tui)")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_url: api_url.to_string(),
            rate_limit_ms,
        }
    }

    /// Fetch a card by exact name from Scryfall. Returns None if not found.
    pub async fn fetch_by_name(&self, name: &str) -> Result<Option<Card>> {
        sleep(Duration::from_millis(self.rate_limit_ms)).await;

        let url = format!("{}/cards/named", self.api_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("fuzzy", name)])
            .send()
            .await?;

        if resp.status() == 404 {
            return Ok(None);
        }

        if !resp.status().is_success() {
            warn!("Scryfall name lookup failed for '{}': {}", name, resp.status());
            return Ok(None);
        }

        let sc: ScryfallCard = resp.json().await?;
        Ok(Some(scryfall_to_card(sc)))
    }

    /// Search Scryfall for recently previewed cards in a set.
    pub async fn fetch_previewed_cards(&self, set_code: &str) -> Result<Vec<Card>> {
        sleep(Duration::from_millis(self.rate_limit_ms)).await;

        let query = format!("s:{} is:preview", set_code);
        let url = format!("{}/cards/search", self.api_url);

        let mut cards = Vec::new();
        let mut next_page: Option<String> = None;

        loop {
            let resp = if let Some(ref page_url) = next_page {
                self.client.get(page_url).send().await?
            } else {
                self.client
                    .get(&url)
                    .query(&[("q", &query), ("order", &"released".to_string())])
                    .send()
                    .await?
            };

            if resp.status() == 404 {
                // No preview cards found for this set
                break;
            }

            if !resp.status().is_success() {
                warn!("Scryfall search failed for set '{}': {}", set_code, resp.status());
                break;
            }

            let body: ScryfallSearchResponse = resp.json().await?;
            info!(
                "Scryfall: fetched {} preview cards for {}",
                body.data.len(),
                set_code
            );

            for sc in body.data {
                cards.push(scryfall_to_card(sc));
            }

            if body.has_more {
                next_page = body.next_page;
                sleep(Duration::from_millis(self.rate_limit_ms)).await;
            } else {
                break;
            }
        }

        Ok(cards)
    }
}

fn scryfall_to_card(sc: ScryfallCard) -> Card {
    let image_url = sc
        .image_uris
        .as_ref()
        .and_then(|u| u.normal.clone().or(u.small.clone()));

    let power_toughness = match (sc.power, sc.toughness) {
        (Some(p), Some(t)) => Some(format!("{}/{}", p, t)),
        _ => None,
    };

    let now = Utc::now();
    let source = Source {
        platform: Platform::Scryfall,
        url: sc.scryfall_uri.clone(),
        author: Some("Scryfall".to_string()),
        posted_at: now,
        raw_title: format!("{} ({}) — official Scryfall listing", sc.name, sc.set),
        upvotes: None,
        authenticity_score: 1.0, // Scryfall = fully confirmed
    };

    Card {
        id: format!("scryfall_{}", sc.id),
        name: sc.name,
        set_code: sc.set.to_uppercase(),
        mana_cost: sc.mana_cost,
        card_type: sc.type_line,
        text: sc.oracle_text,
        power_toughness,
        loyalty: sc.loyalty,
        image_url,
        confidence: Confidence::Verified,
        sources: vec![source],
        first_seen: now,
        last_updated: now,
        is_fake: false,
    }
}

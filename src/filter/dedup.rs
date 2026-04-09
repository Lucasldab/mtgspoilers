use std::collections::HashMap;

use crate::models::card::Card;

pub struct Deduplicator {
    // Card name -> canonical card ID
    name_index: HashMap<String, String>,
    // Normalized URL -> card ID
    url_index: HashMap<String, String>,
}

impl Deduplicator {
    pub fn new() -> Self {
        Self {
            name_index: HashMap::new(),
            url_index: HashMap::new(),
        }
    }

    /// Check if this is a duplicate and return the canonical ID if so.
    /// image_data parameter is retained for API compatibility but image hashing is disabled.
    pub fn check_duplicate(&self, card_name: &str, image_url: Option<&str>, _image_data: Option<&()>) -> Option<String> {
        // Check 1: Exact name match
        if let Some(id) = self.name_index.get(&normalize_name(card_name)) {
            return Some(id.clone());
        }

        // Check 2: URL normalization
        if let Some(url) = image_url {
            let canonical = normalize_url(url);
            if let Some(id) = self.url_index.get(&canonical) {
                return Some(id.clone());
            }
        }

        None
    }

    /// Register a new card in the deduplication index.
    /// image_data parameter is retained for API compatibility but image hashing is disabled.
    pub fn register(&mut self, card: &Card, _image_data: Option<&()>) {
        self.name_index.insert(normalize_name(&card.name), card.id.clone());

        if let Some(url) = &card.image_url {
            self.url_index.insert(normalize_url(url), card.id.clone());
        }
    }
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "")
}

fn normalize_url(url: &str) -> String {
    // Remove query params, normalize domains
    url.split('?').next()
        .unwrap_or(url)
        .replace("i.redd.it", "reddit.com")
        .replace("preview.redd.it", "reddit.com")
        .to_lowercase()
}

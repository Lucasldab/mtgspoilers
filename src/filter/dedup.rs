use std::collections::HashMap;
use image::{DynamicImage, GenericImageView};
use img_hash::{HasherConfig, ImageHash};

use crate::models::card::Card;

pub struct Deduplicator {
    // Card name -> canonical card ID
    name_index: HashMap<String, String>,
    // Perceptual hash -> card ID
    image_hashes: HashMap<ImageHash, String>,
    // Normalized URL -> card ID
    url_index: HashMap<String, String>,
    hasher: img_hash::Hasher,
}

impl Deduplicator {
    pub fn new() -> Self {
        let hasher = HasherConfig::new()
            .hash_size(16, 16)
            .to_hasher();

        Self {
            name_index: HashMap::new(),
            image_hashes: HashMap::new(),
            url_index: HashMap::new(),
            hasher,
        }
    }

    /// Check if this is a duplicate and return the canonical ID if so
    pub fn check_duplicate(&self, card_name: &str, image_url: Option<&str>, image_data: Option<&DynamicImage>) -> Option<String> {
        // Check 1: Exact name match
        if let Some(id) = self.name_index.get(&normalize_name(card_name)) {
            return Some(id.clone());
        }

        // Check 2: Image perceptual hash
        if let Some(img) = image_data {
            let hash = self.hasher.hash_image(img);
            if let Some((existing_hash, id)) = self.find_similar_hash(&hash) {
                tracing::debug!("Image hash match: {} similar to existing", card_name);
                return Some(id.clone());
            }
        }

        // Check 3: URL normalization
        if let Some(url) = image_url {
            let canonical = normalize_url(url);
            if let Some(id) = self.url_index.get(&canonical) {
                return Some(id.clone());
            }
        }

        None
    }

    /// Register a new card in the deduplication index
    pub fn register(&mut self, card: &Card, image_data: Option<&DynamicImage>) {
        self.name_index.insert(normalize_name(&card.name), card.id.clone());

        if let Some(url) = &card.image_url {
            self.url_index.insert(normalize_url(url), card.id.clone());
        }

        if let Some(img) = image_data {
            let hash = self.hasher.hash_image(img);
            self.image_hashes.insert(hash, card.id.clone());
        }
    }

    fn find_similar_hash(&self, target: &ImageHash) -> Option<(&ImageHash, &String)> {
        // Find hash with Hamming distance < 5 (similar but not identical)
        self.image_hashes.iter()
            .find(|(hash, _)| target.dist(hash) < 5)
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

/// Fetch and hash image from URL
pub async fn fetch_and_hash(url: &str) -> anyhow::Result<Option<(DynamicImage, ImageHash)>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;

    let img = image::load_from_memory(&bytes)?;
    let hasher = HasherConfig::new().to_hasher();
    let hash = hasher.hash_image(&img);

    Ok(Some((img, hash)))
}

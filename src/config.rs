use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub reddit: RedditConfig,
    pub scryfall: ScryfallConfig,
    pub filter: FilterConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub tick_rate_ms: u64,
    pub auto_refresh_minutes: u64,
}

#[derive(Debug, Deserialize)]
pub struct RedditConfig {
    pub subreddit: String,
    pub fetch_limit: u32,
    pub trusted_users: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScryfallConfig {
    pub api_url: String,
    pub rate_limit_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct FilterConfig {
    pub min_authenticity_score: f32,
    pub dedup_hash_threshold: u32,
    pub hide_fake_by_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub confidence_colors: bool,
    pub show_thumbnails: bool,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub reddit: RedditConfig,
    #[serde(default)]
    pub scryfall: ScryfallConfig,
    #[serde(default)]
    pub filter: FilterConfig,
    #[serde(default)]
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

impl Default for AppConfig {
    fn default() -> Self {
        Self { tick_rate_ms: 250, auto_refresh_minutes: 0 }
    }
}

impl Default for RedditConfig {
    fn default() -> Self {
        Self {
            subreddit: "magicTCG".to_string(),
            fetch_limit: 25,
            trusted_users: vec![],
        }
    }
}

impl Default for ScryfallConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.scryfall.com".to_string(),
            rate_limit_ms: 100,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_authenticity_score: 0.3,
            dedup_hash_threshold: 10,
            hide_fake_by_default: true,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self { confidence_colors: true, show_thumbnails: false }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            reddit: RedditConfig::default(),
            scryfall: ScryfallConfig::default(),
            filter: FilterConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    /// Load config from config.toml, falling back to defaults if the file is missing or invalid.
    /// Never returns an error — always returns a usable Config.
    pub fn load_or_default() -> Self {
        match Self::load() {
            Ok(config) => {
                tracing::info!("Config loaded from config.toml");
                config
            }
            Err(e) => {
                tracing::warn!("config.toml not found or invalid ({}), using defaults", e);
                Self::default()
            }
        }
    }
}

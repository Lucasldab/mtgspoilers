use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Confidence {
    Rumor,      // Reddit only, unverified
    Probable,   // Image analysis passed
    Confirmed,  // Multiple sources
    Official,   // WotC/Scryfall
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::Rumor => "🟡 Rumor",
            Confidence::Probable => "🔵 Probable",
            Confidence::Confirmed => "🟢 Confirmed",
            Confidence::Official => "⚪ Official",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Confidence::Rumor => Color::Yellow,
            Confidence::Probable => Color::Blue,
            Confidence::Confirmed => Color::Green,
            Confidence::Official => Color::White,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub mana_cost: Option<String>,
    pub set_code: String,
    pub card_type: Option<String>,
    pub text: Option<String>,
    pub power_toughness: Option<String>,
    pub loyalty: Option<String>,
    pub image_url: Option<String>,
    pub confidence: Confidence,
    pub sources: Vec<Source>,
    pub first_seen: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub is_fake: bool,  // User-marked or auto-detected fake
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub platform: Platform,
    pub url: String,
    pub author: Option<String>,
    pub posted_at: DateTime<Utc>,
    pub raw_title: String,
    pub upvotes: Option<i32>,
    pub authenticity_score: f32,  // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Reddit,
    Scryfall,
    WotC,
    MythicSpoiler,
    Unknown,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Reddit => write!(f, "Reddit"),
            Platform::Scryfall => write!(f, "Scryfall"),
            Platform::WotC => write!(f, "WotC"),
            Platform::MythicSpoiler => write!(f, "MythicSpoiler"),
            Platform::Unknown => write!(f, "Unknown"),
        }
    }
}

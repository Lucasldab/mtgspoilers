use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};
use crate::models::card::{Card, Confidence, Source, Platform};
use anyhow::Result;
use chrono::{DateTime, Utc, NaiveDateTime};

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Self::init_tables(&pool).await?;

        Ok(Self { pool })
    }

    async fn init_tables(pool: &Pool<Sqlite>) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cards (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                mana_cost TEXT,
                set_code TEXT NOT NULL,
                card_type TEXT,
                text TEXT,
                power_toughness TEXT,
                loyalty TEXT,
                image_url TEXT,
                confidence TEXT NOT NULL,
                is_fake BOOLEAN DEFAULT FALSE,
                first_seen TIMESTAMP NOT NULL,
                last_updated TIMESTAMP NOT NULL
            )
            "#
        ).execute(pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                url TEXT NOT NULL,
                author TEXT,
                posted_at TIMESTAMP NOT NULL,
                raw_title TEXT NOT NULL,
                upvotes INTEGER,
                authenticity_score REAL NOT NULL,
                FOREIGN KEY (card_id) REFERENCES cards(id) ON DELETE CASCADE
            )
            "#
        ).execute(pool).await?;

        Ok(())
    }

    pub async fn save_card(&self, card: &Card) -> Result<()> {
        let conf_str = format!("{:?}", card.confidence);
        let first_seen = card.first_seen.naive_utc();
        let last_updated = card.last_updated.naive_utc();

        sqlx::query(
            r#"
            INSERT INTO cards (id, name, mana_cost, set_code, card_type, text,
                             power_toughness, loyalty, image_url, confidence,
                             is_fake, first_seen, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                mana_cost = excluded.mana_cost,
                card_type = excluded.card_type,
                text = excluded.text,
                image_url = excluded.image_url,
                confidence = excluded.confidence,
                is_fake = excluded.is_fake,
                last_updated = excluded.last_updated
            "#
        )
        .bind(&card.id)
        .bind(&card.name)
        .bind(&card.mana_cost)
        .bind(&card.set_code)
        .bind(&card.card_type)
        .bind(&card.text)
        .bind(&card.power_toughness)
        .bind(&card.loyalty)
        .bind(&card.image_url)
        .bind(&conf_str)
        .bind(card.is_fake)
        .bind(first_seen)
        .bind(last_updated)
        .execute(&self.pool)
        .await?;

        for source in &card.sources {
            self.save_source(&card.id, source).await?;
        }

        Ok(())
    }

    pub async fn save_source(&self, card_id: &str, source: &Source) -> Result<()> {
        let posted_at = source.posted_at.naive_utc();

        sqlx::query(
            r#"
            INSERT INTO sources (card_id, platform, url, author, posted_at,
                               raw_title, upvotes, authenticity_score)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#
        )
        .bind(card_id)
        .bind(format!("{:?}", source.platform))
        .bind(&source.url)
        .bind(&source.author)
        .bind(posted_at)
        .bind(&source.raw_title)
        .bind(source.upvotes)
        .bind(source.authenticity_score)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_cards(&self, filter: &CardFilter) -> Result<Vec<Card>> {
        let mut query = String::from(
            "SELECT id, name, mana_cost, set_code, card_type, text,
                    power_toughness, loyalty, image_url, confidence,
                    is_fake, first_seen, last_updated FROM cards WHERE 1=1"
        );

        if let Some(conf) = &filter.confidence {
            query.push_str(&format!(" AND confidence = '{:?}'", conf));
        }

        if let Some(set) = &filter.set_code {
            // Sanitize: only allow alphanumeric set codes
            let safe_set: String = set.chars().filter(|c| c.is_alphanumeric()).collect();
            query.push_str(&format!(" AND set_code = '{}'", safe_set));
        }

        if filter.hide_fake {
            query.push_str(" AND is_fake = false");
        }

        if let Some(search) = &filter.search {
            // Sanitize search: escape single quotes
            let safe_search = search.replace('\'', "''");
            query.push_str(&format!(
                " AND (name LIKE '%{}%' OR text LIKE '%{}%' OR set_code LIKE '%{}%')",
                safe_search, safe_search, safe_search
            ));
        }

        query.push_str(" ORDER BY first_seen DESC");

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let mut result = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let sources = self.get_sources(&id).await?;

            let first_seen: NaiveDateTime = row.try_get("first_seen")?;
            let last_updated: NaiveDateTime = row.try_get("last_updated")?;

            result.push(Card {
                id,
                name: row.try_get("name")?,
                mana_cost: row.try_get("mana_cost").ok(),
                set_code: row.try_get("set_code")?,
                card_type: row.try_get("card_type").ok(),
                text: row.try_get("text").ok(),
                power_toughness: row.try_get("power_toughness").ok(),
                loyalty: row.try_get("loyalty").ok(),
                image_url: row.try_get("image_url").ok(),
                confidence: parse_confidence(&row.try_get::<String, _>("confidence")?),
                sources,
                first_seen: DateTime::from_naive_utc_and_offset(first_seen, Utc),
                last_updated: DateTime::from_naive_utc_and_offset(last_updated, Utc),
                is_fake: row.try_get("is_fake")?,
            });
        }

        Ok(result)
    }

    pub async fn get_sources(&self, card_id: &str) -> Result<Vec<Source>> {
        let rows = sqlx::query(
            "SELECT platform, url, author, posted_at, raw_title, upvotes, authenticity_score
             FROM sources WHERE card_id = ?1"
        )
        .bind(card_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sources = Vec::new();
        for row in rows {
            let posted_at: NaiveDateTime = row.try_get("posted_at")?;

            sources.push(Source {
                platform: parse_platform(&row.try_get::<String, _>("platform")?),
                url: row.try_get("url")?,
                author: row.try_get("author").ok(),
                posted_at: DateTime::from_naive_utc_and_offset(posted_at, Utc),
                raw_title: row.try_get("raw_title")?,
                upvotes: row.try_get("upvotes").ok(),
                authenticity_score: row.try_get("authenticity_score")?,
            });
        }

        Ok(sources)
    }

    pub async fn add_source(&self, card_id: &str, source: &Source) -> Result<()> {
        self.save_source(card_id, source).await
    }

    pub async fn mark_fake(&self, card_id: &str, is_fake: bool) -> Result<()> {
        sqlx::query("UPDATE cards SET is_fake = ?1 WHERE id = ?2")
            .bind(is_fake)
            .bind(card_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

fn parse_confidence(s: &str) -> Confidence {
    match s {
        "Verified" => Confidence::Verified,
        _ => Confidence::Unverified,
    }
}

fn parse_platform(s: &str) -> Platform {
    match s {
        "Reddit" => Platform::Reddit,
        "Scryfall" => Platform::Scryfall,
        "WotC" => Platform::WotC,
        "MythicSpoiler" => Platform::MythicSpoiler,
        _ => Platform::Unknown,
    }
}

pub struct CardFilter {
    pub confidence: Option<Confidence>,
    pub set_code: Option<String>,
    pub hide_fake: bool,
    pub search: Option<String>,
}

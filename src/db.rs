use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use crate::models::card::{Card, Confidence, Source, Platform};
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn save_card(&self, card: &Card) -> Result<()> {
        sqlx::query!(
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
            "#,
            card.id,
            card.name,
            card.mana_cost,
            card.set_code,
            card.card_type,
            card.text,
            card.power_toughness,
            card.loyalty,
            card.image_url,
            format!("{:?}", card.confidence),
            card.is_fake,
            card.first_seen,
            card.last_updated
        )
        .execute(&self.pool)
        .await?;

        // Save sources
        for source in &card.sources {
            self.save_source(&card.id, source).await?;
        }

        Ok(())
    }

    pub async fn get_cards(&self, filter: &CardFilter) -> Result<Vec<Card>> {
        let mut query = String::from(
            "SELECT * FROM cards WHERE 1=1"
        );

        if let Some(conf) = &filter.confidence {
            query.push_str(&format!(" AND confidence = '{:?}'", conf));
        }

        if let Some(set) = &filter.set_code {
            query.push_str(" AND set_code = ?");
        }

        if filter.hide_fake {
            query.push_str(" AND is_fake = false");
        }

        query.push_str(" ORDER BY first_seen DESC");

        let cards = sqlx::query_as::<_, CardRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        // Load sources for each card
        let mut result = Vec::new();
        for row in cards {
            let sources = self.get_sources(&row.id).await?;
            result.push(row.to_card(sources));
        }

        Ok(result)
    }

    pub async fn mark_fake(&self, card_id: &str, is_fake: bool) -> Result<()> {
        sqlx::query!(
            "UPDATE cards SET is_fake = ?1 WHERE id = ?2",
            is_fake,
            card_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

pub struct CardFilter {
    pub confidence: Option<Confidence>,
    pub set_code: Option<String>,
    pub hide_fake: bool,
    pub search: Option<String>,
}

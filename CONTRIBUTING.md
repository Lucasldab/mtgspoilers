# Contributing

## Dev setup

**Prerequisites:** Rust stable (1.75+), SQLite dev headers.

```bash
git clone https://github.com/Lucasldab/mtgspoilers
cd mtgspoilers
cargo build
```

The app reads `config.toml` on startup and falls back to built-in defaults if the file is missing — no config file required for development.

**Required env vars:** none at runtime. If you add a source that calls an authenticated API, document its key in the PR body and read it via `std::env::var`.

## Running tests

```bash
cargo test
```

Integration tests hit a real SQLite in-memory DB (`sqlx`). Do not mock the database — past experience showed mock/prod divergence masking broken migrations.

## Project layout

```
src/
  api/          # External API clients (one file per source)
  filter/       # Dedup and authenticity scoring
  models/       # Card, Source, Platform, Confidence types
  db.rs         # SQLite persistence via sqlx
  fetcher.rs    # Orchestrates the full fetch pipeline
  app.rs        # TUI state machine
  config.rs     # config.toml loader with typed defaults
  main.rs       # Entry point — wires everything together
migrations/     # SQL schema files
```

## The 4-tier confidence system

Cards flow through four dispositions:

| Tier | Label | Saved? | Key fields |
|------|-------|--------|------------|
| 0 | Rejected | No | `authenticity_score < 0.3`; currently unreachable (scorer stub always returns 0.5) |
| 1 | Unverified | Yes | `confidence = Unverified`, `id = "reddit_*"` |
| 2 | Verified | Yes | `confidence = Verified`, full Scryfall metadata |
| 3 | Fake | Yes (hidden) | `is_fake = true` |

The pipeline lives in `src/fetcher.rs`:

1. Source client produces posts/items.
2. `AuthenticityScorer::score_reddit()` gates Tier 0 rejects (`filter/verify.rs`).
3. `Deduplicator` checks name + URL indexes (`filter/dedup.rs`).
4. `ScryfallClient::fetch_by_name()` upgrades a card to Verified if found.
5. `Database::save_card()` persists the result.

## Adding a new spoiler source

A "source" is anything that produces candidate card names — a website, Discord feed, Twitter scraper, etc.

### 1. Create the API client

Add `src/api/mysource.rs` modelled on `src/api/reddit.rs`:

```rust
pub struct MySourceClient { /* config fields */ }

impl MySourceClient {
    pub fn new(/* ... */) -> Self { /* ... */ }

    /// Returns raw posts/items from the source.
    pub async fn fetch_recent(&mut self) -> anyhow::Result<Vec<MySourcePost>> { /* ... */ }
}

pub struct MySourcePost {
    pub id: String,
    pub title: String,
    pub url: String,
    pub created_utc: chrono::DateTime<chrono::Utc>,
    // ...
}

impl MySourcePost {
    /// Convert to the shared `Source` type for DB persistence.
    pub fn to_source(&self, score: f32) -> crate::models::card::Source { /* ... */ }
}
```

Register it in `src/api/mod.rs`:

```rust
pub mod mysource;
```

### 2. Add config

Add a struct in `src/config.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct MySourceConfig {
    pub enabled: bool,
    // source-specific fields
}

impl Default for MySourceConfig {
    fn default() -> Self { Self { enabled: false } }
}
```

Add a field to `Config` and its `Default` impl.

### 3. Wire into the fetcher

In `src/fetcher.rs`, add your client as a field on `Fetcher` and call `fetch_recent()` inside `fetch_once()`. After fetching, run each item through the same pipeline:

```
score → dedup check → Scryfall lookup → save_card
```

Use `Platform::Reddit` as a reference; add a new `Platform` variant in `src/models/card.rs` if needed.

### 4. Test it

- Unit-test the client's post-to-card-name extraction.
- Add an integration test that feeds mock posts through `Fetcher` and asserts the correct tier is assigned.

## Submitting a PR

Open a draft PR early. The PR description should cover:
- What source was added and why it produces useful spoiler signal.
- Any new env vars or config keys (with example values).
- Known limitations or edge cases.

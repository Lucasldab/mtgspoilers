# Codebase Scan

## (a) Modules and Responsibilities

| Module | File(s) | Responsibility |
|--------|---------|---------------|
| `main` | `src/main.rs` | Entry point. Wires terminal (crossterm/ratatui), DB, Fetcher, and App together. Runs the 250 ms draw+event loop. Purges cards older than 7 days on startup. |
| `config` | `src/config.rs` | Loads `config.toml` (TOML) with `load_or_default()` — never panics. Sections: `AppConfig`, `RedditConfig`, `ScryfallConfig`, `FilterConfig`, `DisplayConfig`. |
| `models` | `src/models/card.rs` | Core data types: `Card`, `Source`, `Platform` enum, `Confidence` enum (binary: `Verified` / `Unverified`). `Source.authenticity_score: f32` carries per-source signal independent of card-level confidence. |
| `api::reddit` | `src/api/reddit.rs` | `RedditClient` wraps the `roux` crate. `fetch_recent()` pulls 25 hot posts, filters by spoiler keyword / flair, deduplicates by `seen_ids`. `RedditPost::extract_card_name()` applies two compiled regexes (stored in `OnceLock`) to extract a structured name from the post title. |
| `api::scryfall` | `src/api/scryfall.rs` | `ScryfallClient` with configurable rate-limit sleep. `fetch_by_name()` uses fuzzy `/cards/named`. `fetch_previewed_cards()` pages through `/cards/search?q=s:{set} is:preview`. Converts `ScryfallCard` JSON into domain `Card`. |
| `filter::dedup` | `src/filter/dedup.rs` | In-memory `Deduplicator` with two `HashMap`s: `name_index` (normalized name → card ID) and `url_index` (normalized URL → card ID). Seeded from DB on startup via `from_db()`. `check_duplicate()` runs both checks; `register()` inserts into both indexes. |
| `filter::verify` | `src/filter/verify.rs` | `AuthenticityScorer` stub. Returns a fixed `0.5` for every Reddit post. Heuristic scoring was removed; the struct is kept only to avoid import churn in `fetcher.rs`. |
| `fetcher` | `src/fetcher.rs` | `Fetcher` orchestrates the full pipeline: Reddit fetch → dedup check → Scryfall lookup → DB persist. Owns `Deduplicator`, `RedditClient`, and `ScryfallClient`. `extract_set_from_title()` regex (`OnceLock`) pulls a 2–4 character set code from `[SET]` bracketed titles. |
| `db` | `src/db.rs` | `Database` over SQLite via `sqlx`. Key methods: `save_card` (upsert), `save_source`, `get_cards` (dynamic filter query via `QueryBuilder`), `get_sources`, `mark_fake`, `purge_old_cards`, `get_all_card_stubs`. Schema created inline in code and also in `migrations/001_initial.sql`. |
| `app` | `src/app.rs` | `App` is the TUI state machine. Modes: `Normal`, `Search`, `Filter`, `Detail`, `MarkingFake`. Flags `needs_fetch` and `needs_refresh` decouple key events from async work; `pending_action` drains in `tick()`. |
| `ui` | `src/ui.rs` | Ratatui render pass: 3-row layout (header / main / footer). Main area splits into 40 % card list + 60 % detail panel. Modal overlays for search, filter, and mark-fake confirmation. |

---

## (b) Confidence System Flow

The system has **four distinct disposition tiers**, though the `Confidence` enum is binary. Two tiers live in the pipeline before a card is persisted; two are post-persist states.

```
Reddit post
    │
    ▼
[Tier 0 — Score gate]                           fetcher.rs:43-48
  AuthenticityScorer::score_reddit() → 0.5      filter/verify.rs:9
  if score < 0.3 → warn + skip                  fetcher.rs:45-47
  (Currently dead branch: scorer always returns 0.5)
    │
    ▼
[Dedup check]                                   fetcher.rs:54-58
  Deduplicator::check_duplicate(name, url)       filter/dedup.rs:36-51
  Hit → add source to existing card, continue   fetcher.rs:55-57
  Miss → proceed to Scryfall
    │
    ▼
[Scryfall lookup]                               fetcher.rs:61
  ScryfallClient::fetch_by_name(&name)          api/scryfall.rs:66-88
    │
    ├── Ok(Some(card)) ──────────────────────── fetcher.rs:63-70
    │     Confidence::Verified                  models/card.rs:6-7
    │     authenticity_score = 1.0 (source)     api/scryfall.rs:163
    │     save_card() → DB                      fetcher.rs:69
    │
    ├── Ok(None) / 404 ─────────────────────── fetcher.rs:73-96
    │     Confidence::Unverified                models/card.rs:8
    │     id = "reddit_{post.id}"               fetcher.rs:78
    │     set_code = extract_set_from_title()   fetcher.rs:75-76
    │     or "UNK" fallback                     fetcher.rs:76
    │     save_card() → DB                      fetcher.rs:94
    │
    └── Err(e) ──────────────────────────────── fetcher.rs:98-122
          warn + treat same as Ok(None)
          Confidence::Unverified
    │
    ▼
[Tier 3 — User-marked Fake]                     app.rs:226-231
  Key 'x' → 'y' → PendingAction::MarkFake       app.rs:226-231
  db.mark_fake(card_id, true)                    db.rs:227-234
  is_fake = true persisted                       migrations/001_initial.sql:12
  Filtered from display by default               db.rs:148-149
```

**Summary table:**

| Tier | Label | Persistence | Key fields |
|------|-------|-------------|-----------|
| 0 | Rejected | Never saved | score < 0.3; currently unreachable (scorer stub) |
| 1 | Unverified | Saved | `confidence = "Unverified"`, `id = "reddit_*"`, minimal metadata |
| 2 | Verified | Saved | `confidence = "Verified"`, full Scryfall metadata, `authenticity_score = 1.0` |
| 3 | Fake | Saved, hidden | `is_fake = true`, any confidence level |

---

## (c) Dedup Edge Case: Same Card Name, Multiple Sets (Reprint Collision)

### Description

`Deduplicator.name_index` maps `normalize_name(card.name) → card_id` (`filter/dedup.rs:38, 56`).

`normalize_name` strips all non-alphanumeric characters and lowercases (`filter/dedup.rs:64-67`). The key is **name only** — there is no set-code component.

MTG regularly reprints cards across sets. "Lightning Bolt" appears in Alpha, M10, MH3, and many others. If a spoiler post for MH3 is processed first, the second post for a DMR reprint of the same card hits the name-index and is treated as a duplicate (`fetcher.rs:54-57`): the code calls `db.add_source(&existing_id, ...)` and skips the new card entirely.

The consequence is:
1. The MH3 card absorbs the DMR spoiler as an additional source.
2. The DMR version is never saved as a distinct card, so users never see it in the card list or detail view.
3. The set filter in `get_cards()` (`db.rs:143-146`) becomes misleading — filtering by "DMR" returns zero results even though a real spoiler exists.

### Why it matters

The dedup index has no TTL or set scope. Once a card name is registered (even from a temporary "UNK" unverified entry), all future posts with that name collapse into it regardless of set. A card that becomes unverified first and then re-appears in Scryfall data for a different set will never be upgraded to Verified because `check_duplicate` fires before the Scryfall lookup (`fetcher.rs:54` vs `fetcher.rs:61`).

### Proposed Test

```rust
// filter/dedup.rs — unit test module

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::card::{Card, Confidence, Source, Platform};
    use chrono::Utc;

    fn make_card(id: &str, name: &str, set: &str) -> Card {
        Card {
            id: id.to_string(),
            name: name.to_string(),
            set_code: set.to_string(),
            mana_cost: None,
            card_type: None,
            text: None,
            power_toughness: None,
            loyalty: None,
            image_url: None,
            confidence: Confidence::Unverified,
            sources: vec![],
            first_seen: Utc::now(),
            last_updated: Utc::now(),
            is_fake: false,
        }
    }

    #[test]
    fn reprint_same_name_different_set_is_falsely_deduplicated() {
        let mut dedup = Deduplicator::new();

        let mh3_bolt = make_card("scryfall_mh3_bolt", "Lightning Bolt", "MH3");
        dedup.register(&mh3_bolt, None);

        // A reprint in a different set should NOT collide with the MH3 entry,
        // but currently it does because name_index ignores set_code.
        let result = dedup.check_duplicate("Lightning Bolt", None, None);

        // This assertion documents the bug: it returns Some (false positive).
        // The correct behavior would be to return None so both reprints can
        // coexist as distinct cards.
        assert_eq!(
            result,
            Some("scryfall_mh3_bolt".to_string()),
            "BUG: reprint in a different set is collapsed into the first card's entry"
        );
    }

    #[test]
    fn normalize_name_strips_punctuation_and_lowercases() {
        // Apostrophes and spaces vanish — "Gideon's Intervention" == "gideonsintervention"
        assert_eq!(normalize_name("Gideon's Intervention"), "gideonsintervention");
        assert_eq!(normalize_name("Lightning Bolt"), normalize_name("Lightning  Bolt"));
    }

    #[test]
    fn url_normalization_collapses_redd_it_variants() {
        let url1 = "https://i.redd.it/abc123.png?width=640";
        let url2 = "https://preview.redd.it/abc123.png";
        // Both should normalize to the same key, so the second post is correctly
        // identified as a duplicate of the first.
        assert_eq!(normalize_url(url1), normalize_url(url2));
    }
}
```

**Fix direction:** Extend `name_index` to key on `(normalize_name, set_code)` rather than name alone. When `set_code` is unknown (`"UNK"`), fall back to name-only matching to preserve the current behavior for undated leaks.

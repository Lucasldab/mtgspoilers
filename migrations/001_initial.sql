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
);

CREATE INDEX idx_cards_set ON cards(set_code);
CREATE INDEX idx_cards_confidence ON cards(confidence);
CREATE INDEX idx_cards_first_seen ON cards(first_seen);

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
);

CREATE INDEX idx_sources_card ON sources(card_id);

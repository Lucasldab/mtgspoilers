use crossterm::event::KeyCode;
use crate::models::card::{Card, Confidence};
use crate::db::{Database, CardFilter};

pub struct App {
    pub cards: Vec<Card>,
    pub selected_index: usize,
    pub filter: CardFilter,
    pub mode: AppMode,
    pub search_input: String,
    pub db: Database,
    pub should_quit: bool,
    /// Signals the main loop to call refresh on the next tick.
    pub needs_refresh: bool,
    /// Pending async action to execute in the main loop.
    pub pending_action: Option<PendingAction>,
    /// Status message shown in the footer.
    pub status_message: Option<String>,
}

#[derive(Debug)]
pub enum PendingAction {
    MarkFake(String, bool),
}

#[derive(PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Filter,
    Detail,
    MarkingFake,
}

impl App {
    pub async fn new(db: Database) -> anyhow::Result<Self> {
        let filter = CardFilter {
            confidence: None,
            set_code: None,
            hide_fake: true,
            search: None,
        };

        let cards = db.get_cards(&filter).await?;

        Ok(Self {
            cards,
            selected_index: 0,
            filter,
            mode: AppMode::Normal,
            search_input: String::new(),
            db,
            should_quit: false,
            needs_refresh: false,
            pending_action: None,
            status_message: None,
        })
    }

    /// Called by the main loop each tick to flush pending async work.
    pub async fn tick(&mut self) -> anyhow::Result<()> {
        if let Some(action) = self.pending_action.take() {
            match action {
                PendingAction::MarkFake(card_id, is_fake) => {
                    self.db.mark_fake(&card_id, is_fake).await?;
                    self.status_message = Some(if is_fake {
                        "Marked as fake.".to_string()
                    } else {
                        "Unmarked.".to_string()
                    });
                    self.needs_refresh = true;
                }
            }
        }

        if self.needs_refresh {
            self.needs_refresh = false;
            self.refresh().await?;
        }

        Ok(())
    }

    pub async fn refresh(&mut self) -> anyhow::Result<()> {
        self.cards = self.db.get_cards(&self.filter).await?;
        if self.selected_index >= self.cards.len() {
            self.selected_index = self.cards.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn on_key(&mut self, key: KeyCode) {
        self.status_message = None;
        match self.mode {
            AppMode::Normal => self.handle_normal(key),
            AppMode::Search => self.handle_search(key),
            AppMode::Filter => self.handle_filter(key),
            AppMode::Detail => self.handle_detail(key),
            AppMode::MarkingFake => self.handle_marking_fake(key),
        }
    }

    fn handle_normal(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.cards.is_empty() {
                    self.selected_index = (self.selected_index + 1)
                        .min(self.cards.len().saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Char('g') => self.selected_index = 0,
            KeyCode::Char('G') => {
                self.selected_index = self.cards.len().saturating_sub(1);
            }
            KeyCode::Char('/') => {
                self.search_input = self.filter.search.clone().unwrap_or_default();
                self.mode = AppMode::Search;
            }
            KeyCode::Char('f') => self.mode = AppMode::Filter,
            KeyCode::Enter => {
                if !self.cards.is_empty() {
                    self.mode = AppMode::Detail;
                }
            }
            KeyCode::Char('x') => {
                if !self.cards.is_empty() {
                    self.mode = AppMode::MarkingFake;
                }
            }
            KeyCode::Char('r') => {
                self.needs_refresh = true;
                self.status_message = Some("Refreshing...".to_string());
            }
            KeyCode::Char('c') => {
                self.filter.search = None;
                self.needs_refresh = true;
            }
            _ => {}
        }
    }

    fn handle_search(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.search_input.clear();
            }
            KeyCode::Enter => {
                let trimmed = self.search_input.trim().to_string();
                self.filter.search = if trimmed.is_empty() { None } else { Some(trimmed) };
                self.search_input.clear();
                self.mode = AppMode::Normal;
                self.needs_refresh = true;
            }
            KeyCode::Char(c) => self.search_input.push(c),
            KeyCode::Backspace => { self.search_input.pop(); }
            _ => {}
        }
    }

    fn handle_filter(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Char('c') => {
                self.filter.confidence = match self.filter.confidence {
                    None => Some(Confidence::Verified),
                    Some(Confidence::Verified) => Some(Confidence::Unverified),
                    Some(Confidence::Unverified) => None,
                };
                self.needs_refresh = true;
            }
            KeyCode::Char('h') => {
                self.filter.hide_fake = !self.filter.hide_fake;
                self.needs_refresh = true;
            }
            _ => {}
        }
    }

    fn handle_detail(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = AppMode::Normal,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.cards.is_empty() {
                    self.selected_index = (self.selected_index + 1)
                        .min(self.cards.len().saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn handle_marking_fake(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('y') => {
                if let Some(card) = self.cards.get(self.selected_index) {
                    let card_id = card.id.clone();
                    self.pending_action = Some(PendingAction::MarkFake(card_id, true));
                }
                self.mode = AppMode::Normal;
            }
            _ => self.mode = AppMode::Normal,
        }
    }

    pub fn selected_card(&self) -> Option<&Card> {
        self.cards.get(self.selected_index)
    }
}

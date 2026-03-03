use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;

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
}

pub enum AppMode {
    Normal,
    Search,
    Filter,
    Detail,
    MarkingFake,  // Confirmation for marking fake
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
        })
    }

    pub async fn refresh(&mut self) -> anyhow::Result<()> {
        self.cards = self.db.get_cards(&self.filter).await?;
        if self.selected_index >= self.cards.len() {
            self.selected_index = self.cards.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn on_key(&mut self, key: KeyCode) {
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
                self.selected_index = (self.selected_index + 1).min(self.cards.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Char('/') => self.mode = AppMode::Search,
            KeyCode::Char('f') => self.mode = AppMode::Filter,
            KeyCode::Enter => {
                if !self.cards.is_empty() {
                    self.mode = AppMode::Detail;
                }
            }
            KeyCode::Char('x') => self.mode = AppMode::MarkingFake,
            KeyCode::Char('r') => {
                // Trigger refresh (would be async in real impl)
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
                self.filter.search = Some(self.search_input.clone());
                self.mode = AppMode::Normal;
                // Async refresh would happen here
            }
            KeyCode::Char(c) => self.search_input.push(c),
            KeyCode::Backspace => { self.search_input.pop(); }
            _ => {}
        }
    }

    fn handle_marking_fake(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('y') => {
                if let Some(card) = self.cards.get(self.selected_index) {
                    // Async: self.db.mark_fake(&card.id, true);
                    // Remove from current view
                    self.cards.remove(self.selected_index);
                    self.selected_index = self.selected_index.min(self.cards.len().saturating_sub(1));
                }
                self.mode = AppMode::Normal;
            }
            _ => self.mode = AppMode::Normal,
        }
    }

    // ... other handlers
}

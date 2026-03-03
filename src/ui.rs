use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, HighlightSpacing, List, ListItem, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap, Tabs,
    },
    Frame,
};

use crate::app::{App, AppMode};
use crate::models::card::Card;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Footer
        ])
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    // Draw overlays based on mode
    match app.mode {
        AppMode::Search => draw_search_popup(f, app),
        AppMode::Filter => draw_filter_popup(f, app),
        AppMode::MarkingFake => draw_mark_fake_confirm(f, app),
        _ => {}
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" MTG Spoiler Tracker v{} ", env!("CARGO_PKG_VERSION"));

    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        );

    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_card_list(f, app, chunks[0]);
    draw_card_detail(f, app, chunks[1]);
}

fn draw_card_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.cards.iter().enumerate().map(|(i, card)| {
        let confidence = card.confidence.as_str();
        let style = if i == app.selected_index {
            Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(card.confidence.color())
        };

        let line = Line::from(vec![
            Span::styled(format!("{} ", confidence), style),
            Span::raw(&card.name),
            Span::styled(format!(" [{}]", card.set_code), Style::default().fg(Color::Gray)),
        ]);

        ListItem::new(line).style(style)
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Cards ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_spacing(HighlightSpacing::Always);

    f.render_widget(list, area);
}

fn draw_card_detail(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL);

    if let Some(card) = app.cards.get(app.selected_index) {
        let text = format!(
            "Name: {}\n\
             Set: {}\n\
             Mana Cost: {}\n\
             Confidence: {}\n\
             Sources: {}\n\
             First Seen: {}\n\
             \n\
             {}",
            card.name,
            card.set_code,
            card.mana_cost.as_deref().unwrap_or("N/A"),
            card.confidence.as_str(),
            card.sources.len(),
            card.first_seen.format("%Y-%m-%d %H:%M"),
            card.text.as_deref().unwrap_or("No text available")
        );

        let para = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(block);

        f.render_widget(para, area);
    } else {
        f.render_widget(block, area);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.mode {
        AppMode::Normal => "j/k: Navigate | Enter: Details | /: Search | f: Filter | x: Mark Fake | r: Refresh | q: Quit",
        AppMode::Search => "Type to search | Enter: Confirm | Esc: Cancel",
        AppMode::Filter => "c: Confidence | s: Set | h: Hide Fake | Esc: Back",
        AppMode::Detail => "Esc: Back | o: Open URL",
        AppMode::MarkingFake => "y: Confirm Fake | n: Cancel",
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, area);
}

fn draw_search_popup(f: &mut Frame, app: &App) {
    let block = Block::default()
        .title(" Search ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let area = centered_rect(60, 20, f.size());
    f.render_widget(Clear, area);
    f.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let input = Paragraph::new(app.search_input.clone())
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(input, inner);
}

fn draw_mark_fake_confirm(f: &mut Frame, app: &App) {
    if let Some(card) = app.cards.get(app.selected_index) {
        let text = format!("Mark '{}' as FAKE?\n\nThis will hide it from future views.\n\n[y] Yes  [n] No", card.name);
        let para = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Confirm ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
            );

        let area = centered_rect(50, 30, f.size());
        f.render_widget(Clear, area);
        f.render_widget(para, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct UserCard {
    pub id: i32,
    pub username: String,
    pub view_video: bool,
    pub read_camera_configs: bool,
    pub update_signals: bool,
    pub admin_users: bool,
    pub view_ai_events: bool,
    pub manage_ai: bool,
}

pub async fn run_users_menu_shared(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    let mut state = UserAppState::new(db.clone()).await;
    loop {
        terminal.draw(|f| ui(f, &mut state))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !state.status_msg.is_empty() {
                        state.status_msg.clear();
                        continue;
                    }
                    if state.confirm_delete.is_some() {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('s') | KeyCode::Enter => {
                                if let Some(id) = state.confirm_delete {
                                    {
                                        let mut l = db.lock();
                                        if let Err(e) = l.delete_user(id) {
                                            state.status_msg = format!("❌ Error: {}", e);
                                            return Ok(());
                                        }
                                        if let Err(e) = l.flush("TUI user delete") {
                                            state.status_msg = format!("❌ Flush Error: {}", e);
                                            return Ok(());
                                        }
                                    }
                                    state.refresh(db.clone()).await;
                                }
                                state.confirm_delete = None;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => state.confirm_delete = None,
                            _ => {}
                        }
                        continue;
                    }
                    if state.show_add_menu || state.edit_user.is_some() {
                        if key.code == KeyCode::Esc {
                            state.show_add_menu = false;
                            state.edit_user = None;
                        } else {
                            handle_user_input(&key, &mut state, db).await?;
                        }
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => state.next(),
                        KeyCode::Up | KeyCode::Char('k') => state.previous(),
                        KeyCode::Char('a') => {
                            state.clear_inputs();
                            state.show_add_menu = true;
                        }
                        KeyCode::Char('e') => {
                            if let Some(u) = state.get_selected_user().cloned() {
                                state.clear_inputs();
                                state.menu_input = TextInput::new(u.username.clone());
                                state.menu_input3 = TextInput::new(
                                    if u.view_video { "y" } else { "n" }.to_string(),
                                );
                                state.menu_input4 = TextInput::new(
                                    if u.admin_users { "y" } else { "n" }.to_string(),
                                );
                                state.menu_input5 = TextInput::new(
                                    if u.read_camera_configs { "y" } else { "n" }.to_string(),
                                );
                                state.menu_input6 = TextInput::new(
                                    if u.update_signals { "y" } else { "n" }.to_string(),
                                );
                                state.menu_input7 = TextInput::new(
                                    if u.view_ai_events { "y" } else { "n" }.to_string(),
                                );
                                state.menu_input8 = TextInput::new(
                                    if u.manage_ai { "y" } else { "n" }.to_string(),
                                );
                                state.edit_user = Some(u);
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(u) = state.get_selected_user() {
                                state.confirm_delete = Some(u.id);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_user_input(
    key: &event::KeyEvent,
    state: &mut UserAppState,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    match key.code {
        KeyCode::Tab => state.menu_tab = (state.menu_tab + 1) % 8,
        KeyCode::Char(c) => match state.menu_tab {
            0 => state.menu_input.insert_char(c),
            1 => state.menu_input2.insert_char(c),
            2 => state.menu_input3.insert_char(c),
            3 => state.menu_input4.insert_char(c),
            4 => state.menu_input5.insert_char(c),
            5 => state.menu_input6.insert_char(c),
            6 => state.menu_input7.insert_char(c),
            7 => state.menu_input8.insert_char(c),
            _ => {}
        },
        KeyCode::Backspace => match state.menu_tab {
            0 => state.menu_input.backspace(),
            1 => state.menu_input2.backspace(),
            2 => state.menu_input3.backspace(),
            3 => state.menu_input4.backspace(),
            4 => state.menu_input5.backspace(),
            5 => state.menu_input6.backspace(),
            6 => state.menu_input7.backspace(),
            7 => state.menu_input8.backspace(),
            _ => {}
        },
        KeyCode::Enter => {
            let user = state.menu_input.get_content().to_string();
            let pass = state.menu_input2.get_content().to_string();
            
            let parse_yn = |ti: &TextInput| ti.get_content().to_lowercase().starts_with('y');
            
            let can_v = parse_yn(&state.menu_input3);
            let is_a = parse_yn(&state.menu_input4);
            let can_r = parse_yn(&state.menu_input5);
            let can_u = parse_yn(&state.menu_input6);
            let can_ae = parse_yn(&state.menu_input7);
            let can_mai = parse_yn(&state.menu_input8);

            if !user.is_empty() {
                let mut l = db.lock();
                if state.show_add_menu {
                    let mut ch = db::UserChange::add_user(user);
                    if !pass.is_empty() {
                        ch.set_password(pass);
                    }
                    ch.permissions.view_video = can_v;
                    ch.permissions.admin_users = is_a;
                    ch.permissions.read_camera_configs = can_r;
                    ch.permissions.update_signals = can_u;
                    ch.permissions.view_ai_events = can_ae;
                    ch.permissions.manage_ai = can_mai;

                    if let Err(e) = l.apply_user_change(ch) {
                        state.status_msg = format!("❌ Error: {}", e);
                        return Ok(());
                    }
                    if let Err(e) = l.flush("TUI user add") {
                        state.status_msg = format!("❌ Flush Error: {}", e);
                        return Ok(());
                    }
                } else if let Some(u) = &state.edit_user {
                    if let Some(obj) = l.users_by_id().get(&u.id) {
                        let mut ch = obj.change();
                        if !pass.is_empty() {
                            ch.set_password(pass);
                        }
                        ch.permissions.view_video = can_v;
                        ch.permissions.admin_users = is_a;
                        ch.permissions.read_camera_configs = can_r;
                        ch.permissions.update_signals = can_u;
                        ch.permissions.view_ai_events = can_ae;
                        ch.permissions.manage_ai = can_mai;
                        
                        if let Err(e) = l.apply_user_change(ch) {
                            state.status_msg = format!("❌ Error: {}", e);
                            return Ok(());
                        }
                        if let Err(e) = l.flush("TUI user edit") {
                            state.status_msg = format!("❌ Flush Error: {}", e);
                            return Ok(());
                        }
                    }
                }
                drop(l);
                state.refresh(db.clone()).await;
                state.status_msg = "✅ Saved successfully".to_string();
            }
            state.show_add_menu = false;
            state.edit_user = None;
        }
        _ => {}
    }
    Ok(())
}

fn ui(frame: &mut Frame, state: &mut UserAppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" User Management ")
            .border_style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );
    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let style = if Some(i) == state.list_state.selected() {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(format!(
                "  {} [Video: {}, Admin: {}, AI: {}]",
                u.username,
                if u.view_video { "Y" } else { "N" },
                if u.admin_users { "Y" } else { "N" },
                if u.view_ai_events { "Y" } else { "N" }
            ))
            .style(style)
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(" Users ")),
        chunks[1],
        &mut state.list_state,
    );
    if state.show_add_menu || state.edit_user.is_some() {
        render_user_modal(frame, frame.area(), state);
    }
    if state.confirm_delete.is_some() {
        render_delete_modal(frame, frame.area());
    }
    if !state.status_msg.is_empty() {
        let area = centered_rect(50, 20, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(format!("\n{}\n\nPress any key", state.status_msg))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                ),
            area,
        );
    }
}

fn render_user_modal(frame: &mut Frame, area: Rect, state: &mut UserAppState) {
    let area = centered_rect(70, 90, area);
    frame.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);
    let labels = [
        "Username",
        "Password (blank to keep current)",
        "View Video (y/n)",
        "Admin (y/n)",
        "Read Camera Configs (y/n)",
        "Update Signals (y/n)",
        "View AI Events (y/n)",
        "Manage AI Settings (y/n)",
    ];
    let inputs = [
        &state.menu_input,
        &state.menu_input2,
        &state.menu_input3,
        &state.menu_input4,
        &state.menu_input5,
        &state.menu_input6,
        &state.menu_input7,
        &state.menu_input8,
    ];
    frame.render_widget(
        Block::default()
            .title(" User Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
        area,
    );
    for i in 0..8 {
        let b = Block::default()
            .borders(Borders::ALL)
            .title(labels[i])
            .border_style(if state.menu_tab == i {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });
        let display_content = if i == 1 {
            "*".repeat(inputs[i].get_content().chars().count())
        } else {
            inputs[i].get_content().to_string()
        };
        frame.render_widget(Paragraph::new(display_content).block(b), chunks[i]);
        if state.menu_tab == i {
            let cursor_x = chunks[i].x
                + inputs[i].get_content()[..inputs[i].cursor_position]
                    .chars()
                    .count() as u16
                + 1;
            frame.set_cursor_position((cursor_x, chunks[i].y + 1));
        }
    }
}

fn render_delete_modal(frame: &mut Frame, area: Rect) {
    let area = centered_rect(40, 20, area);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("\nDelete user?\n\n[y] Yes | [n] No")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            ),
        area,
    );
}

fn centered_rect(x: u16, y: u16, r: Rect) -> Rect {
    let p_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - y) / 2),
            Constraint::Percentage(y),
            Constraint::Percentage((100 - y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - x) / 2),
            Constraint::Percentage(x),
            Constraint::Percentage((100 - x) / 2),
        ])
        .split(p_v[1])[1]
}

#[derive(Clone)]
struct TextInput {
    content: String,
    cursor_position: usize,
}
impl TextInput {
    fn new(initial: String) -> Self {
        Self {
            cursor_position: initial.len(),
            content: initial,
        }
    }
    fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }
    fn backspace(&mut self) {
        if self.cursor_position > 0 {
            if let Some((idx, _)) = self.content[..self.cursor_position].char_indices().last() {
                self.content.remove(idx);
                self.cursor_position = idx;
            }
        }
    }
    fn clear(&mut self) {
        self.content.clear();
        self.cursor_position = 0;
    }
    fn get_content(&self) -> &str {
        &self.content
    }
}

struct UserAppState {
    items: Vec<UserCard>,
    list_state: ListState,
    show_add_menu: bool,
    edit_user: Option<UserCard>,
    confirm_delete: Option<i32>,
    menu_tab: usize,
    menu_input: TextInput,
    menu_input2: TextInput,
    menu_input3: TextInput,
    menu_input4: TextInput,
    menu_input5: TextInput,
    menu_input6: TextInput,
    menu_input7: TextInput,
    menu_input8: TextInput,
    status_msg: String,
}
impl UserAppState {
    async fn new(db: Arc<db::Database>) -> Self {
        let items = load_users(&db).await;
        Self {
            items,
            list_state: ListState::default().with_selected(Some(0)),
            show_add_menu: false,
            edit_user: None,
            confirm_delete: None,
            menu_tab: 0,
            menu_input: TextInput::new(String::new()),
            menu_input2: TextInput::new(String::new()),
            menu_input3: TextInput::new(String::new()),
            menu_input4: TextInput::new(String::new()),
            menu_input5: TextInput::new(String::new()),
            menu_input6: TextInput::new(String::new()),
            menu_input7: TextInput::new(String::new()),
            menu_input8: TextInput::new(String::new()),
            status_msg: String::new(),
        }
    }
    async fn refresh(&mut self, db: Arc<db::Database>) {
        self.items = load_users(&db).await;
    }
    fn clear_inputs(&mut self) {
        self.menu_input.clear();
        self.menu_input2.clear();
        self.menu_input3.clear();
        self.menu_input4.clear();
        self.menu_input5.clear();
        self.menu_input6.clear();
        self.menu_input7.clear();
        self.menu_input8.clear();
        self.menu_tab = 0;
    }
    fn next(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i < self.items.len().saturating_sub(1) {
                self.list_state.select(Some(i + 1));
            }
        }
    }
    fn previous(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if i > 0 {
                self.list_state.select(Some(i - 1));
            }
        }
    }
    fn get_selected_user(&self) -> Option<&UserCard> {
        self.list_state.selected().and_then(|i| self.items.get(i))
    }
}

async fn load_users(db: &Arc<db::Database>) -> Vec<UserCard> {
    let mut res = Vec::new();
    let l = db.lock();
    for (&id, user) in l.users_by_id() {
        res.push(UserCard {
            id,
            username: user.username.clone(),
            view_video: user.permissions.view_video,
            read_camera_configs: user.permissions.read_camera_configs,
            update_signals: user.permissions.update_signals,
            admin_users: user.permissions.admin_users,
            view_ai_events: user.permissions.view_ai_events,
            manage_ai: user.permissions.manage_ai,
        });
    }
    res
}

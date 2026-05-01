// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct DirCard {
    pub id: i32,
    pub path: String,
    pub stream_count: usize,
    pub exists: bool,
    pub free_space: String,
}

pub async fn run_dirs_menu_shared(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    let mut state = DirAppState::new(db.clone()).await;
    loop {
        terminal.draw(|f| ui(f, &mut state))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if state.confirm_delete.is_some() {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('s') | KeyCode::Enter => {
                                if let Some(id) = state.confirm_delete {
                                    {
                                        let mut l = db.lock();
                                        let cam_ids: Vec<i32> =
                                            l.cameras_by_id().iter().map(|(&id, _)| id).collect();
                                        for cid in cam_ids {
                                            if let Ok(mut cc) = l.null_camera_change(cid) {
                                                let mut changed = false;
                                                for stream in cc.streams.iter_mut() {
                                                    if stream.sample_file_dir_id == Some(id) {
                                                        stream.sample_file_dir_id = None;
                                                        changed = true;
                                                    }
                                                }
                                                if changed {
                                                    let _ = l.update_camera(cid, cc);
                                                }
                                            }
                                        }
                                    }
                                    let _ = db.delete_sample_file_dir(id).await;
                                    state.refresh(db.clone()).await;
                                }
                                state.confirm_delete = None;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => state.confirm_delete = None,
                            _ => {}
                        }
                        continue;
                    }
                    if state.show_add_menu || state.edit_dir.is_some() {
                        if key.code == KeyCode::Esc {
                            state.show_add_menu = false;
                            state.edit_dir = None;
                        } else {
                            handle_dir_input(&key, &mut state, db).await?;
                        }
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => state.next(),
                        KeyCode::Up | KeyCode::Char('k') => state.previous(),
                        KeyCode::Char('a') => {
                            state.menu_input.clear();
                            state.show_add_menu = true;
                        }
                        KeyCode::Char('e') => {
                            if let Some(d) = state.get_selected_dir().cloned() {
                                state.menu_input = TextInput::new(d.path.clone());
                                state.edit_dir = Some(d);
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(d) = state.get_selected_dir() {
                                state.confirm_delete = Some(d.id);
                            }
                        }
                        KeyCode::Char('r') => state.refresh(db.clone()).await,
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_dir_input(
    key: &event::KeyEvent,
    state: &mut DirAppState,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    match key.code {
        KeyCode::Char(c) => state.menu_input.insert_char(c),
        KeyCode::Backspace => state.menu_input.backspace(),
        KeyCode::Enter => {
            let mut path = state.menu_input.get_content().to_string();
            if path.is_empty() {
                let user = std::env::var("USER").unwrap_or_else(|_| "ale".to_string());
                path = format!("/home/{}/Videos/Moonshadow-NVR", user);
            }
            if state.show_add_menu {
                let _ = db.add_sample_file_dir(path.into()).await;
            } else if let Some(d) = &state.edit_dir {
                if db.add_sample_file_dir(path.into()).await.is_ok() {
                    let _ = db.delete_sample_file_dir(d.id).await;
                }
            }
            state.refresh(db.clone()).await;
            state.show_add_menu = false;
            state.edit_dir = None;
        }
        _ => {}
    }
    Ok(())
}

fn ui(frame: &mut Frame, state: &mut DirAppState) {
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
            .title(
                Line::from(" 📂 Directory Management ")
                    .bold()
                    .fg(Color::Cyan),
            )
            .border_style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new("Configure storage paths").style(Style::default().fg(Color::DarkGray)),
        Rect::new(chunks[0].x + 2, chunks[0].y + 1, chunks[0].width - 4, 1),
    );

    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let sc = if d.exists { Color::Green } else { Color::Red };
            let si = if d.exists { "●" } else { "○" };
            let line = Line::from(vec![
                Span::raw(format!("{:2} ", i + 1)),
                Span::styled(si, Style::default().fg(sc)),
                Span::raw(format!(" {:<40} ", d.path)),
                Span::styled(
                    format!(" [S: {}]", d.stream_count),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  {}", d.free_space),
                    Style::default().fg(Color::Cyan),
                ),
            ]);
            let style = if Some(i) == state.list_state.selected() {
                Style::default()
                    .bg(Color::Rgb(49, 48, 60))
                    .fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Storage Pools "),
        ),
        chunks[1],
        &mut state.list_state,
    );

    let help = if state.show_add_menu || state.edit_dir.is_some() {
        "[Enter] Save | [Esc] Cancel"
    } else if state.confirm_delete.is_some() {
        "[y/s] Confirm | [n] Cancel"
    } else {
        "↑↓/jk Navigate | [a] Add | [e] Edit | [d] Delete | [r] Refresh | [Esc] Back"
    };
    frame.render_widget(
        Paragraph::new(help)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
    if state.show_add_menu || state.edit_dir.is_some() {
        render_dir_modal(frame, frame.area(), state);
    }
    if state.confirm_delete.is_some() {
        render_delete_modal(frame, frame.area());
    }
}

fn render_dir_modal(frame: &mut Frame, area: Rect, state: &DirAppState) {
    let area = centered_rect(60, 20, area);
    frame.render_widget(Clear, area);
    let b = Block::default()
        .borders(Borders::ALL)
        .title(if state.show_add_menu {
            " Add Path "
        } else {
            " Edit Path "
        })
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(
        Paragraph::new(state.menu_input.get_content()).block(b),
        area,
    );
    let cursor_x = area.x
        + state.menu_input.get_content()[..state.menu_input.cursor_position]
            .chars()
            .count() as u16
        + 1;
    frame.set_cursor_position((cursor_x, area.y + 1));
}

fn render_delete_modal(frame: &mut Frame, area: Rect) {
    let area = centered_rect(40, 20, area);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("\nDelete this directory?\n\n[y/s] Yes | [n] No")
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

struct DirAppState {
    items: Vec<DirCard>,
    list_state: ListState,
    show_add_menu: bool,
    edit_dir: Option<DirCard>,
    confirm_delete: Option<i32>,
    menu_input: TextInput,
}
impl DirAppState {
    async fn new(db: Arc<db::Database>) -> Self {
        let items = load_dirs(&db).await;
        Self {
            items,
            list_state: ListState::default().with_selected(Some(0)),
            show_add_menu: false,
            edit_dir: None,
            confirm_delete: None,
            menu_input: TextInput::new(String::new()),
        }
    }
    async fn refresh(&mut self, db: Arc<db::Database>) {
        self.items = load_dirs(&db).await;
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
    fn get_selected_dir(&self) -> Option<&DirCard> {
        self.list_state.selected().and_then(|i| self.items.get(i))
    }
}

async fn load_dirs(db: &Arc<db::Database>) -> Vec<DirCard> {
    let mut res = Vec::new();
    let l = db.lock();
    for (&id, dir) in l.sample_file_dirs_by_id() {
        let ps = dir.pool().path().to_string_lossy().to_string();
        let ex = Path::new(&ps).exists();
        let streams = l
            .streams_by_id()
            .values()
            .filter(|s| s.inner.lock().sample_file_dir.as_ref().map(|d| d.id) == Some(id))
            .count();
        res.push(DirCard {
            id,
            path: ps,
            stream_count: streams,
            exists: ex,
            free_space: if ex {
                "Active".to_string()
            } else {
                "Error".to_string()
            },
        });
    }
    res
}

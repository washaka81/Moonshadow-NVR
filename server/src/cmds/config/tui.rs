// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2020-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive TUI camera management with ratatui and Catppuccin theme.

use crate::cmds::open_conn;
use crate::cmds::OpenMode;
use base::clock;
use base::err;
use base::Error;
use bpaf::Bpaf;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use db::json::CameraConfig;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarState},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct CameraCard {
    pub id: i32,
    pub uuid: String,
    pub short_name: String,
    pub description: String,
    pub status: CameraStatus,
    pub stream_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraStatus {
    Online,
    Offline,
    Disabled,
}

impl CameraStatus {
    fn from_enabled_and_recording(enabled: bool, has_recordings: bool) -> Self {
        if !enabled {
            CameraStatus::Disabled
        } else if has_recordings {
            CameraStatus::Online
        } else {
            CameraStatus::Offline
        }
    }
}

#[derive(Bpaf, Debug)]
#[bpaf(command("cameras-tui"))]
#[allow(dead_code)]
pub struct Args {
    #[bpaf(external(crate::parse_db_dir))]
    pub db_dir: PathBuf,
}

#[allow(dead_code)]
pub fn run(args: Args) -> Result<i32, Error> {
    let (_db_dir, mut conn) = open_conn(&args.db_dir, OpenMode::ReadWrite)?;

    let cur_ver = db::get_schema_version(&conn)?;
    if cur_ver.is_none() {
        println!("Initializing database...");
        conn.execute_batch(
            r#"
            pragma journal_mode = delete;
            pragma page_size = 16384;
            vacuum;
            pragma journal_mode = wal;
            "#,
        )?;
        db::init(&mut conn)?;
    }

    let db = Arc::new(db::Database::new(clock::RealClocks {}, conn, true)?);

    run_tui_camera_menu(&db)?;

    Ok(0)
}

pub fn run_tui_camera_menu(db: &Arc<db::Database>) -> Result<(), Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, db);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    let mut state = AppState::new(db.clone());

    loop {
        terminal.draw(|f| ui(f, &mut state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        state.next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.previous();
                    }
                    KeyCode::Char('g') => {
                        state.list_state.select(Some(0));
                    }
                    KeyCode::Char('G') => {
                        state
                            .list_state
                            .select(Some(state.items.len().saturating_sub(1)));
                    }
                    KeyCode::Enter => {
                        if let Some(camera) = state.get_selected_camera().cloned() {
                            state.show_camera_detail = true;
                            state.detail_camera = Some(camera);
                        }
                    }
                    KeyCode::Char('a') => {
                        state.show_add_menu = true;
                    }
                    KeyCode::Char('d') => {
                        if let Some(camera) = state.get_selected_camera() {
                            state.confirm_delete = Some(camera.id);
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(camera) = state.get_selected_camera().cloned() {
                            state.edit_camera = Some(camera);
                        }
                    }
                    KeyCode::Char('r') => {
                        state.refresh(db.clone());
                    }
                    KeyCode::Char('o') => {
                        state.toggle_status_filter();
                    }
                    _ => {}
                }
            }
        }

        if state.show_camera_detail
            || state.show_add_menu
            || state.edit_camera.is_some()
            || state.confirm_delete.is_some()
        {
            while event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if key.code == KeyCode::Esc {
                            state.show_camera_detail = false;
                            state.show_add_menu = false;
                            state.edit_camera = None;
                            state.confirm_delete = None;
                        } else if state.confirm_delete.is_some() {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Enter => {
                                    if let Some(cam_id) = state.confirm_delete {
                                        let mut l = db.lock();
                                        let _ = l.delete_camera(cam_id);
                                        drop(l);
                                        state.refresh(db.clone());
                                    }
                                    state.confirm_delete = None;
                                }
                                KeyCode::Char('n') => {
                                    state.confirm_delete = None;
                                }
                                _ => {}
                            }
                        } else if state.show_add_menu || state.edit_camera.is_some() {
                            handle_menu_input(&key, &mut state, db)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_menu_input(
    key: &event::KeyEvent,
    state: &mut AppState,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    match key.code {
        KeyCode::Tab => {
            state.menu_tab = (state.menu_tab + 1) % 4;
        }
        KeyCode::Left => {
            state.menu_tab = (state.menu_tab + 3) % 4;
        }
        KeyCode::Right => {
            state.menu_tab = (state.menu_tab + 1) % 4;
        }
        KeyCode::Char(c) => match state.menu_tab {
            0 => state.menu_input.push(c),
            1 => state.menu_input2.push(c),
            2 => state.menu_input3.push(c),
            3 => state.menu_input4.push(c),
            _ => {}
        },
        KeyCode::Backspace => match state.menu_tab {
            0 => {
                state.menu_input.pop();
            }
            1 => {
                state.menu_input2.pop();
            }
            2 => {
                state.menu_input3.pop();
            }
            3 => {
                state.menu_input4.pop();
            }
            _ => {}
        },
        KeyCode::Enter => {
            if state.show_add_menu {
                let short_name = state.menu_input.clone();
                if !short_name.is_empty() {
                    let change = db::CameraChange {
                        short_name,
                        config: CameraConfig {
                            description: state.menu_input2.clone(),
                            username: state.menu_input3.clone(),
                            password: state.menu_input4.clone(),
                            ..Default::default()
                        },
                        streams: Default::default(),
                    };
                    let mut l = db.lock();
                    let _ = l.add_camera(change);
                    drop(l);
                    state.refresh(db.clone());
                }
                state.show_add_menu = false;
                state.menu_input.clear();
                state.menu_input2.clear();
                state.menu_input3.clear();
                state.menu_input4.clear();
            } else if let Some(camera) = state.edit_camera.clone() {
                let mut l = db.lock();
                let mut change = l.null_camera_change(camera.id).unwrap();
                change.short_name = if state.menu_input.is_empty() {
                    camera.short_name.clone()
                } else {
                    state.menu_input.clone()
                };
                change.config.description = if state.menu_input2.is_empty() {
                    camera.description.clone()
                } else {
                    state.menu_input2.clone()
                };
                let _ = l.update_camera(camera.id, change);
                drop(l);
                state.refresh(db.clone());
                state.edit_camera = None;
                state.menu_input.clear();
                state.menu_input2.clear();
            }
        }
        _ => {}
    }
    Ok(())
}

struct AppState {
    items: Vec<CameraCard>,
    list_state: ListState,
    show_camera_detail: bool,
    detail_camera: Option<CameraCard>,
    show_add_menu: bool,
    edit_camera: Option<CameraCard>,
    confirm_delete: Option<i32>,
    menu_tab: usize,
    menu_input: String,
    menu_input2: String,
    menu_input3: String,
    menu_input4: String,
    status_filter: Option<CameraStatus>,
}

impl AppState {
    fn new(db: Arc<db::Database>) -> Self {
        let items = load_cameras(&db);
        let mut list_state = ListState::default();
        if !items.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            items,
            list_state,
            show_camera_detail: false,
            detail_camera: None,
            show_add_menu: false,
            edit_camera: None,
            confirm_delete: None,
            menu_tab: 0,
            menu_input: String::new(),
            menu_input2: String::new(),
            menu_input3: String::new(),
            menu_input4: String::new(),
            status_filter: None,
        }
    }

    fn refresh(&mut self, db: Arc<db::Database>) {
        self.items = load_cameras(&db);
        if let Some(idx) = self.list_state.selected() {
            if idx >= self.items.len() {
                self.list_state
                    .select(Some(self.items.len().saturating_sub(1)));
            }
        }
    }

    fn next(&mut self) {
        if let Some(idx) = self.list_state.selected() {
            if idx < self.items.len().saturating_sub(1) {
                self.list_state.select(Some(idx + 1));
            }
        }
    }

    fn previous(&mut self) {
        if let Some(idx) = self.list_state.selected() {
            if idx > 0 {
                self.list_state.select(Some(idx - 1));
            }
        }
    }

    fn get_selected_camera(&self) -> Option<&CameraCard> {
        self.list_state
            .selected()
            .and_then(|idx| self.items.get(idx))
    }

    fn toggle_status_filter(&mut self) {
        self.status_filter = match &self.status_filter {
            None => Some(CameraStatus::Online),
            Some(CameraStatus::Online) => Some(CameraStatus::Offline),
            Some(CameraStatus::Offline) => Some(CameraStatus::Disabled),
            Some(CameraStatus::Disabled) => None,
        };
    }
}

fn load_cameras(db: &Arc<db::Database>) -> Vec<CameraCard> {
    let l = db.lock();
    let cameras = l.cameras_by_id();

    let mut result: Vec<CameraCard> = Vec::new();

    for (id, cam) in cameras {
        let stream_count = cam.streams.iter().filter(|s| s.is_some()).count();
        let enabled = !cam.config.description.is_empty() || stream_count > 0;
        let has_recordings = stream_count > 0;
        let status = CameraStatus::from_enabled_and_recording(enabled, has_recordings);

        result.push(CameraCard {
            id: *id,
            uuid: cam.uuid.to_string(),
            short_name: cam.short_name.clone(),
            description: cam.config.description.clone(),
            status,
            stream_count,
        });
    }

    result.sort_by(|a, b| a.short_name.cmp(&b.short_name));
    result
}

fn ui(frame: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, chunks[0]);
    render_camera_list(frame, chunks[1], state);
    render_footer(frame, chunks[2], state);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Catppuccin::base())
        .title(
            Line::from(" 📷 Moonshadow NVR - Camera Management ")
                .bold()
                .fg(Catppuccin::lavender()),
        );

    frame.render_widget(block, area);
}

fn render_camera_list(frame: &mut Frame, area: Rect, state: &mut AppState) {
    if state.items.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Catppuccin::base())
            .title(Line::from(" No Cameras ").fg(Catppuccin::peach()));

        let text = Paragraph::new("No cameras configured.\n\nPress 'a' to add a new camera.")
            .style(Catppuccin::text())
            .block(block);

        frame.render_widget(text, area);
        return;
    }

    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .map(|(idx, camera)| {
            let status_color = match camera.status {
                CameraStatus::Online => Catppuccin::green(),
                CameraStatus::Offline => Catppuccin::yellow(),
                CameraStatus::Disabled => Catppuccin::red(),
            };
            let status_text = match camera.status {
                CameraStatus::Online => "● Online",
                CameraStatus::Offline => "○ Offline",
                CameraStatus::Disabled => "○ Disabled",
            };

            let line = Line::from(vec![
                Span::raw(format!("{:2} ", idx + 1)),
                Span::raw(format!("{:<20} ", camera.short_name)),
                Span::raw(format!("{:<30} ", camera.description)),
                Span::styled(status_text, status_color),
                Span::raw(format!("  [{} streams]", camera.stream_count)),
            ]);

            if Some(idx) == state.list_state.selected() {
                ListItem::new(line).style(Catppuccin::selection())
            } else {
                ListItem::new(line).style(Catppuccin::text())
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Catppuccin::base())
                .title(Line::from(" Cameras ").fg(Catppuccin::lavender()))
                .border_style(Catppuccin::surface()),
        )
        .highlight_style(Catppuccin::highlight());

    frame.render_stateful_widget(list, area, &mut state.list_state);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Catppuccin::surface());

    let help_text = if state.show_add_menu {
        "[Tab] Next field | [Enter] Save | [Esc] Cancel"
    } else if state.edit_camera.is_some() {
        "[Tab] Next field | [Enter] Save | [Esc] Cancel"
    } else if state.confirm_delete.is_some() {
        "[y] Confirm delete | [n] Cancel"
    } else if state.show_camera_detail {
        "[Esc] Back"
    } else {
        "↑↓ Navigate | [Enter] Details | [a] Add | [e] Edit | [d] Delete | [r] Refresh | [o] Filter | [q] Quit"
    };

    let text = Paragraph::new(help_text)
        .style(Catppuccin::subtext0())
        .block(block);

    frame.render_widget(text, area);
}

struct Catppuccin;

impl Catppuccin {
    fn base() -> Style {
        Style::default().bg(Color::Reset)
    }

    fn text() -> Style {
        Style::default().fg(Color::Reset)
    }

    fn lavender() -> Color {
        Color::Rgb(191, 219, 246)
    }

    fn peach() -> Color {
        Color::Rgb(205, 127, 100)
    }

    fn green() -> Color {
        Color::Rgb(166, 227, 161)
    }

    fn yellow() -> Color {
        Color::Rgb(249, 226, 175)
    }

    fn red() -> Color {
        Color::Rgb(243, 139, 168)
    }

    fn surface() -> Style {
        Style::default().fg(Color::Rgb(137, 180, 250))
    }

    fn selection() -> Style {
        Style::default()
            .bg(Color::Rgb(137, 180, 250))
            .fg(Color::Rgb(30, 30, 46))
    }

    fn highlight() -> Style {
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Rgb(137, 180, 250))
    }

    fn subtext0() -> Style {
        Style::default().fg(Color::Rgb(166, 173, 200))
    }
}

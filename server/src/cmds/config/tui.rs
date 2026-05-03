// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2020-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive TUI configuration system with ratatui.

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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct CameraCard {
    pub id: i32,
    pub short_name: String,
    pub description: String,
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
            // Find the start of the previous character
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

#[derive(Bpaf, Debug)]
#[bpaf(command("config-tui"))]
#[allow(dead_code)]
pub struct Args {
    #[bpaf(external(crate::parse_db_dir))]
    pub db_dir: PathBuf,
}

pub fn run_main_menu(db: &Arc<db::Database>, db_dir: &Path) -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(io::Error::other)?;
    rt.block_on(async {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }
        let result = run_main_menu_app(&mut terminal, db, db_dir).await;
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
    })
}

async fn run_main_menu_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: &Arc<db::Database>,
    db_dir: &Path,
) -> io::Result<()> {
    let mut state = MainMenuState::default();
    loop {
        terminal.draw(|f| render_main_menu_screen(f, &mut state))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !state.status_msg.is_empty() {
                        state.status_msg.clear();
                        continue;
                    }
                    if state.confirm_reset {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('s') => {
                                state.status_msg = perform_factory_reset(db, db_dir).await;
                                state.confirm_reset = false;
                            }
                            _ => {
                                state.confirm_reset = false;
                            }
                        }
                        continue;
                    }
                    if state.show_export || state.show_import {
                        match key.code {
                            KeyCode::Esc => {
                                state.show_export = false;
                                state.show_import = false;
                                state.input.clear();
                            }
                            KeyCode::Char(c) => state.input.insert_char(c),
                            KeyCode::Backspace => state.input.backspace(),
                            KeyCode::Enter => {
                                let filename = state.input.get_content().to_string();
                                if !filename.is_empty() {
                                    if state.show_export {
                                        state.status_msg = perform_export(&filename, db_dir);
                                    } else {
                                        state.status_msg = perform_import(&filename, db_dir);
                                    }
                                }
                                state.show_export = false;
                                state.show_import = false;
                                state.input.clear();
                            }
                            _ => {}
                        }
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected = (state.selected + 1) % 8
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = if state.selected > 0 {
                                state.selected - 1
                            } else {
                                7
                            }
                        }
                        KeyCode::Enter => match state.selected {
                            0 => {
                                let _ = run_cameras_app(terminal, db).await;
                            }
                            1 => {
                                let _ = crate::cmds::config::dirs_tui::run_dirs_menu_shared(
                                    terminal, db,
                                )
                                .await;
                            }
                            2 => {
                                let _ = crate::cmds::config::users_tui::run_users_menu_shared(
                                    terminal, db,
                                )
                                .await;
                            }
                            3 => {
                                let _ = run_hardware_app(terminal, db).await;
                            }
                            4 => {
                                state.input = TextInput::new("backup.sql".to_string());
                                state.show_export = true;
                            }
                            5 => {
                                state.input = TextInput::new(String::new());
                                state.show_import = true;
                            }
                            6 => {
                                state.confirm_reset = true;
                            }
                            7 => break,
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

async fn perform_factory_reset(db: &Arc<db::Database>, db_dir: &Path) -> String {
    let video_dirs: Vec<PathBuf> = {
        let l = db.lock();
        l.sample_file_dirs_by_id()
            .values()
            .map(|d| d.pool().path().to_path_buf())
            .collect()
    };

    // 1. Delete all video files
    for dir in video_dirs {
        let dir: PathBuf = dir;
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::create_dir_all(&dir);
        }
    }

    // 2. Reset database (delete and re-init)
    let db_path = db_dir.join("db");
    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("moonshadow-nvr"));

    let cmd = format!(
        "rm -f {}* && {} init --db-dir {}",
        db_path.display(),
        current_exe.display(),
        db_dir.display()
    );
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
    {
        Ok(s) if s.success() => "✅ System Reset Successful. Restart NVR.".to_string(),
        Ok(s) => format!(
            "❌ Reset failed (exit code: {}). Database might be busy.",
            s
        ),
        Err(e) => format!("❌ Reset failed to start: {}", e),
    }
}

fn perform_export(filename: &str, db_dir: &Path) -> String {
    let db_path = db_dir.join("db");
    let cmd = format!("sqlite3 {} .dump > {}", db_path.display(), filename);
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
    {
        Ok(s) if s.success() => format!("✅ Database exported to {}", filename),
        _ => "❌ Export failed. Is sqlite3 installed?".to_string(),
    }
}

fn perform_import(filename: &str, db_dir: &Path) -> String {
    if !Path::new(filename).exists() {
        return format!("❌ Error: File {} not found", filename);
    }
    let db_path = db_dir.join("db");
    let cmd = format!(
        "rm -f {} && sqlite3 {} < {}",
        db_path.display(),
        db_path.display(),
        filename
    );
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
    {
        Ok(s) if s.success() => "✅ Database imported successfully".to_string(),
        _ => "❌ Critical error during import".to_string(),
    }
}

async fn run_hardware_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    let mut hw_state = HardwareState::new(db);
    loop {
        terminal.draw(|f| render_hardware_screen(f, &mut hw_state))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !hw_state.status_msg.is_empty() {
                        hw_state.status_msg.clear();
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => {
                            hw_state.selected = (hw_state.selected + 1) % 12
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            hw_state.selected = if hw_state.selected > 0 {
                                hw_state.selected - 1
                            } else {
                                11
                            }
                        }
                        KeyCode::Char(' ') | KeyCode::Enter => match hw_state.selected {
                            0 => hw_state.accel = !hw_state.accel,
                            1 => hw_state.vulkan_pre = !hw_state.vulkan_pre,
                            2 => hw_state.ov_repair = !hw_state.ov_repair,
                            3 => {
                                hw_state.ai_mode = match hw_state.ai_mode.as_str() {
                                    "low" => "medium".to_string(),
                                    "medium" => "high".to_string(),
                                    "high" => "auto".to_string(),
                                    _ => "low".to_string(),
                                };
                            }
                            4 => hw_state.enable_detection = !hw_state.enable_detection,
                            5 => hw_state.enable_lpr = !hw_state.enable_lpr,
                            6 => hw_state.enable_face = !hw_state.enable_face,
                            7 => hw_state.enable_heatmap = !hw_state.enable_heatmap,
                            8 => hw_state.prefer_npu = !hw_state.prefer_npu,
                            9 => hw_state.prefer_tpu = !hw_state.prefer_tpu,
                            11 => {
                                let mut l = db.lock();
                                let mut cfg = l.global_config().clone();
                                cfg.hardware_acceleration = hw_state.accel;
                                cfg.vulkan_preprocessing = hw_state.vulkan_pre;
                                cfg.openvino_repair = hw_state.ov_repair;
                                cfg.ai_mode = hw_state.ai_mode.clone();
                                cfg.enable_object_detection = hw_state.enable_detection;
                                cfg.enable_lpr = hw_state.enable_lpr;
                                cfg.enable_face_recognition = hw_state.enable_face;
                                cfg.enable_heatmap = hw_state.enable_heatmap;
                                cfg.prefer_npu = hw_state.prefer_npu;
                                cfg.prefer_tpu = hw_state.prefer_tpu;
                                cfg.model_path = hw_state.model.get_content().to_string();
                                if let Err(e) = l.set_global_config(cfg) {
                                    hw_state.status_msg = format!("❌ Error: {}", e);
                                } else if let Err(e) = l.flush("TUI config save") {
                                    hw_state.status_msg = format!("❌ Flush Error: {}", e);
                                } else {
                                    hw_state.status_msg =
                                        "✅ Global configuration saved".to_string();
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Char(c) if hw_state.selected == 10 => {
                            hw_state.model.insert_char(c);
                        }
                        KeyCode::Backspace if hw_state.selected == 10 => {
                            hw_state.model.backspace();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

struct HardwareState {
    selected: usize,
    accel: bool,
    vulkan_pre: bool,
    ov_repair: bool,
    ai_mode: String,
    model: TextInput,
    // New AI features
    enable_detection: bool,
    enable_lpr: bool,
    enable_face: bool,
    enable_heatmap: bool,
    // Advanced Hardware
    prefer_npu: bool,
    prefer_tpu: bool,
    status_msg: String,
}
impl HardwareState {
    fn new(db: &Arc<db::Database>) -> Self {
        let l = db.lock();
        let cfg = l.global_config();
        Self {
            selected: 0,
            accel: cfg.hardware_acceleration,
            vulkan_pre: cfg.vulkan_preprocessing,
            ov_repair: cfg.openvino_repair,
            ai_mode: if cfg.ai_mode.is_empty() {
                "medium".to_string()
            } else {
                cfg.ai_mode.clone()
            },
            model: TextInput::new(if cfg.model_path.is_empty() {
                "yolov8n.onnx".to_string()
            } else {
                cfg.model_path.clone()
            }),
            enable_detection: cfg.enable_object_detection,
            enable_lpr: cfg.enable_lpr,
            enable_face: cfg.enable_face_recognition,
            enable_heatmap: cfg.enable_heatmap,
            prefer_npu: cfg.prefer_npu,
            prefer_tpu: cfg.prefer_tpu,
            status_msg: String::new(),
        }
    }
}

fn render_hardware_screen(frame: &mut Frame, state: &mut HardwareState) {
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
            .title(" Hardware & AI Configuration ")
            .border_style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );

    let options = [
        format!(
            "  [Accel] OpenVINO Hardware (NPU/GPU/CPU): [{}]",
            if state.accel { "X" } else { " " }
        ),
        format!(
            "  [Preproc] Vulkan iGPU Parallel Pre-proc: [{}]",
            if state.vulkan_pre { "X" } else { " " }
        ),
        format!(
            "  [Fix] Repair OpenVINO Bridge (Auto-fix): [{}]",
            if state.ov_repair { "X" } else { " " }
        ),
        format!("  [Mode] AI Processing Speed: < {} >", state.ai_mode),
        format!(
            "  [Detect] Object Detection (Person/Vehicle): [{}]",
            if state.enable_detection { "X" } else { " " }
        ),
        format!(
            "  [LPR] License Plate Recognition (Chile): [{}]",
            if state.enable_lpr { "X" } else { " " }
        ),
        format!(
            "  [Face] Face Registration & Identities: [{}]",
            if state.enable_face { "X" } else { " " }
        ),
        format!(
            "  [Heatmap] Suspicious Behavior (Dwell): [{}]",
            if state.enable_heatmap { "X" } else { " " }
        ),
        format!(
            "  [NPU] Prefer Neural Processing Unit: [{}]",
            if state.prefer_npu { "X" } else { " " }
        ),
        format!(
            "  [TPU] Prefer Tensor Processing Unit: [{}]",
            if state.prefer_tpu { "X" } else { " " }
        ),
        format!("  [Model] Path: {}", state.model.get_content()),
        "  💾 Save and Back".to_string(),
    ];

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(49, 48, 60))
            } else {
                Style::default()
            };
            ListItem::new(t.as_str()).style(style)
        })
        .collect();

    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(" Settings ")),
        chunks[1],
    );
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
    frame.render_widget(
        Paragraph::new("↑↓ Navigate | Space/Enter Toggle/Select | q/Esc Back")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

struct MainMenuState {
    selected: usize,
    show_export: bool,
    show_import: bool,
    confirm_reset: bool,
    input: TextInput,
    status_msg: String,
}
impl Default for MainMenuState {
    fn default() -> Self {
        Self {
            selected: 0,
            show_export: false,
            show_import: false,
            confirm_reset: false,
            input: TextInput::new(String::new()),
            status_msg: String::new(),
        }
    }
}

fn render_main_menu_screen(frame: &mut Frame, state: &mut MainMenuState) {
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
            .title(" Moonshadow NVR Manager ")
            .border_style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );
    let options = [
        "📷 Manage Cameras",
        "📂 Manage Directories",
        "👥 Manage Users",
        "⚡ Hardware & AI",
        "📤 Export Database",
        "📥 Import Database",
        "🔥 Factory Reset",
        "🚪 Exit",
    ];
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(49, 48, 60))
            } else {
                Style::default()
            };
            ListItem::new(format!("  {}", t)).style(style)
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(" Main Menu ")),
        chunks[1],
        &mut ListState::default().with_selected(Some(state.selected)),
    );
    frame.render_widget(
        Paragraph::new("↑↓ Navigate | Enter Select | q/Esc Exit")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
    if state.show_export || state.show_import {
        let area = centered_rect(60, 20, frame.area());
        frame.render_widget(Clear, area);
        let b = Block::default()
            .borders(Borders::ALL)
            .title(if state.show_export {
                " Export SQL "
            } else {
                " Import SQL "
            })
            .border_style(Style::default().fg(Color::Yellow));
        frame.render_widget(Paragraph::new(state.input.get_content()).block(b), area);
        frame.set_cursor_position((area.x + state.input.cursor_position as u16 + 1, area.y + 1));
    }
    if state.confirm_reset {
        let area = centered_rect(50, 20, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(Paragraph::new("\n⚠️ FACTORY RESET ⚠️\n\nDelete ALL videos and database?\n\n[y/s] CONFIRM | [Any] Cancel").alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red))), area);
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
                        .border_style(Style::default().fg(Color::Green)),
                ),
            area,
        );
    }
}

pub async fn run_cameras_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db: &Arc<db::Database>,
) -> io::Result<()> {
    let mut state = AppState::new(db.clone());
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
                                        let _ = l.delete_camera(id);
                                        if let Err(e) = l.flush("TUI camera delete") {
                                            state.status_msg = format!("❌ Flush Error: {}", e);
                                        }
                                    }
                                    state.refresh(db.clone());
                                }
                                state.confirm_delete = None;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => state.confirm_delete = None,
                            _ => {}
                        }
                        continue;
                    }
                    if state.show_add_menu || state.edit_camera.is_some() {
                        if key.code == KeyCode::Esc {
                            state.show_add_menu = false;
                            state.edit_camera = None;
                        } else {
                            handle_camera_input(&key, &mut state, db)?;
                        }
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => state.next(),
                        KeyCode::Up | KeyCode::Char('k') => state.previous(),
                        KeyCode::Char('a') => {
                            state.clear_menu_inputs();
                            state.show_add_menu = true;
                        }
                        KeyCode::Char('e') => {
                            if let Some(cam) = state.get_selected_camera().cloned() {
                                state.clear_menu_inputs();
                                state.menu_input = TextInput::new(cam.short_name.clone());
                                state.menu_input2 = TextInput::new(cam.description.clone());
                                let l = db.lock();
                                if let Some(c) = l.cameras_by_id().get(&cam.id) {
                                    state.menu_input3 = TextInput::new(c.config.username.clone());
                                    state.menu_input4 = TextInput::new(c.config.password.clone());
                                    if let Some(s_id) = c.streams[0] {
                                        if let Some(s) = l.streams_by_id().get(&s_id) {
                                            let s_lock = s.inner.lock();
                                            state.menu_input5 = TextInput::new(
                                                s_lock
                                                    .config
                                                    .url
                                                    .as_ref()
                                                    .map(|u| u.to_string())
                                                    .unwrap_or_default(),
                                            );
                                            state.menu_input7 = TextInput::new(
                                                (s_lock.config.retain_bytes / 1024 / 1024 / 1024)
                                                    .to_string(),
                                            );
                                            state.menu_input8 = TextInput::new(
                                                if s_lock.config.mode == "record" {
                                                    "y"
                                                } else {
                                                    "n"
                                                }
                                                .to_string(),
                                            );
                                        }
                                    }
                                    if let Some(s_id) = c.streams[1] {
                                        if let Some(s) = l.streams_by_id().get(&s_id) {
                                            state.menu_input6 = TextInput::new(
                                                s.inner
                                                    .lock()
                                                    .config
                                                    .url
                                                    .as_ref()
                                                    .map(|u| u.to_string())
                                                    .unwrap_or_default(),
                                            );
                                        }
                                    }
                                }
                                state.edit_camera = Some(cam);
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(c) = state.get_selected_camera() {
                                state.confirm_delete = Some(c.id);
                            }
                        }
                        KeyCode::Char('r') => state.refresh(db.clone()),
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_camera_input(
    key: &event::KeyEvent,
    state: &mut AppState,
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
            let mut l = db.lock();
            let first_dir_id = l.sample_file_dirs_by_id().keys().next().copied();
            let mut has_storage_warning = false;
            if first_dir_id.is_none() {
                has_storage_warning = true;
            }
            let retain_gb: u64 = state.menu_input7.get_content().parse().unwrap_or(0);
            let retain_bytes = (retain_gb * 1024 * 1024 * 1024) as i64;

            let mode = if state
                .menu_input8
                .get_content()
                .to_lowercase()
                .starts_with('y')
            {
                "record".to_string()
            } else {
                String::new()
            };

            if state.show_add_menu {
                let name = state.menu_input.get_content().to_string();
                if !name.is_empty() {
                    let mut ch = db::CameraChange {
                        short_name: name,
                        config: CameraConfig {
                            description: state.menu_input2.get_content().to_string(),
                            username: state.menu_input3.get_content().to_string(),
                            password: state.menu_input4.get_content().to_string(),
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    if let Ok(u) = url::Url::parse(state.menu_input5.get_content()) {
                        ch.streams[0].config.url = Some(u);
                        ch.streams[0].config.mode = mode.clone();
                        ch.streams[0].config.rtsp_transport = "tcp".to_owned();
                        ch.streams[0].sample_file_dir_id = first_dir_id;
                        ch.streams[0].config.retain_bytes = retain_bytes;
                    }
                    if let Ok(u) = url::Url::parse(state.menu_input6.get_content()) {
                        ch.streams[1].config.url = Some(u);
                        ch.streams[1].config.mode = mode.clone();
                        ch.streams[1].config.rtsp_transport = "tcp".to_owned();
                        ch.streams[1].sample_file_dir_id = first_dir_id;
                        ch.streams[1].config.retain_bytes = retain_bytes;
                    }
                    if let Err(e) = l.add_camera(ch) {
                        state.status_msg = format!("❌ Error adding camera: {}", e);
                        return Ok(());
                    }
                }
            } else if let Some(cam) = state.edit_camera.clone() {
                if let Ok(mut ch) = l.null_camera_change(cam.id) {
                    ch.short_name = state.menu_input.get_content().to_string();
                    ch.config.description = state.menu_input2.get_content().to_string();
                    ch.config.username = state.menu_input3.get_content().to_string();
                    ch.config.password = state.menu_input4.get_content().to_string();
                    let u1 = state.menu_input5.get_content();
                    if !u1.is_empty() {
                        if let Ok(u) = url::Url::parse(u1) {
                            ch.streams[0].config.url = Some(u);
                            ch.streams[0].config.mode = mode.clone();
                            ch.streams[0].config.rtsp_transport = "tcp".to_owned();
                            ch.streams[0].sample_file_dir_id = first_dir_id;
                            ch.streams[0].config.retain_bytes = retain_bytes;
                        }
                    } else {
                        ch.streams[0].config.url = None;
                        ch.streams[0].config.mode = String::new();
                        ch.streams[0].sample_file_dir_id = None;
                    }
                    let u2 = state.menu_input6.get_content();
                    if !u2.is_empty() {
                        if let Ok(u) = url::Url::parse(u2) {
                            ch.streams[1].config.url = Some(u);
                            ch.streams[1].config.mode = mode.clone();
                            ch.streams[1].config.rtsp_transport = "tcp".to_owned();
                            ch.streams[1].sample_file_dir_id = first_dir_id;
                            ch.streams[1].config.retain_bytes = retain_bytes;
                        }
                    } else {
                        ch.streams[1].config.url = None;
                        ch.streams[1].config.mode = String::new();
                        ch.streams[1].sample_file_dir_id = None;
                    }
                    if let Err(e) = l.update_camera(cam.id, ch) {
                        state.status_msg = format!("❌ Error updating camera: {}", e);
                        return Ok(());
                    }
                }
            }
            if let Err(e) = l.flush("TUI camera save") {
                state.status_msg = format!("❌ Flush Error: {}", e);
                return Ok(());
            }
            drop(l);
            state.refresh(db.clone());
            state.show_add_menu = false;
            state.edit_camera = None;
            state.clear_menu_inputs();
            if has_storage_warning {
                state.status_msg =
                    "✅ Saved, but NO STORAGE POOL configured. Recordings won't be saved."
                        .to_string();
            } else {
                state.status_msg = "✅ Saved successfully".to_string();
            }
        }
        _ => {}
    }
    Ok(())
}

struct AppState {
    items: Vec<CameraCard>,
    list_state: ListState,
    show_add_menu: bool,
    edit_camera: Option<CameraCard>,
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
impl AppState {
    fn new(db: Arc<db::Database>) -> Self {
        let items = load_cameras(&db);
        Self {
            items,
            list_state: ListState::default().with_selected(Some(0)),
            show_add_menu: false,
            edit_camera: None,
            confirm_delete: None,
            menu_tab: 0,
            menu_input: TextInput::new(String::new()),
            menu_input2: TextInput::new(String::new()),
            menu_input3: TextInput::new(String::new()),
            menu_input4: TextInput::new(String::new()),
            menu_input5: TextInput::new(String::new()),
            menu_input6: TextInput::new(String::new()),
            menu_input7: TextInput::new("10".to_string()),
            menu_input8: TextInput::new("y".to_string()),
            status_msg: String::new(),
        }
    }
    fn refresh(&mut self, db: Arc<db::Database>) {
        self.items = load_cameras(&db);
    }
    fn clear_menu_inputs(&mut self) {
        self.menu_input.clear();
        self.menu_input2.clear();
        self.menu_input3.clear();
        self.menu_input4.clear();
        self.menu_input5.clear();
        self.menu_input6.clear();
        self.menu_input7 = TextInput::new("10".to_string());
        self.menu_input8 = TextInput::new("y".to_string());
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
    fn get_selected_camera(&self) -> Option<&CameraCard> {
        self.list_state.selected().and_then(|i| self.items.get(i))
    }
}

fn load_cameras(db: &Arc<db::Database>) -> Vec<CameraCard> {
    let mut res = Vec::new();
    let l = db.lock();
    for (&id, cam) in l.cameras_by_id() {
        res.push(CameraCard {
            id,
            short_name: cam.short_name.clone(),
            description: cam.config.description.clone(),
        });
    }
    res.sort_by(|a, b| a.short_name.cmp(&b.short_name));
    res
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
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" Camera Management ")
            .border_style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );
    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if Some(i) == state.list_state.selected() {
                Style::default()
                    .bg(Color::Rgb(49, 48, 60))
                    .fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(format!("  {} - {}", c.short_name, c.description)).style(style)
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(" Cameras ")),
        chunks[1],
        &mut state.list_state,
    );
    if state.show_add_menu || state.edit_camera.is_some() {
        render_camera_modal(frame, frame.area(), state);
    }
    if state.confirm_delete.is_some() {
        render_camera_delete_modal(frame, frame.area());
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
    frame.render_widget(
        Paragraph::new("↑↓/jk Navigate | [a] Add | [e] Edit | [d] Delete | [Esc] Back")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

fn render_camera_modal(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let area = centered_rect(65, 90, area);
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
        "Name",
        "Description",
        "RTSP Username",
        "RTSP Password",
        "Main URL",
        "Sub URL",
        "Retention (Gigabytes)",
        "Record (y/n)",
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
            .title(" Camera Configuration ")
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
        let display_content = if i == 3 {
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

fn render_camera_delete_modal(frame: &mut Frame, area: Rect) {
    let area = centered_rect(40, 20, area);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("\nDelete this camera?\n\n[y/s] Yes | [n] No")
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

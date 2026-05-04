// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2020-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive CLI camera management with pagination for 1000+ cameras.

use crate::cmds::open_conn;
use crate::cmds::OpenMode;
use base::clock;
use base::err;
use base::Error;
use bpaf::Bpaf;
use console::{pad_str, Alignment};
use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind};
use db::json::CameraConfig;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

const ITEMS_PER_PAGE: usize = 20;

const PC_PURPLE: &str = "\x1b[38;2;191;219;216m";
const PC_PINK: &str = "\x1b[38;2;205;127;151m";
const PC_GREEN: &str = "\x1b[38;2;166;227;161m";
const PC_YELLOW: &str = "\x1b[38;2;249;226;175m";
const PC_BLUE: &str = "\x1b[38;2;137;180;250m";
const PC_RESET: &str = "\x1b[0m";

fn pc(s: &str) -> String {
    format!("{}{}", PC_PURPLE, s)
}

fn gr(s: &str) -> String {
    format!("{}{}", PC_GREEN, s)
}

fn pk(s: &str) -> String {
    format!("{}{}", PC_PINK, s)
}

fn yl(s: &str) -> String {
    format!("{}{}", PC_YELLOW, s)
}

fn bl(s: &str) -> String {
    format!("{}{}", PC_BLUE, s)
}

fn edit_stream_interactive(
    theme: &ColorfulTheme,
    stream_name: &str,
    default_config: Option<&db::json::StreamConfig>,
) -> Result<Option<db::json::StreamConfig>, Error> {
    println!();
    println!("{}Configure {} Stream", pc("⚙️ "), stream_name);

    let default_enabled = default_config.map(|c| c.mode == "record").unwrap_or(false);
    let enable_stream = Confirm::with_theme(theme)
        .with_prompt(format!("Enable {} Stream?", stream_name))
        .default(default_enabled)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !enable_stream {
        return Ok(Some(db::json::StreamConfig {
            mode: "off".to_string(),
            ..Default::default()
        }));
    }

    let default_url = default_config
        .and_then(|c| c.url.as_ref())
        .map(|u| u.to_string())
        .unwrap_or_default();
    
    let url_str: String = Input::with_theme(theme)
        .with_prompt(format!("RTSP URL (excluding credentials) [{}]", default_url))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
        
    let final_url_str = if url_str.is_empty() { default_url } else { url_str };
    let url = if final_url_str.is_empty() {
        None
    } else {
        Some(Url::parse(&final_url_str).map_err(|e| err!(InvalidArgument, msg("Invalid URL: {}", e)))?)
    };

    let transport_options = vec!["tcp", "udp"];
    let default_transport_idx = default_config
        .map(|c| if c.rtsp_transport == "udp" { 1 } else { 0 })
        .unwrap_or(0);
        
    let transport_idx = Select::with_theme(theme)
        .with_prompt("Transport Protocol")
        .items(&transport_options)
        .default(default_transport_idx)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    let default_retain_gb = default_config
        .map(|c| c.retain_bytes / (1024 * 1024 * 1024))
        .unwrap_or(10);
        
    let retain_gb_str: String = Input::with_theme(theme)
        .with_prompt(format!("Retention in GB [{}]", default_retain_gb))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let retain_gb = if retain_gb_str.is_empty() {
        default_retain_gb
    } else {
        retain_gb_str.parse::<i64>().map_err(|_| err!(InvalidArgument, msg("Invalid number")))?
    };

    Ok(Some(db::json::StreamConfig {
        mode: "record".to_string(),
        url,
        rtsp_transport: transport_options[transport_idx].to_string(),
        retain_bytes: retain_gb * 1024 * 1024 * 1024,
        ..Default::default()
    }))
}

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
#[bpaf(command("cameras"))]
#[allow(dead_code)]
pub struct Args {
    #[bpaf(external(crate::parse_db_dir))]
    db_dir: PathBuf,
}

#[allow(dead_code)]
pub fn run(args: Args) -> Result<i32, Error> {
    let (_db_dir, mut conn) = open_conn(&args.db_dir, OpenMode::ReadWrite)?;

    let cur_ver = db::get_schema_version(&conn)?;
    if cur_ver.is_none() {
        println!("{}Initializing database...", pc("🗄️  "));
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

    println!("{}Moonshadow NVR - Camera Management", pc("📷 "));
    run_camera_ui(&db)?;

    Ok(0)
}

pub fn run_camera_ui(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();
    let mut search_query = String::new();
    let mut current_page = 0;

    loop {
        print!("\x1B[2J\x1B[1;1H");

        let all_cameras = load_cameras(db);

        let filtered: Vec<CameraCard> = if search_query.is_empty() {
            all_cameras.clone()
        } else {
            let query_lower = search_query.to_lowercase();
            all_cameras
                .into_iter()
                .filter(|c| {
                    c.short_name.to_lowercase().contains(&query_lower)
                        || c.description.to_lowercase().contains(&query_lower)
                        || c.uuid.to_lowercase().contains(&query_lower)
                })
                .collect()
        };

        let total_pages = (filtered.len() as usize).div_ceil(ITEMS_PER_PAGE);
        if total_pages == 0 {
            print_header();
            print_empty_state();
        } else {
            if current_page >= total_pages {
                current_page = total_pages.saturating_sub(1);
            }

            print_header();
            print_pagination(current_page, total_pages, filtered.len());

            let start = current_page * ITEMS_PER_PAGE;
            let end = (start + ITEMS_PER_PAGE).min(filtered.len());
            let page_items = &filtered[start..end];

            print_table(page_items);
        }

        println!();
        if !search_query.is_empty() {
            println!("{}Search: {}{}", bl("🔍 "), yl(&search_query), PC_RESET);
        }
        println!("{}Controls: {}q{}=quit | {}a{}=add | {}r{}=refresh | {}s{}=search | {}c{}=clear | {}n{}=next | {}p{}=prev | {}g{}=goto | [num]=view | {}↑↓{}=navigate",
            bl("► "), pk("q"), PC_RESET,
            pk("a"), PC_RESET,
            pk("r"), PC_RESET,
            pk("s"), PC_RESET,
            pk("c"), PC_RESET,
            pk("n"), PC_RESET,
            pk("p"), PC_RESET,
            pk("g"), PC_RESET,
            pk("↑"), pk("↓")
        );
        println!();

        print!("Enter command (or ↑↓ to navigate): ");
        let input: String = Input::with_theme(&theme)
            .allow_empty(true)
            .interact_text()
            .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

        if poll(Duration::from_millis(50)).map(|ready| ready && matches!(read(), Ok(Event::Key(k)) if k.code == KeyCode::Down && k.kind == KeyEventKind::Press)).unwrap_or(false) {
            let _ = read();
            if current_page < total_pages.saturating_sub(1) {
                current_page += 1;
            }
            continue;
        }
        if poll(Duration::from_millis(50)).map(|ready| ready && matches!(read(), Ok(Event::Key(k)) if k.code == KeyCode::Up && k.kind == KeyEventKind::Press)).unwrap_or(false) {
            let _ = read();
             if current_page > 0 {
                 current_page = current_page.saturating_sub(1);
            }
            continue;
        }

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "q" | "quit" | "Q" => {
                break;
            }
            "a" | "add" | "A" => {
                add_camera_interactive(db)?;
                current_page = 0;
                search_query.clear();
            }
            "r" | "refresh" | "R" => {
                current_page = 0;
                search_query.clear();
            }
            "s" | "search" | "S" => {
                print!("Enter search query: ");
                let query: String = Input::with_theme(&theme)
                    .allow_empty(true)
                    .interact_text()
                    .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
                search_query = query;
                current_page = 0;
            }
            "c" | "clear" | "C" => {
                search_query.clear();
                current_page = 0;
            }
            "n" | "next" | "N" => {
                if current_page < total_pages.saturating_sub(1) {
                    current_page += 1;
                }
            }
            "p" | "prev" | "P" => {
                if current_page > 0 {
                    current_page = current_page.saturating_sub(1);
                }
            }
            "g" | "goto" | "G" => {
                print!("Enter page number (1-{}): ", total_pages.max(1));
                let page_input: String = Input::with_theme(&theme)
                    .allow_empty(true)
                    .interact_text()
                    .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

                if let Ok(page_num) = page_input.trim().parse::<usize>() {
                    if page_num > 0 && page_num <= total_pages {
                        current_page = page_num - 1;
                    }
                }
            }
            _ => {
                if let Ok(num) = input.parse::<usize>() {
                    let idx = num.saturating_sub(1);
                    let start = current_page * ITEMS_PER_PAGE;
                    let actual_idx = start + idx;

                    if actual_idx < filtered.len() {
                        show_camera_detail(db, filtered[actual_idx].id)?;
                    }
                }
            }
        }
    }

    println!("{}Goodbye!", gr("👋 "));
    Ok(())
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

fn print_header() {
    println!(
        "{}╔══════════════════════════════════════════════════════════════════════════════╗",
        PC_PURPLE
    );
    println!(
        "║{} 🗄️  Moonshadow NVR - Camera Manager                                         {} ║",
        PC_BLUE, PC_RESET
    );
    println!(
        "{}╚══════════════════════════════════════════════════════════════════════════════╝",
        PC_PURPLE
    );
    println!();
}

fn print_empty_state() {
    print!("{}", PC_PINK);
    println!("  ┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("  │  📷  No cameras configured                                             │");
    print!("{}", PC_RESET);
    println!(
        "  │  Press {}a{} to add your first camera                                      │",
        pk("a"),
        PC_RESET
    );
    println!(
        "  │  or press {}q{} to return to the main menu                                   │",
        pk("q"),
        PC_RESET
    );
    print!("{}", PC_PINK);
    println!("  └─────────────────────────────────────────────────────────────────────────────┘");
    print!("{}", PC_RESET);
    println!();
}

fn print_table(cameras: &[CameraCard]) {
    print!("{}", PC_BLUE);
    println!("  ┌─────┬──────────────────────┬────────────────────────────────────────────┬──────────┬─────────┐");
    println!(
        "  │ {:^3} │ {:^20} │ {:^40} │ {:^8} │ {:^6} │",
        pc("#"),
        pc("Camera Name"),
        pc("Description"),
        pc("Status"),
        pc("Streams")
    );
    println!("  ├─────┼──────────────────────┼────────────────────────────────────────────┼──────────┼─────────┤");

    for (idx, cam) in cameras.iter().enumerate() {
        let status_str = match cam.status {
            CameraStatus::Online => gr("● Online"),
            CameraStatus::Offline => yl("○ Offline"),
            CameraStatus::Disabled => pc("○ Disabled"),
        };

        let name = pad_str(&cam.short_name, 20, Alignment::Left, None);
        let desc = pad_str(&cam.description, 40, Alignment::Left, None);
        let id_str = format!("{}", idx + 1);

        println!(
            "  │ {:^3} │ {:20} │ {:40} │ {:^8} │ {:^6} │",
            bl(&id_str),
            bl(&name),
            bl(&desc),
            status_str,
            gr(&format!("{}", cam.stream_count))
        );
    }

    println!("  └─────┴──────────────────────┴────────────────────────────────────────────┴──────────┴─────────┘");
    println!("{}", PC_RESET);
    println!();
}

fn print_pagination(current_page: usize, total_pages: usize, total_items: usize) {
    println!(
        "  {}Showing page {}{}/{} of {} cameras{}",
        bl("📄 "),
        yl(format!("{}", current_page + 1).as_str()),
        PC_RESET,
        gr(format!("{}", total_pages).as_str()),
        gr(format!("{}", total_items).as_str()),
        PC_RESET
    );
    println!();
}

#[allow(dead_code)]
pub fn run_classic(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    loop {
        println!();
        println!("{}Camera Management (Classic)", pc("📷 "));

        let cameras = load_cameras(db);

        if cameras.is_empty() {
            println!("{}No cameras configured. Add one first.", yl("⚠️  "));
            let should_add = Confirm::with_theme(&theme)
                .with_prompt("Add a new camera?")
                .default(true)
                .interact()
                .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

            if should_add {
                add_camera_interactive(db)?;
            }
            break;
        }

        let mut options: Vec<String> = cameras
            .iter()
            .map(|c| format!("📷 {} ({})", c.short_name, c.id))
            .collect();

        options.push("➕ Add new camera".to_string());
        options.push("↩️  Back to main menu".to_string());

        let selection = Select::with_theme(&theme)
            .with_prompt("Select a camera")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

        if selection == options.len() - 1 {
            break;
        }

        if selection == options.len() - 2 {
            add_camera_interactive(db)?;
            continue;
        }

        let camera_id = cameras[selection].id;
        show_camera_detail(db, camera_id)?;
    }

    Ok(())
}

fn show_camera_detail(db: &Arc<db::Database>, camera_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    let (short_name, uuid, description, stream_count) = {
        let l = db.lock();
        let camera = match l.cameras_by_id().get(&camera_id) {
            Some(cam) => cam,
            None => {
                println!("{}Camera not found!", yl("⚠️  "));
                return Ok(());
            }
        };
        let sc = camera.streams.iter().filter(|s| s.is_some()).count();
        (
            camera.short_name.clone(),
            camera.uuid.to_string(),
            camera.config.description.clone(),
            sc,
        )
    };

    loop {
        println!();
        println!("{}Camera Details: {}", pc("📷 "), bl(&short_name));
        println!("{}UUID: {}", pc("🆔 "), bl(&uuid));
        println!("{}Description: {}", pc("📝 "), bl(&description));
        println!("{}Streams: {}", pc("📡 "), gr(&format!("{}", stream_count)));

        println!();
        let options = vec!["✏️  Edit camera", "🗑️  Delete camera", "↩️  Back"];

        let selection = Select::with_theme(&theme)
            .with_prompt("Choose action")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

        match selection {
            0 => {
                edit_camera(db, camera_id)?;
            }
            1 => {
                delete_camera(db, camera_id)?;
                break;
            }
            2 => {
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn edit_camera(db: &Arc<db::Database>, camera_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    let mut change = db.lock().null_camera_change(camera_id).unwrap();
    
    let (short_name_default, description_default, onvif_default, username_default, password_default, stream_configs) = {
        let l = db.lock();
        let camera = match l.cameras_by_id().get(&camera_id) {
            Some(cam) => cam,
            None => return Ok(()),
        };
        
        let mut stream_configs = [None, None, None];
        for (i, stream_id_opt) in camera.streams.iter().enumerate() {
            if i >= 3 { break; }
            if let Some(stream_id) = stream_id_opt {
                if let Some(s) = l.streams_by_id().get(stream_id) {
                    stream_configs[i] = Some(s.inner.lock().config.clone());
                }
            }
        }
        
        (
            camera.short_name.clone(), 
            camera.config.description.clone(),
            camera.config.onvif_base_url.as_ref().map(|u| u.to_string()).unwrap_or_default(),
            camera.config.username.clone(),
            camera.config.password.clone(),
            stream_configs,
        )
    };

    println!();
    println!("{}Edit Camera Config", pc("✏️ "));

    let short_name: String = Input::with_theme(&theme)
        .with_prompt(format!("Short name [{}]: ", short_name_default))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
    change.short_name = if short_name.is_empty() { short_name_default } else { short_name };

    let desc_input: String = Input::with_theme(&theme)
        .with_prompt(format!("Description [{}]: ", if description_default.is_empty() { "(empty)" } else { &description_default }))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
    change.config.description = if desc_input.is_empty() { description_default } else { desc_input };

    let onvif_input: String = Input::with_theme(&theme)
        .with_prompt(format!("ONVIF Base URL [{}]: ", if onvif_default.is_empty() { "(empty)" } else { &onvif_default }))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
    let onvif_final = if onvif_input.is_empty() { onvif_default } else { onvif_input };
    change.config.onvif_base_url = if onvif_final.is_empty() { None } else { Some(Url::parse(&onvif_final).map_err(|e| err!(InvalidArgument, msg("Invalid URL: {}", e)))?) };

    let user_input: String = Input::with_theme(&theme)
        .with_prompt(format!("Username [{}]: ", if username_default.is_empty() { "(empty)" } else { &username_default }))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
    change.config.username = if user_input.is_empty() { username_default } else { user_input };

    let pass_input: String = Input::with_theme(&theme)
        .with_prompt(format!("Password [{}]: ", if password_default.is_empty() { "(empty)" } else { "***" }))
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
    change.config.password = if pass_input.is_empty() { password_default } else { pass_input };

    let stream_names = ["Main", "Sub", "Ext"];
    for (i, name) in stream_names.iter().enumerate() {
        if let Some(config) = edit_stream_interactive(&theme, name, stream_configs[i].as_ref())? {
            change.streams[i].config = config;
            // The frontend assigns `sample_file_dir_id` on stream creation. We default to the first available dir.
            let dir_id = {
                let l = db.lock();
                l.sample_file_dirs_by_id().keys().next().copied().unwrap_or(1)
            };
            change.streams[i].sample_file_dir_id = Some(dir_id);
        }
    }

    let mut l = db.lock();
    match l.update_camera(camera_id, change) {
        Ok(_) => {
            println!("{}Camera updated successfully!", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to update camera: {}", yl("❌ "), e);
        }
    }
    drop(l);

    Ok(())
}

fn add_camera_interactive(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Add New Camera", pc("➕ "));

    let short_name: String = Input::with_theme(&theme)
        .with_prompt("Short name (required)")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let description: String = Input::with_theme(&theme)
        .with_prompt("Description")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let onvif_input: String = Input::with_theme(&theme)
        .with_prompt("ONVIF Base URL (optional)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;
    let onvif_base_url = if onvif_input.is_empty() {
        None
    } else {
        Some(Url::parse(&onvif_input).map_err(|e| err!(InvalidArgument, msg("Invalid URL: {}", e)))?)
    };

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username (optional)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let password: String = Input::with_theme(&theme)
        .with_prompt("Password (optional)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let mut streams: [db::StreamChange; 3] = [Default::default(), Default::default(), Default::default()];
    let stream_names = ["Main", "Sub", "Ext"];
    for (i, name) in stream_names.iter().enumerate() {
        if let Some(config) = edit_stream_interactive(&theme, name, None)? {
            streams[i].config = config;
            let dir_id = {
                let l = db.lock();
                l.sample_file_dirs_by_id().keys().next().copied().unwrap_or(1)
            };
            streams[i].sample_file_dir_id = Some(dir_id);
        }
    }

    let confirm = Confirm::with_theme(&theme)
        .with_prompt("Save camera?")
        .default(true)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !confirm {
        println!("{}Cancelled.", yl("⚠️  "));
        return Ok(());
    }

    let change = db::CameraChange {
        short_name,
        config: CameraConfig {
            description,
            onvif_base_url,
            username,
            password,
            ..Default::default()
        },
        streams,
    };

    let mut l = db.lock();
    match l.add_camera(change) {
        Ok(_) => {
            println!("{}Camera added successfully!", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to add camera: {}", yl("❌ "), e);
        }
    }
    drop(l);

    Ok(())
}

fn delete_camera(db: &Arc<db::Database>, camera_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    let confirm = Confirm::with_theme(&theme)
        .with_prompt(format!(
            "Delete camera {}? This cannot be undone!",
            camera_id
        ))
        .default(false)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !confirm {
        println!("{}Cancelled.", yl("⚠️  "));
        return Ok(());
    }

    let mut l = db.lock();
    match l.delete_camera(camera_id) {
        Ok(_) => {
            println!("{}Camera deleted successfully!", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to delete camera: {}", yl("❌ "), e);
        }
    }
    drop(l);

    Ok(())
}

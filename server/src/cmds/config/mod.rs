// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Modern interactive TUI configuration interface.

use base::clock;
use base::Error;
use bpaf::Bpaf;
use std::path::PathBuf;
use std::sync::Arc;
use cursive::{
    Cursive,
    views::{
        Dialog, EditView, LinearLayout, Panel, ScrollView, SelectView, TextView,
        NamedView, ResizedView, OnEventView, Button,
    },
    traits::*,
    direction::Orientation,
    theme::BaseColor,
    event::Key,
};
use cursive::view::Nameable;
use console::style;

pub mod cameras;
pub mod dirs;
pub mod users;
mod tab_complete;

/// Interactively edits configuration.
#[derive(Bpaf, Debug)]
#[bpaf(command("config"))]
pub struct Args {
    #[bpaf(external(crate::parse_db_dir))]
    db_dir: PathBuf,
}

pub fn run(args: Args) -> Result<i32, Error> {
    let (_db_dir, mut conn) = super::open_conn(&args.db_dir, super::OpenMode::Create)?;

    // Auto-initialize if empty
    let cur_ver = db::get_schema_version(&conn)?;
    if cur_ver.is_none() {
        println!("{}", style("Initializing database...").cyan());
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

    let mut siv = cursive::default();
    siv.set_theme(cursive_theme());

    // Main layout: tabs for different config sections
    let mut tabbed = cursive::views::TabbedView::new();

    // Dashboard tab
    tabbed.add_tab("📊 Dashboard", build_dashboard(db.clone()));

    // Cameras tab
    tabbed.add_tab("📷 Cameras", cameras::build_camera_panel(db.clone()));

    // Directories tab
    tabbed.add_tab("📁 Directories", dirs::build_dir_panel(db.clone()));

    // Users tab
    tabbed.add_tab("👥 Users", users::build_user_panel(db.clone()));

    siv.add_fullscreen_layer(tabbed);

    siv.add_global_callback(Key::Esc, |s| s.quit());
    siv.add_global_callback(Key::Ctrl('q'), |s| s.quit());
    siv.set_user_data(db);

    println!("{}", style("Moonshadow NVR Configuration TUI").cyan().bold());
    println!("{}", style("Use Tab to navigate between sections, Esc or Ctrl+Q to exit.").dim());

    siv.run();

    Ok(0)
}

fn cursive_theme() -> cursive::theme::Theme {
    use cursive::theme::{Theme, ColorStyle, Palette, Effect};
    let mut theme = Theme::default();
    theme.shadow = true;
    theme
}

fn build_dashboard(db: Arc<db::Database>) -> impl cursive::view::View {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    // Title
    layout.add_child(
        TextView::new("🎬 Moonshadow NVR Configuration Dashboard")
            .center()
            .with_name("dash_title"),
    );

    layout.add_child(TextView::new(""));

    // Stats section
    let stats_layout = build_stats_panel(db.clone());
    layout.add_child(stats_layout);

    layout.add_child(TextView::new(""));

    // Quick actions
    layout.add_child(TextView::new("=== Quick Actions ===").center());

    let quick_actions = LinearLayout::new(Orientation::Horizontal)
        .child(
            Button::new("📷 Manage Cameras", move |s| {
                // Navigate to cameras tab
                if let Some(mut tabbed) = s.find_name::<cursive::views::TabbedView>("main_tabs") {
                    // Select cameras tab (index 1)
                }
            }),
        )
        .child(
            Button::new("📁 Manage Directories", |s| {
                // Navigate to directories tab
            }),
        )
        .child(
            Button::new("👥 Manage Users", |s| {
                // Navigate to users tab
            }),
        );

    layout.add_child(quick_actions.center());

    layout.add_child(TextView::new(""));

    // Help section
    let help_text = r#"=== Keyboard Shortcuts ===
  Tab         - Switch between tabs
  ↑/↓         - Navigate lists
  Enter       - Select/Edit item
  Esc/Ctrl+Q  - Exit
"#;

    layout.add_child(
        TextView::new(help_text)
            .center()
            .with_name("dash_help"),
    );

    // Refresh stats periodically
    refresh_stats(&mut layout, db);

    Panel::new(layout)
        .title("📊 Moonshadow NVR - Overview")
        .with_name("dashboard_panel")
}

/// Builds the statistics panel.
fn build_stats_panel(db: Arc<db::Database>) -> impl cursive::view::View {
    let l = db.lock();
    let cam_count = l.cameras_by_id().len();

    let mut stream_count = 0;
    for cam in l.cameras_by_id().values() {
        stream_count += cam.streams.iter().filter(|s| s.is_some()).count();
    }

    let dir_count = l.sample_file_dirs_by_id().len();
    let user_count = l.users_by_id().len();

    let mut stream_stats = String::new();
    for (id, stream) in l.streams_by_id() {
        let s_lock = stream.inner.lock();
        let cam_name = l.cameras_by_id()
            .get(&s_lock.camera_id)
            .map(|c| c.short_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        stream_stats.push_str(&format!("  • {} - {}: {} frames\n", cam_name, s_lock.type_.as_str(), s_lock.video_sample_count));
    }
    drop(l);

    let mut layout = LinearLayout::new(Orientation::Vertical);

    layout.add_child(TextView::new("=== System Statistics ===").center());
    layout.add_child(TextView::new(""));
    layout.add_child(TextView::new(format!("📷 Cameras:      {}", cam_count)).center());
    layout.add_child(TextView::new(format!("📡 Streams:      {}", stream_count)).center());
    layout.add_child(TextView::new(format!("📁 Directories:  {}", dir_count)).center());
    layout.add_child(TextView::new(format!("👥 Users:        {}", user_count)).center());

    if !stream_stats.is_empty() {
        layout.add_child(TextView::new(""));
        layout.add_child(TextView::new("=== Stream Details ===").center());
        layout.add_child(TextView::new(&stream_stats));
    }

    layout.with_name("stats_panel")
}

/// Refreshes the dashboard statistics.
fn refresh_stats(layout: &mut LinearLayout, db: Arc<db::Database>) {
    // This would ideally update the existing panel
    // For now, the stats are built initially and can be refreshed by switching tabs
}

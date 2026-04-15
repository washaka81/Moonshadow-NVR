// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Modern interactive directory and retention configuration panel.

use base::strutil::{decode_size, encode_size};
use base::Error;
use console::style;
use std::sync::Arc;

use cursive::{
    direction::Orientation,
    event::Key,
    traits::*,
    view::Nameable,
    views::{Dialog, EditView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};

/// Builds the main directory configuration panel.
pub fn build_dir_panel(db: Arc<db::Database>) -> impl cursive::view::View {
    let panel =
        Panel::new(build_dir_list_view(db.clone())).title("📁 Directory & Retention Configuration");

    panel.with_name("dirs_panel")
}

/// Builds the directory list view.
fn build_dir_list_view(db: Arc<db::Database>) -> impl cursive::view::View {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    // Directory list
    layout.add_child(
        SelectView::<i32>::new()
            .on_submit(move |s, dir_id: &i32| {
                show_dir_detail(s, *dir_id, db.clone());
            })
            .with_name("dir_list")
            .scrollable()
            .full_screen(),
    );

    // Action buttons
    let buttons = LinearLayout::new(Orientation::Horizontal)
        .child(cursive::views::Button::new(
            "✏️ Edit Retention",
            move |s| {
                edit_selected_dir_retention(s, db.clone());
            },
        ))
        .child(cursive::views::Button::new("🔄 Refresh", move |s| {
            refresh_dir_list(s, db.clone());
        }));

    layout.add_child(buttons);

    // Load initial data
    refresh_dir_list_impl(&mut layout, db);

    LinearLayout::new(Orientation::Vertical)
        .child(layout)
        .full_screen()
}

/// Refreshes the directory list from database.
fn refresh_dir_list(s: &mut Cursive, db: Arc<db::Database>) {
    // This function is not implemented correctly yet
    // For now, do nothing
}

fn refresh_dir_list_impl(layout: &mut LinearLayout, db: Arc<db::Database>) {
    let l = db.lock();
    let dirs = l.sample_file_dirs_by_id();

    let mut select = SelectView::<i32>::new().on_submit(move |s, dir_id: &i32| {
        show_dir_detail(s, *dir_id, db.clone());
    });

    for (id, dir) in dirs {
        let path = dir.pool().path().to_string_lossy();
        let label = format!("📁 ID {}: {}", id, path);
        select.add_item(label, *id);
    }

    if dirs.is_empty() {
        select.add_item("(No directories configured)", -1);
    }

    if let Some(mut select_view) = layout.find_name::<SelectView<i32>>("dir_list") {
        *select_view = select;
    }
    drop(l);
}

/// Gets the currently selected directory ID.
fn get_selected_dir_id(s: &mut Cursive) -> Option<i32> {
    s.find_name::<SelectView<i32>>("dir_list")
        .and_then(|select| select.selection().map(|arc| *arc))
        .filter(|id| *id >= 0)
}

/// Shows the directory detail dialog.
fn show_dir_detail(s: &mut Cursive, dir_id: i32, db: Arc<db::Database>) {
    if dir_id < 0 {
        return;
    }

    let l = db.lock();
    let dir = match l.sample_file_dirs_by_id().get(&dir_id) {
        Some(d) => d,
        None => return,
    };

    let path = dir.pool().path().to_string_lossy().into_owned();

    // Get streams using this directory
    let mut streams_info = Vec::new();
    for stream in l.streams_by_id().values() {
        let s_lock = stream.inner.lock();
        if s_lock.sample_file_dir.as_ref().map(|d| d.id) == Some(dir_id) {
            let camera_name = l
                .cameras_by_id()
                .get(&s_lock.camera_id)
                .map(|c| c.short_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            streams_info.push((
                s_lock.id,
                format!("{} - {}", camera_name, s_lock.type_.as_str()),
                s_lock.config.retain_bytes,
            ));
        }
    }
    drop(l);

    let mut layout = LinearLayout::new(Orientation::Vertical);

    layout.add_child(TextView::new(format!("📁 Directory ID: {}", dir_id)));
    layout.add_child(TextView::new(format!("Path: {}", path)));
    layout.add_child(TextView::new(""));
    layout.add_child(TextView::new("=== Associated Streams & Retention ==="));

    for (stream_id, name, limit) in &streams_info {
        let limit_str = encode_size(*limit);
        let entry = format!("🎥 {} (ID: {})\n   Limit: {}", name, stream_id, limit_str);
        layout.add_child(TextView::new(entry));
        layout.add_child(cursive::views::Button::new("Edit", move |s| {
            show_retention_edit_dialog(s.clone(), *stream_id, &db);
        }));
    }

    if streams_info.is_empty() {
        layout.add_child(TextView::new(
            "\n(No streams associated with this directory)",
        ));
    }

    let dialog = Dialog::new()
        .title("Directory Details")
        .content(layout)
        .button("Close", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Edits retention for the selected directory.
fn edit_selected_dir_retention(s: &mut Cursive, db: Arc<db::Database>) {
    if let Some(dir_id) = get_selected_dir_id(s) {
        show_dir_retention_for_dir(s, dir_id, db);
    }
}

/// Shows retention editing for a specific directory.
fn show_dir_retention_for_dir(s: &mut Cursive, dir_id: i32, db: Arc<db::Database>) {
    let l = db.lock();
    let mut streams_info = Vec::new();

    for stream in l.streams_by_id().values() {
        let s_lock = stream.inner.lock();
        if s_lock.sample_file_dir.as_ref().map(|d| d.id) == Some(dir_id) {
            let camera_name = l
                .cameras_by_id()
                .get(&s_lock.camera_id)
                .map(|c| c.short_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            streams_info.push((
                s_lock.id,
                format!("{} - {}", camera_name, s_lock.type_.as_str()),
                s_lock.config.retain_bytes,
            ));
        }
    }
    drop(l);

    if streams_info.is_empty() {
        let dialog = Dialog::new()
            .title("ℹ️ Information")
            .content(TextView::new("No streams associated with this directory."))
            .button("OK", |s| {
                s.pop_layer();
            });
        s.add_layer(dialog);
        return;
    }

    let mut layout = LinearLayout::new(Orientation::Vertical);
    layout.add_child(TextView::new("Select a stream to edit retention:\n"));

    let mut select = SelectView::<(i32, usize)>::new();
    for (i, (stream_id, name, limit)) in streams_info.iter().enumerate() {
        let label = format!("{}: {}", name, encode_size(*limit));
        select.add_item(label, (*stream_id, i));
    }

    let db_clone = db.clone();
    select.set_on_submit(move |s, item: &(i32, usize)| {
        let (stream_id, _) = *item;
        show_retention_edit_dialog(s, stream_id, &db_clone);
    });

    layout.add_child(select);
    layout.add_child(cursive::views::Button::new("Back", |s| {
        s.pop_layer();
    }));

    let dialog = Dialog::new()
        .title("Edit Retention Limits")
        .content(layout)
        .button("Close", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Shows the retention edit dialog for a specific stream.
fn show_retention_edit_dialog(s: &mut Cursive, stream_id: i32, db: &Arc<db::Database>) {
    let l = db.lock();
    let stream = match l.streams_by_id().get(&stream_id) {
        Some(s) => s,
        None => return,
    };

    let s_lock = stream.inner.lock();
    let current_limit = s_lock.config.retain_bytes;
    let camera_name = l
        .cameras_by_id()
        .get(&s_lock.camera_id)
        .map(|c| c.short_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    drop(l);

    let current_limit_str = encode_size(current_limit);

    let mut layout = LinearLayout::new(Orientation::Vertical);
    layout.add_child(TextView::new(format!(
        "Stream: {} - {}",
        camera_name, stream_id
    )));
    layout.add_child(TextView::new(format!(
        "Current Limit: {}",
        current_limit_str
    )));
    layout.add_child(TextView::new(""));
    layout.add_child(TextView::new("Examples: 100GB, 1TB, 500GB"));

    let limit_edit = EditView::new()
        .content(&current_limit_str)
        .with_name("retention_limit");

    layout.add_child(limit_edit);

    let db_clone = db.clone();
    let dialog = Dialog::new()
        .title("Edit Retention Limit")
        .content(layout)
        .button("💾 Save", move |s| {
            if let Some(edit_view) = s.find_name::<EditView>("retention_limit") {
                let new_limit_str = edit_view.get_content().to_string();

                match decode_size(&new_limit_str) {
                    Ok(new_limit) => {
                        let change = db::RetentionChange {
                            stream_id,
                            new_record: true,
                            new_limit,
                        };

                        match db_clone.lock().update_retention(&[change]) {
                            Ok(_) => {
                                println!("{}", style("Retention limit updated.").green());
                                s.pop_layer();
                            }
                            Err(e) => {
                                let error_dialog = Dialog::new()
                                    .title("❌ Error")
                                    .content(TextView::new(format!("Failed to update: {}", e)))
                                    .button("OK", |s| {
                                        s.pop_layer();
                                    });
                                s.add_layer(error_dialog);
                            }
                        }
                    }
                    Err(e) => {
                        let error_dialog = Dialog::new()
                            .title("❌ Validation Error")
                            .content(TextView::new("Invalid size format"))
                            .button("OK", |s| {
                                s.pop_layer();
                            });
                        s.add_layer(error_dialog);
                    }
                }
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Legacy wizard function for backward compatibility.
pub fn run_wizard(db: &Arc<db::Database>) -> Result<(), Error> {
    println!(
        "{}",
        style("Starting interactive directory configuration...").cyan()
    );
    run_interactive(db)
}

fn run_interactive(db: &Arc<db::Database>) -> Result<(), Error> {
    let mut siv = cursive::default();
    siv.set_user_data(db.clone());

    let panel = build_dir_panel(db.clone());
    siv.add_fullscreen_layer(panel);

    siv.run();

    Ok(())
}

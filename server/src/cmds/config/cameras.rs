// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2020-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Modern interactive camera configuration panel using cursive TUI.

use base::Error;
use console::style;
use db::StreamType;
use std::sync::Arc;
use url::Url;

use cursive::{
    Cursive,
    views::{
        Dialog, EditView, LinearLayout, Panel, ResizedView, SelectView, TextView,
        TextArea, Checkbox, RadioButton,
    },
    traits::*,
    direction::Orientation,
    event::Key,
    view::Nameable,
};

/// Builds the main camera configuration panel.
pub fn build_camera_panel(db: Arc<db::Database>) -> impl cursive::view::View {
    let panel = Panel::new(build_camera_list_view(db.clone()))
        .title("📷 Camera Configuration");

    panel.with_name("cameras_panel")
}

/// Builds the camera list view with search functionality.
fn build_camera_list_view(db: Arc<db::Database>) -> impl cursive::view::View {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    // Search bar
    let search_edit = EditView::new()
        .on_submit(move |s, query| {
            filter_camera_list(s, query, db.clone());
        })
        .with_name("camera_search")
        .fixed_width(40);

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(TextView::new("🔍 Search: "))
            .child(search_edit),
    );

    // Camera list
    layout.add_child(
        SelectView::new()
            .with_name("camera_list")
            .on_submit(move |s, camera_id: &i32| {
                show_camera_detail(s, *camera_id, db.clone());
            })
            .scrollable()
            .with_name("camera_select")
            .full_screen(),
    );

    // Action buttons
    let buttons = LinearLayout::new(Orientation::Horizontal)
        .child(
            cursive::views::Button::new("➕ Add", move |s| {
                show_add_camera_dialog(s, db.clone());
            }),
        )
        .child(
            cursive::views::Button::new("✏️ Edit", move |s| {
                edit_selected_camera(s, db.clone());
            }),
        )
        .child(
            cursive::views::Button::new("🗑️ Delete", move |s| {
                delete_selected_camera(s, db.clone());
            }),
        )
        .child(
            cursive::views::Button::new("🔄 Refresh", move |s| {
                refresh_camera_list(s, db.clone());
            }),
        );

    layout.add_child(buttons);

    // Load initial data
    refresh_camera_list_impl(&mut layout, db);

    LinearLayout::new(Orientation::Vertical)
        .child(layout)
        .full_screen()
}

/// Refreshes the camera list from database.
fn refresh_camera_list(s: &mut Cursive, db: Arc<db::Database>) {
    let panel = s.find_name::<Panel>("cameras_panel").unwrap();
    refresh_camera_list_impl(&mut panel, db);
}

fn refresh_camera_list_impl(layout: &mut LinearLayout, db: Arc<db::Database>) {
    let l = db.lock();
    let cameras = l.cameras_by_id();

    let mut select = SelectView::<i32>::new()
        .on_submit(move |s, camera_id: &i32| {
            show_camera_detail(s.clone(), *camera_id, db.clone());
        });

    for (id, cam) in cameras {
        let stream_count = cam.streams.iter().filter(|s| s.is_some()).count();
        let label = format!(
            "📷 {} (ID: {}) - {} streams",
            cam.short_name,
            cam.id,
            stream_count
        );
        select.add_item(label, *id, *id);
    }

    if let Some(mut select_view) = layout.find_name::<SelectView<i32>>("camera_select") {
        *select_view = select;
    }
    drop(l);
}

/// Filters the camera list by search query.
fn filter_camera_list(s: &mut Cursive, query: &str, db: Arc<db::Database>) {
    let l = db.lock();
    let cameras = l.cameras_by_id();
    let query_lower = query.to_lowercase();

    let mut select = SelectView::<i32>::new()
        .on_submit(move |s, camera_id: &i32| {
            show_camera_detail(s.clone(), *camera_id, db.clone());
        });

    for (id, cam) in cameras {
        let matches = query.is_empty()
            || cam.short_name.to_lowercase().contains(&query_lower)
            || cam.description.to_lowercase().contains(&query_lower)
            || cam.uuid.to_string().contains(&query_lower);

        if matches {
            let stream_count = cam.streams.iter().filter(|s| s.is_some()).count();
            let label = format!(
                "📷 {} (ID: {})",
                cam.short_name,
                cam.id,
            );
            select.add_item(label, *id, *id);
        }
    }

    if let Some(mut select_view) = s.find_name::<SelectView<i32>>("camera_select") {
        *select_view = select;
    }
    drop(l);
}

/// Gets the currently selected camera ID.
fn get_selected_camera_id(s: &mut Cursive) -> Option<i32> {
    s.find_name::<SelectView<i32>>("camera_select")
        .and_then(|select| select.selection().cloned())
}

/// Shows the add camera dialog.
fn show_add_camera_dialog(s: &mut Cursive, db: Arc<db::Database>) {
    let mut change = db::CameraChange::default();

    let dialog = Dialog::new()
        .title("➕ Add New Camera")
        .content(build_camera_form(&mut change, &db))
        .button("Save", move |s| {
            if save_camera(s, &change, &db) {
                s.pop_layer();
                refresh_camera_list(s, db.clone());
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Shows the camera detail/edit dialog.
fn show_camera_detail(s: &mut Cursive, camera_id: i32, db: Arc<db::Database>) {
    let l = db.lock();
    let camera = match l.cameras_by_id().get(&camera_id) {
        Some(cam) => cam.clone(),
        None => return,
    };
    drop(l);

    let mut change = l.null_camera_change(camera_id).unwrap();

    let stream_info = format!(
        "Main: {} | Sub: {} | Ext: {}",
        if change.streams[StreamType::Main.index()].config.mode.is_empty() { "❌" } else { "✅" },
        if change.streams[StreamType::Sub.index()].config.mode.is_empty() { "❌" } else { "✅" },
        if change.streams[StreamType::Ext.index()].config.mode.is_empty() { "❌" } else { "✅" },
    );

    let mut layout = LinearLayout::new(Orientation::Vertical);

    layout.add_child(TextView::new(format!("📷 Camera: {}", camera.short_name)));
    layout.add_child(TextView::new(format!("🆔 UUID: {}", camera.uuid)));
    layout.add_child(TextView::new(format!("📝 Description: {}", camera.config.description)));
    layout.add_child(TextView::new(format!("📡 Streams: {}", stream_info)));
    layout.add_child(TextView::new("\n--- Edit Mode ---"));

    let dialog = Dialog::new()
        .title("Camera Details")
        .content(build_camera_form(&mut change, &db))
        .button("💾 Save", move |s| {
            if save_camera(s, &change, &db) {
                s.pop_layer();
                refresh_camera_list(s, db.clone());
            }
        })
        .button("🗑️ Delete", move |s| {
            if confirm_delete_camera(s, camera_id, &db) {
                s.pop_layer();
                refresh_camera_list(s, db.clone());
            }
        })
        .button("↩️ Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Edits the currently selected camera.
fn edit_selected_camera(s: &mut Cursive, db: Arc<db::Database>) {
    if let Some(camera_id) = get_selected_camera_id(s) {
        show_camera_detail(s, camera_id, db);
    }
}

/// Deletes the currently selected camera.
fn delete_selected_camera(s: &mut Cursive, db: Arc<db::Database>) {
    if let Some(camera_id) = get_selected_camera_id(s) {
        confirm_delete_camera(s, camera_id, &db);
    }
}

/// Shows confirmation dialog before deleting a camera.
fn confirm_delete_camera(s: &mut Cursive, camera_id: i32, db: &Arc<db::Database>) -> bool {
    let dialog = Dialog::new()
        .title("⚠️ Confirm Delete")
        .content(TextView::new("Are you sure you want to delete this camera?\nThis action cannot be undone!"))
        .button("🗑️ Delete", move |s| {
            let db_clone = db.clone();
            match db_clone.lock().delete_camera(camera_id) {
                Ok(_) => {
                    s.pop_layer();
                    refresh_camera_list(s, db_clone.clone());
                    println!("{}", style("Camera deleted successfully.").green());
                }
                Err(e) => {
                    println!("{}", style(format!("Delete failed: {}", e)).red());
                }
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
    false
}

/// Builds the camera configuration form.
fn build_camera_form(change: &mut db::CameraChange, db: &Arc<db::Database>) -> impl cursive::view::View {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    // General section
    layout.add_child(TextView::new("=== General Information ==="));

    let short_name_edit = EditView::new()
        .content(&change.short_name)
        .on_submit(move |_, text| {
            change.short_name = text.to_string();
        })
        .with_name("short_name");

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(TextView::new("Short Name: ").fixed_width(15))
            .child(short_name_edit.full_width()),
    );

    let desc_edit = EditView::new()
        .content(&change.config.description)
        .on_submit(move |_, text| {
            change.config.description = text.to_string();
        })
        .with_name("description");

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(TextView::new("Description: ").fixed_width(15))
            .child(desc_edit.full_width()),
    );

    let username_edit = EditView::new()
        .content(&change.config.username)
        .on_submit(move |_, text| {
            change.config.username = text.to_string();
        })
        .with_name("username");

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(TextView::new("Username: ").fixed_width(15))
            .child(username_edit.full_width()),
    );

    let password_edit = EditView::new()
        .content(&change.config.password)
        .secret()
        .on_submit(move |_, text| {
            change.config.password = text.to_string();
        })
        .with_name("password");

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(TextView::new("Password: ").fixed_width(15))
            .child(password_edit.full_width()),
    );

    // Streams section
    layout.add_child(TextView::new("\n=== Stream Configuration ==="));

    for st in &[StreamType::Main, StreamType::Sub, StreamType::Ext] {
        let idx = st.index();
        let stream = &mut change.streams[idx];

        let enabled = !stream.config.mode.is_empty();

        layout.add_child(TextView::new(format!("\n--- {} Stream ---", st.as_str())));

        let checkbox = Checkbox::new()
            .checked(enabled)
            .on_change(move |_, checked| {
                if checked {
                    change.streams[idx].config.mode = "record".to_string();
                } else {
                    change.streams[idx].config.mode = "".to_string();
                }
            });

        layout.add_child(
            LinearLayout::new(Orientation::Horizontal)
                .child(TextView::new("Enabled: ").fixed_width(15))
                .child(checkbox),
        );

        let url_str = stream.config.url.as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();

        let url_edit = EditView::new()
            .content(&url_str)
            .on_submit(move |_, text| {
                if !text.is_empty() {
                    if let Ok(url) = Url::parse(text) {
                        change.streams[idx].config.url = Some(url);
                    }
                } else {
                    change.streams[idx].config.url = None;
                }
            })
            .with_name(format!("url_{}", st.as_str()));

        layout.add_child(
            LinearLayout::new(Orientation::Horizontal)
                .child(TextView::new("RTSP URL: ").fixed_width(15))
                .child(url_edit.full_width()),
        );

        // Transport selection
        let transport = if stream.config.rtsp_transport == "tcp" { 1 } else { 0 };

        let radio_group = cursive::views::RadioGroup::new()
            .child("UDP", 0)
            .child("TCP", 1)
            .selected(transport)
            .on_change(move |_, val| {
                change.streams[idx].config.rtsp_transport = if val == 1 { "tcp".to_string() } else { "udp".to_string() };
            });

        layout.add_child(
            LinearLayout::new(Orientation::Horizontal)
                .child(TextView::new("Transport: ").fixed_width(15))
                .child(radio_group),
        );
    }

    layout
}

/// Saves the camera configuration.
fn save_camera(s: &mut Cursive, change: &db::CameraChange, db: &Arc<db::Database>) -> bool {
    if change.short_name.is_empty() {
        let dialog = Dialog::new()
            .title("⚠️ Validation Error")
            .content(TextView::new("Short Name cannot be empty!"))
            .button("OK", |s| {
                s.pop_layer();
            });
        s.add_layer(dialog);
        return false;
    }

    // Check if this is a new camera or update
    let is_new = change.id.is_none();

    let mut l = db.lock();
    let result = if is_new {
        l.add_camera(change.clone())
    } else {
        let camera_id = change.id.unwrap();
        l.update_camera(camera_id, change.clone())
    };
    drop(l);

    match result {
        Ok(_) => {
            println!("{}", style(if is_new { "Camera added successfully!" } else { "Camera updated successfully!" }).green().bold());
            true
        }
        Err(e) => {
            let dialog = Dialog::new()
                .title("❌ Error")
                .content(TextView::new(format!("Failed to save camera: {}", e)))
                .button("OK", |s| {
                    s.pop_layer();
                });
            s.add_layer(dialog);
            false
        }
    }
}

/// Legacy wizard function for backward compatibility.
pub fn run_wizard(db: &Arc<db::Database>) -> Result<(), Error> {
    println!("{}", style("Starting interactive TUI configuration...").cyan());
    run_interactive(db)
}

fn run_interactive(db: &Arc<db::Database>) -> Result<(), Error> {
    let mut siv = cursive::default();
    siv.set_user_data(db.clone());

    let panel = build_camera_panel(db.clone());
    siv.add_fullscreen_layer(panel);

    siv.run();

    Ok(())
}

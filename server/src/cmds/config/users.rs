// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Modern interactive user configuration panel.

use base::Error;
use console::style;
use std::sync::Arc;

use cursive::{
    direction::Orientation,
    event::Key,
    traits::*,
    view::Nameable,
    views::{Checkbox, Dialog, EditView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};

/// Builds the main user configuration panel.
pub fn build_user_panel(db: Arc<db::Database>) -> impl cursive::view::View {
    let panel = Panel::new(build_user_list_view(db.clone())).title("👥 User Configuration");

    panel.with_name("users_panel")
}

/// Builds the user list view.
fn build_user_list_view(db: Arc<db::Database>) -> impl cursive::view::View {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    // User list
    layout.add_child(
        SelectView::<i32>::new()
            .on_submit(move |s, user_id: &i32| {
                show_user_detail(s, *user_id, db.clone());
            })
            .with_name("user_list")
            .scrollable()
            .full_screen(),
    );

    // Action buttons
    let buttons = LinearLayout::new(Orientation::Horizontal)
        .child(cursive::views::Button::new("➕ Add User", move |s| {
            show_add_user_dialog(s, db.clone());
        }))
        .child(cursive::views::Button::new("✏️ Edit", move |s| {
            edit_selected_user(s, db.clone());
        }))
        .child(cursive::views::Button::new("🗑️ Delete", move |s| {
            delete_selected_user(s, db.clone());
        }))
        .child(cursive::views::Button::new("🔄 Refresh", move |s| {
            refresh_user_list(s, db.clone());
        }));

    layout.add_child(buttons);

    // Load initial data
    refresh_user_list_impl(&mut layout, db);

    LinearLayout::new(Orientation::Vertical)
        .child(layout)
        .full_screen()
}

/// Refreshes the user list from database.
fn refresh_user_list(s: &mut Cursive, db: Arc<db::Database>) {
    // This function is not implemented correctly yet
    // For now, do nothing
}

fn refresh_user_list_impl(layout: &mut LinearLayout, db: Arc<db::Database>) {
    let l = db.lock();
    let users = l.users_by_id();

    let mut select = SelectView::<i32>::new().on_submit(move |s, user_id: &i32| {
        show_user_detail(s, *user_id, db.clone());
    });

    for (id, user) in users {
        let perms = format_permissions_summary(&user.permissions);
        let label = format!("👤 {} (ID: {}) - {}", user.username, user.id, perms);
        select.add_item(label, *id);
    }

    if users.is_empty() {
        select.add_item("(No users configured)", -1);
    }

    if let Some(mut select_view) = layout.find_name::<SelectView<i32>>("user_list") {
        *select_view = select;
    }
    drop(l);
}

/// Formats a summary of permissions.
fn format_permissions_summary(perms: &db::Permissions) -> String {
    let mut parts = Vec::new();
    if perms.admin_users {
        parts.push("Admin");
    }
    if perms.view_video {
        parts.push("View");
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join(", ")
    }
}

/// Gets the currently selected user ID.
fn get_selected_user_id(s: &mut Cursive) -> Option<i32> {
    s.find_name::<SelectView<i32>>("user_list")
        .and_then(|select| select.selection().map(|arc| *arc))
        .filter(|id| *id >= 0)
}

/// Shows the add user dialog.
fn show_add_user_dialog(s: &mut Cursive, db: Arc<db::Database>) {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    layout.add_child(TextView::new("=== Add New User ===\n"));

    let username_edit = EditView::new()
        .with_prompt("Username: ")
        .with_name("username");

    layout.add_child(username_edit);

    let password_edit = EditView::new()
        .secret()
        .with_prompt("Password: ")
        .with_name("password");

    layout.add_child(password_edit);

    let confirm_edit = EditView::new()
        .secret()
        .with_prompt("Confirm: ")
        .with_name("confirm_password");

    layout.add_child(confirm_edit);

    layout.add_child(TextView::new("\n=== Default Permissions ==="));

    let view_video = Checkbox::new().with_name("perm_view_video");

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(view_video)
            .child(TextView::new(" View Video")),
    );

    let admin_users = Checkbox::new().with_name("perm_admin");

    layout.add_child(
        LinearLayout::new(Orientation::Horizontal)
            .child(admin_users)
            .child(TextView::new(" Admin (Manage Users)")),
    );

    let db_clone = db.clone();
    let dialog = Dialog::new()
        .title("➕ Add New User")
        .content(layout)
        .button("💾 Create", move |s| {
            if create_user(s, &db_clone) {
                s.pop_layer();
                refresh_user_list(s, db_clone.clone());
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Creates a new user from the dialog form.
fn create_user(s: &mut Cursive, db: &Arc<db::Database>) -> bool {
    let username = if let Some(edit) = s.find_name::<EditView>("username") {
        edit.get_content().to_string()
    } else {
        return false;
    };

    let password = if let Some(edit) = s.find_name::<EditView>("password") {
        edit.get_content().to_string()
    } else {
        return false;
    };

    let confirm = if let Some(edit) = s.find_name::<EditView>("confirm_password") {
        edit.get_content().to_string()
    } else {
        return false;
    };

    if username.is_empty() {
        show_error_dialog(s, "Username cannot be empty.");
        return false;
    }

    if password != confirm {
        show_error_dialog(s, "Passwords do not match.");
        return false;
    }

    if password.is_empty() {
        show_error_dialog(s, "Password cannot be empty.");
        return false;
    }

    let view_video = s
        .find_name::<Checkbox>("perm_view_video")
        .map(|c| c.is_checked())
        .unwrap_or(true);

    let admin_users = s
        .find_name::<Checkbox>("perm_admin")
        .map(|c| c.is_checked())
        .unwrap_or(false);

    let mut change = db::UserChange::add_user(username);
    change.set_password(password.into());
    change.permissions.view_video = view_video;
    change.permissions.admin_users = admin_users;

    match db.lock().apply_user_change(change) {
        Ok(_) => {
            println!("{}", style("User added successfully!").green().bold());
            true
        }
        Err(e) => {
            show_error_dialog(s, &format!("Failed to add user: {}", e));
            false
        }
    }
}

/// Shows the user detail/edit dialog.
fn show_user_detail(s: &mut Cursive, user_id: i32, db: Arc<db::Database>) {
    if user_id < 0 {
        return;
    }

    let l = db.lock();
    let user = match l.users_by_id().get(&user_id) {
        Some(u) => u.clone(),
        None => return,
    };
    drop(l);

    let mut layout = LinearLayout::new(Orientation::Vertical);

    layout.add_child(TextView::new(format!("👤 User: {}", user.username)));
    layout.add_child(TextView::new(format!("🆔 ID: {}", user.id)));
    layout.add_child(TextView::new(""));
    layout.add_child(TextView::new("=== Permissions ==="));
    layout.add_child(TextView::new(format!(
        "  View Video: {}",
        if user.permissions.view_video {
            "✅"
        } else {
            "❌"
        }
    )));
    layout.add_child(TextView::new(format!(
        "  Admin Users: {}",
        if user.permissions.admin_users {
            "✅"
        } else {
            "❌"
        }
    )));
    layout.add_child(TextView::new(format!(
        "  Read Camera Configs: {}",
        if user.permissions.read_camera_configs {
            "✅"
        } else {
            "❌"
        }
    )));
    layout.add_child(TextView::new(format!(
        "  Update Signals: {}",
        if user.permissions.update_signals {
            "✅"
        } else {
            "❌"
        }
    )));
    layout.add_child(TextView::new(""));

    // Password change section
    layout.add_child(TextView::new("=== Change Password ==="));

    let new_password_edit = EditView::new()
        .secret()
        .with_prompt("New Password: ")
        .with_name("new_password");

    layout.add_child(new_password_edit);

    let confirm_edit = EditView::new()
        .secret()
        .with_prompt("Confirm: ")
        .with_name("confirm_new_password");

    layout.add_child(confirm_edit);

    let db_clone = db.clone();
    let dialog = Dialog::new()
        .title("User Details")
        .content(layout)
        .button("💾 Change Password", move |s| {
            if change_user_password(s, user_id, &db_clone) {
                s.pop_layer();
                refresh_user_list(s, db_clone.clone());
            }
        })
        .button("🗑️ Delete", move |s| {
            if confirm_delete_user(s, user_id, &db) {
                s.pop_layer();
                refresh_user_list(s, db.clone());
            }
        })
        .button("↩️ Close", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Changes a user password from the dialog form.
fn change_user_password(s: &mut Cursive, user_id: i32, db: &Arc<db::Database>) -> bool {
    let new_password = if let Some(edit) = s.find_name::<EditView>("new_password") {
        edit.get_content().to_string()
    } else {
        return false;
    };

    let confirm = if let Some(edit) = s.find_name::<EditView>("confirm_new_password") {
        edit.get_content().to_string()
    } else {
        return false;
    };

    if new_password.is_empty() {
        show_error_dialog(s, "Password cannot be empty.");
        return false;
    }

    if new_password != confirm {
        show_error_dialog(s, "Passwords do not match.");
        return false;
    }

    let mut l = db.lock();
    let user = match l.users_by_id().get(&user_id) {
        Some(u) => u.clone(),
        None => {
            show_error_dialog(s, "User not found.");
            return false;
        }
    };

    let mut change = user.change();
    change.set_password(new_password.into());

    match l.apply_user_change(change) {
        Ok(_) => {
            println!("{}", style("Password updated successfully.").green().bold());
            true
        }
        Err(e) => {
            show_error_dialog(s, &format!("Failed to update password: {}", e));
            false
        }
    }
}

/// Edits the currently selected user.
fn edit_selected_user(s: &mut Cursive, db: Arc<db::Database>) {
    if let Some(user_id) = get_selected_user_id(s) {
        show_user_detail(s, user_id, db);
    }
}

/// Deletes the currently selected user.
fn delete_selected_user(s: &mut Cursive, db: Arc<db::Database>) {
    if let Some(user_id) = get_selected_user_id(s) {
        confirm_delete_user(s, user_id, &db);
    }
}

/// Shows confirmation dialog before deleting a user.
fn confirm_delete_user(s: &mut Cursive, user_id: i32, db: &Arc<db::Database>) -> bool {
    let db_clone = db.clone();
    let dialog = Dialog::new()
        .title("⚠️ Confirm Delete")
        .content(TextView::new(
            "Are you sure you want to delete this user?\nThis action cannot be undone!",
        ))
        .button("🗑️ Delete", move |s| {
            match db_clone.lock().delete_user(user_id) {
                Ok(_) => {
                    s.pop_layer();
                    println!("{}", style("User deleted successfully.").green());
                }
                Err(e) => {
                    show_error_dialog(s, &format!("Delete failed: {}", e));
                }
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
    false
}

/// Shows an error dialog with the given message.
fn show_error_dialog(s: &mut Cursive, message: &str) {
    let dialog = Dialog::new()
        .title("❌ Error")
        .content(TextView::new(message))
        .button("OK", |s| {
            s.pop_layer();
        });
    s.add_layer(dialog);
}

/// Legacy wizard function for backward compatibility.
pub fn run_wizard(db: &Arc<db::Database>) -> Result<(), Error> {
    println!(
        "{}",
        style("Starting interactive user configuration...").cyan()
    );
    run_interactive(db)
}

fn run_interactive(db: &Arc<db::Database>) -> Result<(), Error> {
    let mut siv = cursive::default();
    siv.set_user_data(db.clone());

    let panel = build_user_panel(db.clone());
    siv.add_fullscreen_layer(panel);

    siv.run();

    Ok(())
}

// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive CLI user configuration with dialoguer.

use base::err;
use base::Error;
use bpaf::Bpaf;
use console::{pad_str, Alignment};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::path::PathBuf;
use std::sync::Arc;

use crate::cmds::open_conn;
use crate::cmds::OpenMode;
use base::clock;

const PC_PURPLE: &str = "\x1b[38;2;191;219;216m";
const PC_GREEN: &str = "\x1b[38:2:166:227:161m";
const PC_BLUE: &str = "\x1b[38:2:137:180:250m";
const PC_PINK: &str = "\x1b[38:2:205:127:151m";
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

fn bl(s: &str) -> String {
    format!("{}{}", PC_BLUE, s)
}

#[derive(Clone, Debug)]
pub struct UserCard {
    pub id: i32,
    pub username: String,
    pub permissions: db::Permissions,
}

#[derive(Bpaf, Debug)]
#[bpaf(command("users"))]
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
        println!("{}Initializing database...", pc("🗄️ "));
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

    println!("{}Moonshadow NVR - User Management", pc("👥 "));
    run_user_ui(&db)?;

    Ok(0)
}

pub fn run_user_ui(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    loop {
        print!("\x1B[2J\x1B[1;1H");

        let users = load_users(db);

        print_header();

        if users.is_empty() {
            print_empty_state();
        } else {
            print_table(&users);
        }

        println!();
        println!("{}Controls: {}q{}=quit | {}a{}=add | {}r{}=refresh | {}e{}=edit | {}d{}=delete | [num]=view",
            bl("► "), pk("q"), PC_RESET,
            pk("a"), PC_RESET,
            pk("r"), PC_RESET,
            pk("e"), PC_RESET,
            pk("d"), PC_RESET
        );
        println!();

        let input: String = Input::with_theme(&theme)
            .allow_empty(true)
            .with_prompt("Enter command")
            .interact_text()
            .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

        let input = input.trim().to_lowercase();

        match input.as_str() {
            "q" | "quit" => break,
            "a" | "add" => {
                add_user_interactive(db)?;
            }
            "r" | "refresh" => {}
            "e" | "edit" => {
                if !users.is_empty() {
                    edit_user_interactive(db, &users)?;
                }
            }
            "d" | "delete" => {
                if !users.is_empty() {
                    delete_user_interactive(db, &users)?;
                }
            }
            _ => {
                if let Ok(num) = input.parse::<usize>() {
                    let idx = num.saturating_sub(1);
                    if idx < users.len() {
                        show_user_detail(db, users[idx].id)?;
                    }
                }
            }
        }
    }

    println!("{}Goodbye!", gr("👋 "));
    Ok(())
}

fn load_users(db: &Arc<db::Database>) -> Vec<UserCard> {
    let l = db.lock();
    let users = l.users_by_id();
    let mut result: Vec<UserCard> = Vec::new();

    for (id, user) in users {
        result.push(UserCard {
            id: *id,
            username: user.username.clone(),
            permissions: user.permissions.clone(),
        });
    }

    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

fn print_header() {
    println!(
        "{}╔══════════════════════════════════════════════════════════════════════════════╗",
        PC_PURPLE
    );
    println!(
        "║{} 👥 Moonshadow NVR - User Manager {} ║",
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
    println!(" ┌─────────────────────────────────────────────────────────────────────────────┐");
    println!(" │ 👥 No users configured                                                      │");
    print!("{}", PC_RESET);
    println!(
        " │ Press {}a{} to add your first user                                              │",
        pk("a"),
        PC_RESET
    );
    println!(
        " │ or press {}q{} to return to the main menu                                       │",
        pk("q"),
        PC_RESET
    );
    println!(" └─────────────────────────────────────────────────────────────────────────────┘");
    print!("{}", PC_RESET);
    println!();
}

fn print_table(users: &[UserCard]) {
    print!("{}", PC_BLUE);
    println!(" ┌─────┬──────────────────────┬─────────────────────────────────────────────┐");
    println!(
        " │ {:^3} │ {:^20} │ {:^40} │",
        pc("#"),
        pc("Username"),
        pc("Permissions")
    );
    println!(" ├─────┼──────────────────────┼─────────────────────────────────────────────┤");

    for (idx, user) in users.iter().enumerate() {
        let perms = format_permissions_summary(&user.permissions);
        let id_str = format!("{}", idx + 1);

        println!(
            " │ {:^3} │ {:<20} │ {:<40} │",
            bl(&id_str),
            bl(&user.username),
            gr(&perms)
        );
    }

    println!(" └─────┴──────────────────────┴─────────────────────────────────────────────┘");
    println!("{}", PC_RESET);
    println!();
}

fn format_permissions_summary(perms: &db::Permissions) -> String {
    let mut parts = Vec::new();
    if perms.admin_users {
        parts.push("Admin");
    }
    if perms.view_video {
        parts.push("View");
    }
    if perms.read_camera_configs {
        parts.push("ReadCam");
    }
    if perms.update_signals {
        parts.push("UpdSig");
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join(", ")
    }
}

fn show_user_detail(db: &Arc<db::Database>, user_id: i32) -> Result<(), Error> {
    let l = db.lock();
    let user = match l.users_by_id().get(&user_id) {
        Some(u) => u.clone(),
        None => {
            println!("{}User not found!", pk("⚠️ "));
            return Ok(());
        }
    };
    drop(l);

    println!();
    println!("{}User Details: {}", pc("👤 "), bl(&user.username));
    println!("{}ID: {}", pc("🆔 "), bl(&user_id.to_string()));
    println!("{}Permissions:", pc("🔐 "));
    println!(
        "  View Video: {}",
        if user.permissions.view_video {
            gr("✅")
        } else {
            pk("❌")
        }
    );
    println!(
        "  Admin Users: {}",
        if user.permissions.admin_users {
            gr("✅")
        } else {
            pk("❌")
        }
    );
    println!(
        "  Read Camera Configs: {}",
        if user.permissions.read_camera_configs {
            gr("✅")
        } else {
            pk("❌")
        }
    );
    println!(
        "  Update Signals: {}",
        if user.permissions.update_signals {
            gr("✅")
        } else {
            pk("❌")
        }
    );
    println!();
    println!(
        "{}Controls: {}p{}=change password | {}d{}=delete | {}q{}=back",
        bl("► "),
        pk("p"),
        PC_RESET,
        pk("d"),
        PC_RESET,
        pk("q"),
        PC_RESET
    );
    println!();

    let theme = ColorfulTheme::default();
    loop {
        let input: String = Input::with_theme(&theme)
            .allow_empty(true)
            .with_prompt("Enter command")
            .interact_text()
            .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

        match input.trim() {
            "p" | "password" => {
                change_password_interactive(db, user_id)?;
                break;
            }
            "d" | "delete" => {
                delete_user(db, user_id)?;
                break;
            }
            "q" | "quit" | "" => break,
            _ => {}
        }
    }

    Ok(())
}

fn add_user_interactive(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Add New User", pc("➕ "));

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username (required)")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let password: String = Input::with_theme(&theme)
        .with_prompt("Password (required)")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let confirm: String = Input::with_theme(&theme)
        .with_prompt("Confirm password")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    if password != confirm {
        println!("{}Passwords do not match.", pk("❌ "));
        return Ok(());
    }

    println!();
    println!("{}Permissions:", pc("🔐 "));
    let view_video = Confirm::with_theme(&theme)
        .with_prompt("View Video permission?")
        .default(true)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    let admin_users = Confirm::with_theme(&theme)
        .with_prompt("Admin Users permission?")
        .default(false)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    let save = Confirm::with_theme(&theme)
        .with_prompt("Save user?")
        .default(true)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !save {
        println!("{}Cancelled.", pk("⚠️ "));
        return Ok(());
    }

    let mut change = db::UserChange::add_user(username);
    change.set_password(password.into());
    change.permissions.view_video = view_video;
    change.permissions.admin_users = admin_users;

    match db.lock().apply_user_change(change) {
        Ok(_) => {
            println!("{}User added successfully!", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to add user: {}", pk("❌ "), e);
        }
    }

    Ok(())
}

fn edit_user_interactive(db: &Arc<db::Database>, users: &[UserCard]) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Select user to edit:", pc("✏️ "));

    let mut options: Vec<String> = users.iter().map(|u| format!("👤 {}", u.username)).collect();
    options.push("↩️ Back".to_string());

    let selection = Select::with_theme(&theme)
        .with_prompt("Select user")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if selection == options.len() - 1 {
        return Ok(());
    }

    let user_id = users[selection].id;
    show_user_detail(db, user_id)?;

    Ok(())
}

fn delete_user_interactive(db: &Arc<db::Database>, users: &[UserCard]) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Select user to delete:", pk("🗑️ "));

    let mut options: Vec<String> = users.iter().map(|u| format!("👤 {}", u.username)).collect();
    options.push("↩️ Back".to_string());

    let selection = Select::with_theme(&theme)
        .with_prompt("Select user")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if selection == options.len() - 1 {
        return Ok(());
    }

    let user_id = users[selection].id;
    delete_user(db, user_id)?;

    Ok(())
}

fn delete_user(db: &Arc<db::Database>, user_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    let confirm = Confirm::with_theme(&theme)
        .with_prompt(format!(
            "Delete user ID {}? This cannot be undone!",
            user_id
        ))
        .default(false)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !confirm {
        println!("{}Cancelled.", pk("⚠️ "));
        return Ok(());
    }

    match db.lock().delete_user(user_id) {
        Ok(_) => {
            println!("{}User deleted successfully!", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to delete user: {}", pk("❌ "), e);
        }
    }

    Ok(())
}

fn change_password_interactive(db: &Arc<db::Database>, user_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Change Password", pc("🔐 "));

    let new_password: String = Input::with_theme(&theme)
        .with_prompt("New password")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let confirm: String = Input::with_theme(&theme)
        .with_prompt("Confirm password")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    if new_password != confirm {
        println!("{}Passwords do not match.", pk("❌ "));
        return Ok(());
    }

    let mut l = db.lock();
    let user = match l.users_by_id().get(&user_id) {
        Some(u) => u.clone(),
        None => {
            println!("{}User not found.", pk("⚠️ "));
            return Ok(());
        }
    };

    let mut change = user.change();
    change.set_password(new_password.into());

    match l.apply_user_change(change) {
        Ok(_) => {
            println!("{}Password updated successfully.", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to update password: {}", pk("❌ "), e);
        }
    }

    Ok(())
}

pub fn run_wizard(db: &Arc<db::Database>) -> Result<(), Error> {
    run_user_ui(db)
}

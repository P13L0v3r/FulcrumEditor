// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use comrak::{markdown_to_html, Options};
use inflector::*;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use tauri::{
    api::dialog::FileDialogBuilder, CustomMenuItem, GlobalShortcutManager, Manager, Menu, Submenu,
    Window,
};

const SAVE_SHORTCUT: &str = "CommandOrControl+S";
const RENAME_SHORTCUT: &str = "CommandOrControl+Shift+R";

fn main() {
    // here `"quit".to_string()` defines the menu item id, and the second parameter is the menu item label.
    let new_file = CustomMenuItem::new("new_file".to_string(), "New");
    let open_file = CustomMenuItem::new("open_file".to_string(), "Open");
    let save_file = CustomMenuItem::new("save_file".to_string(), "Save")
        .accelerator(SAVE_SHORTCUT)
        .disabled();
    let save_as_file = CustomMenuItem::new("save_as_file".to_string(), "Save As");
    let export_file = CustomMenuItem::new("export_file".to_string(), "Export");
    let file_menu = Submenu::new(
        "File",
        Menu::new()
            .add_item(new_file)
            .add_item(open_file)
            .add_item(save_file)
            .add_item(save_as_file)
            .add_item(export_file),
    );

    let rename_symbol = CustomMenuItem::new("rename_symbol".to_string(), "Rename")
        .accelerator(RENAME_SHORTCUT)
        .disabled();
    let edit_menu = Submenu::new("Edit", Menu::new().add_item(rename_symbol));

    let menu = Menu::new().add_submenu(file_menu).add_submenu(edit_menu);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            parse_text,
            render_text,
            set_save_button,
            set_rename_button,
        ])
        .menu(menu)
        .on_menu_event(|event| {
            let event_window = event.window().clone();
            match event.menu_item_id() {
                "new_file" => {
                    event_window.emit("new-file", "").unwrap();
                }
                "open_file" => FileDialogBuilder::new()
                    .add_filter("Text Files", &["txt"])
                    .add_filter("Markdown Files", &["md"])
                    .pick_file(move |picked_file_path| {
                        if let Some(file_path) = picked_file_path {
                            event_window
                                .emit("open-file", file_path.to_str().unwrap())
                                .unwrap();
                        }
                    }),
                "save_file" => {
                    event_window.emit("save-file", "").unwrap();
                }
                "save_as_file" => FileDialogBuilder::new().save_file(move |picked_file_path| {
                    if let Some(file_path) = picked_file_path {
                        println!("{}", file_path.to_str().unwrap());
                        event_window
                            .emit("save-as-file", file_path.to_str().unwrap())
                            .unwrap();
                    }
                }),
                "export_file" => FileDialogBuilder::new()
                    .add_filter("Text Files", &["txt"])
                    .add_filter("Markdown Files", &["md"])
                    .add_filter("HTML Files", &["html"])
                    .save_file(move |picked_file_path| {
                        if let Some(file_path) = picked_file_path {
                            println!("{}", file_path.to_str().unwrap());
                            event_window
                                .emit("export-file", file_path.to_str().unwrap())
                                .unwrap();
                        }
                    }),
                "rename_symbol" => event_window.emit("rename-symbol", "").unwrap(),
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command(rename_all = "snake_case")]
fn set_save_button(enabled: bool, window: Window) -> tauri::Result<()> {
    let app_handle = window.app_handle();
    let mut shortcuts = app_handle.global_shortcut_manager();

    match enabled {
        true => {
            if !shortcuts.is_registered(SAVE_SHORTCUT)? {
                shortcuts
                    .register(SAVE_SHORTCUT, move || {
                        if let Some(window) = app_handle.get_focused_window() {
                            window.emit("save-file", "").unwrap();
                        }
                    })
                    .unwrap();
            }
        }
        false => {
            if shortcuts.is_registered(SAVE_SHORTCUT)? {
                shortcuts.unregister(SAVE_SHORTCUT).unwrap();
            }
        }
    }

    window
        .menu_handle()
        .get_item("save_file")
        .set_enabled(enabled)
}

#[tauri::command(rename_all = "snake_case")]
fn set_rename_button(enabled: bool, window: Window) -> tauri::Result<()> {
    let app_handle = window.app_handle();
    let mut shortcuts = app_handle.global_shortcut_manager();

    match enabled {
        true => {
            if !shortcuts.is_registered(RENAME_SHORTCUT)? {
                shortcuts
                    .register(RENAME_SHORTCUT, move || {
                        if let Some(window) = app_handle.get_focused_window() {
                            window.emit("rename-symbol", "").unwrap();
                        }
                    })
                    .unwrap();
            }
        }
        false => {
            if shortcuts.is_registered(RENAME_SHORTCUT)? {
                shortcuts.unregister(RENAME_SHORTCUT).unwrap();
            }
        }
    }

    window
        .menu_handle()
        .get_item("rename_symbol")
        .set_enabled(enabled)
}

const ENTITY_PREFIX: u8 = b'@';
const FORBIDDEN_NAME_CHARS: &str = ",.<>!@#$%^&*()[]|\\;\'\"?/`~+=";
const ESCAPE: u8 = b'\\';
const SENTENCE_CASE: u8 = b'^';
const TITLE_CASE: u8 = b'!';
const BLOCK_START: u8 = b'{';
const BLOCK_END: u8 = b'}';

type Entity = Map<String, Value>;

const DECLARATION_CHARS: [u8; 4] = [b'd', b'e', b'f', b' '];

#[tauri::command(rename_all = "snake_case")]
fn parse_text(text_to_parse: &str) -> String {
    let mut entity_hash: HashMap<String, Entity> = HashMap::new();

    let mut text_sections: Vec<String> = Vec::new();
    let mut current_text_section: String = String::new();

    let mut json_sections: Vec<String> = Vec::new();
    let mut json_names: Vec<String> = Vec::new();
    let mut current_json_section: String = String::new();

    let mut declaration_progress: usize = 0;
    let mut declaration_name: String = String::new();
    let mut building_json: bool = false;
    let mut saved_backup_text: String = String::new();
    let mut current_block_level: usize = 0;

    for character in text_to_parse.as_bytes() {
        if declaration_progress < DECLARATION_CHARS.len() {
            if *character == DECLARATION_CHARS[declaration_progress] {
                declaration_progress += 1;
            } else {
                declaration_progress = 0;
                current_text_section.push_str(&saved_backup_text);
                declaration_name = String::new();
                saved_backup_text = String::new();
            }
        } else if declaration_progress == DECLARATION_CHARS.len() {
            if *character == b' ' {
                json_names.push(declaration_name.clone());
                declaration_progress = DECLARATION_CHARS.len() + 1;
            } else {
                declaration_name.push(char::from(*character));
            }
        } else if *character == BLOCK_START {
            if current_block_level == 0 {
                let string_to_push = current_text_section.trim().to_string();
                if !string_to_push.is_empty() {
                    text_sections.push(string_to_push);
                }
                current_text_section = String::new();
            }
            current_block_level += 1;
            building_json = current_block_level > 0;
            saved_backup_text = String::new();
        } else if !building_json {
            declaration_progress = 0;
            current_text_section.push_str(&saved_backup_text);
            declaration_name = String::new();
            saved_backup_text = String::new();
        }

        if building_json {
            current_json_section.push(char::from(*character));
        } else if declaration_progress == 0 {
            current_text_section.push(char::from(*character));
        } else {
            saved_backup_text.push(char::from(*character));
        }

        if *character == BLOCK_END && current_block_level > 0 {
            current_block_level -= 1;
            if current_block_level == 0 {
                json_sections.push(current_json_section);
                current_json_section = String::new();
            }
            building_json = current_block_level > 0;
        }
    }

    if building_json {
        json_sections.push(current_json_section);
    } else {
        let mut string_to_push = current_text_section.trim().to_string();
        string_to_push.push('\n');
        if !string_to_push.is_empty() {
            text_sections.push(string_to_push);
        }
    }

    for (json_section, json_name) in json_sections.iter().zip(json_names.iter()) {
        if let Ok(entity) = json5::from_str::<Entity>(json_section) {
            entity_hash.insert(json_name.clone(), entity);
        }
    }

    let mut final_text_string: String = String::new();
    let mut checking_entity: bool = false;
    let mut id_to_check: String = String::new();
    let mut escaping: bool = false;

    for text_section in text_sections.iter() {
        for character in text_section.as_bytes() {
            if *character == ESCAPE {
                escaping = true;
            }

            if checking_entity {
                if char::is_whitespace(char::from(*character))
                    || FORBIDDEN_NAME_CHARS.contains(char::from(*character))
                {
                    checking_entity = false;
                    let entity_with_field: Vec<&str> = id_to_check.split(':').collect();
                    if let Some(entity_tag) = entity_with_field.first() {
                        let mut string_to_push: String = "*unknown*".to_string();
                        if let Some(entity) = entity_hash.get(*entity_tag) {
                            if let Some(entity_field) = entity_with_field.last() {
                                if let Some(field_value) = get_field_by_name(entity, entity_field) {
                                    match *character {
                                        SENTENCE_CASE => {
                                            string_to_push = field_value.to_sentence_case()
                                        }
                                        TITLE_CASE => string_to_push = field_value.to_title_case(),
                                        _ => string_to_push = field_value,
                                    }
                                }
                            }
                        }
                        final_text_string.push_str(&string_to_push);
                        id_to_check = String::new();
                    }
                } else if *character != ENTITY_PREFIX {
                    id_to_check.push(char::from(*character));
                }
            }

            if !escaping {
                if *character == ENTITY_PREFIX {
                    checking_entity = true;
                } else if !checking_entity
                    && *character != SENTENCE_CASE
                    && *character != TITLE_CASE
                    && *character != ESCAPE
                {
                    final_text_string.push(char::from(*character));
                }
            } else if *character != ESCAPE {
                final_text_string.push(char::from(*character));
                escaping = false;
            }
        }
    }

    final_text_string.trim().to_string()
}

#[tauri::command(rename_all = "snake_case")]
fn render_text(text_to_render: &str, file_extension: &str) -> String {
    match file_extension {
        "md" => markdown_to_html(text_to_render, &Options::default()),
        _ => text_to_render.to_string(),
    }
}

fn get_field_by_name(map: &Entity, field: &str) -> Option<String> {
    let value = match map.get(field) {
        Some(value) => value,
        None => return None,
    };

    match String::deserialize(value) {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

/* fn find_first_neq_index_different_lengths(a1: &[u8], a2: &[u8]) -> Option<usize> {
    let mut itera = a1.iter();
    let mut iterb = a2.iter();
    let mut i = 0usize;
    loop {
        match (itera.next(), iterb.next()) {
            (None, None) => return None,
            (None, Some(_)) | (Some(_), None) => return Some(i),
            (Some(a), Some(b)) if a != b => return Some(i),
            _ => {
                i += 1;
            }
        }
    }
} */

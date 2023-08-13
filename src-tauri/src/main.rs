// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;
use std::collections::HashMap;
use std::{env, fs};
use std::path::Path;
use std::error::Error;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io;
use std::sync::mpsc;
use std::{thread, time};
use std::time::{Duration, Instant};
use serde_json::json;
use arboard::Clipboard;
use keyring::{Entry, Result as KeyringResult};


#[macro_use] extern crate magic_crypt;

use magic_crypt::MagicCryptTrait;

#[derive(Debug, Clone)]
enum InputMode {
    Navigation,
    Command,
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Debug)]
enum Action {
    AddPassword,
    EditPassword,
}

struct Command {
    action: Action,
    params: Vec<String>,
}

#[derive(Debug, Clone)]
struct App {
    input: String,
    input_mode: InputMode,
}

impl App {
    fn decode_command(&self) -> Command {
        let mut parts = self.input.split_whitespace().into_iter();
        let command = parts.next();
        match command {
            Some("add") => Command{action: Action::AddPassword, params: parts.into_iter().map(str::to_string).collect()},
            Some("edit") => Command{action: Action::EditPassword, params: parts.into_iter().map(str::to_string).collect()},
            Some(cmd) => panic!("Command doesn't exist: {}", cmd),
            None => panic!("Unable to parse command!"),
        }
    }

    fn add_to_input(&mut self, s: &String) {
        self.input.push_str(s);
    }
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Navigation,
        }
    }
}

fn set_masterkey(masterkey: String) -> () {
    /* Add masterkey to the keyring */
    let user = env::var("USER").unwrap();
    let entry = Entry::new("vikeypass", &user).unwrap();
    entry.set_password(&masterkey).unwrap();
}

fn get_masterkey() -> KeyringResult<String> {
    /* Get masterkey from the keyring */
    let user = env::var("USER").unwrap();
    let entry = Entry::new("vikeypass", &user).unwrap();
    entry.get_password()
}

fn to_clipboard(text: &str) {
    /* Copy selected password to the system clipboard for some time. */
    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_text(text.clone()).unwrap();

    thread::spawn(move || {
        /* If the password is still in the buffer then erase it. */
        let ten_millis = time::Duration::from_millis(10000);
        thread::sleep(ten_millis);
        // TODO: Check if the clipboard has changed.
        // let current_text = clipboard.get_text().unwrap();
        clipboard.set_text("").unwrap();
    });
}

fn load_database() -> Result<HashMap<String, String>, Box<dyn Error>> {
    /* Uses password from user's keyring to decrypt the data */
    let mcrypt = new_magic_crypt!(get_masterkey().unwrap(), 256);
    let filepath = get_database_filepath();
    let encrypted_data = fs::read_to_string(filepath)
        .expect("Should have been able to read the file");
    let decrypted_data = mcrypt.decrypt_base64_to_string(&encrypted_data).unwrap();
    let map: HashMap<String, String> = serde_json::from_str(&decrypted_data).unwrap();
    Ok(map)
}

fn save_database(passwords: &HashMap<String, String>) -> () {
    /* Uses password from user's keyring to encrypt the data */
    let json_passwords = json!(passwords);
    let mcrypt = new_magic_crypt!(get_masterkey().unwrap(), 256);
    let encrypted_data = mcrypt.encrypt_str_to_base64(json_passwords.to_string());
    let filepath = get_database_filepath();
    fs::write(filepath, encrypted_data).expect("Unable to write file");
}

fn get_database_filepath() -> String {
    /* Default value is `~/.vikeypass.data` */
    let home_dir = env::var("HOME").unwrap();
    let file_env = std::env::var("VIKEYPASS_FILE");
    match  file_env {
        Ok(p) => p,
        Err(_) => String::from(Path::new(&home_dir).join(".vikeypass.data").to_str().unwrap()),
    }
}

fn execute (command: &Command) -> Result<&str, Box<dyn std::error::Error>> {
    let mut passwords = load_database().unwrap();
    match command.action {
        Action::AddPassword => {
            passwords.insert(command.params[0].clone(), command.params[1].clone());
            save_database(&passwords);
            Ok("New password has been added")

        },
        Action::EditPassword => {
            passwords.entry(command.params[0].clone()).and_modify(|pwd| *pwd = command.params[1].clone());
            save_database(&passwords);
            Ok("Updated successfully")
        },
    }
}

#[tauri::command]
fn get_passwords() -> HashMap<String, String> {
    let mut passwords = load_database().unwrap();
    passwords
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_passwords])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
} 

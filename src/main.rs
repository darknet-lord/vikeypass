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
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, List, ListItem, Paragraph, Tabs,
    },
    Terminal,
};
use serde_json::json;
use arboard::Clipboard;
use keyring::{Entry, Result as KeyringResult};

#[macro_use] extern crate magic_crypt;

use magic_crypt::MagicCryptTrait;


enum InputMode {
    Navigation,
    Command,
}

enum Event<I> {
    Input(I),
    Tick,
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


fn execute (command: &String) {
    println!("{}", command);
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Add to setup.
    // set_masterkey(String::from("masterkey"));
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let mut footer_message = "";
    let menu_titles = vec!["Home", "Add", "Edit", "Delete", "Quit"];
    let mut passwords = load_database().unwrap();
    let mut selected_idx = 0;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let mut input_mode = InputMode::Navigation;
    let mut command_buffer = String::new();

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                ).split(size);

            let command = Paragraph::new(&*command_buffer)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title("Command")
                    .border_type(BorderType::Plain),
                );


            let items: Vec<ListItem> = passwords.keys().enumerate().map(|(idx, keyname)| {
                match idx {
                    i if i == selected_idx => ListItem::new(keyname.clone())
                        .style(Style::default().bg(Color::Yellow)),
                    _ => ListItem::new(keyname.clone())
                        .style(Style::default().bg(Color::Black)),
                }
            }).collect();


            let list = List::new(items)
                .block(Block::default().title("Passwords").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                .highlight_symbol(">>");

            let footer = Paragraph::new(footer_message)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title("Status")
                    .border_type(BorderType::Plain),
                );

            rect.render_widget(command, chunks[0]);
            rect.render_widget(list, chunks[1]);
            rect.render_widget(footer, chunks[2]);
        })?;


        match rx.recv()? {
            Event::Input(event) => match input_mode {
                InputMode::Navigation => {
                    match event.code {
                        KeyCode::Char('i') => {
                            input_mode = InputMode::Command;
                            command_buffer = String::from("");
                            footer_message = "Command";
                        },
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            terminal.show_cursor()?;
                            break;
                        },
                        KeyCode::Char('j') => {
                            if selected_idx < passwords.len() - 1 {
                                selected_idx += 1
                            }
                        },
                        KeyCode::Char('k') => {
                            if selected_idx > 0 {
                                selected_idx -= 1
                            }
                        },
                        KeyCode::Char('y') => {
                            let key = passwords.keys().nth(selected_idx as usize).unwrap();
                            let pwd = passwords.get(key).unwrap();
                            to_clipboard(pwd);
                            footer_message = "Password copied for 10 seconds";
                        },
                        KeyCode::Char('d') => {
                            let key = passwords.keys().nth(selected_idx as usize).unwrap();
                            passwords.remove(&key.clone()).unwrap();
                            footer_message = "Entry has been destroyed";
                            // TODO: Save the changes.
                            // save_database(&passwords);
                        },
                        _ => (),
                    }
                },
                InputMode::Command => {
                    match event.code {
                        KeyCode::Esc => {
                            input_mode = InputMode::Navigation;
                            footer_message = "Navigation";
                        },
                        KeyCode::Enter => {
                            execute(&command_buffer);
                        },
                        KeyCode::Char(c) => {
                            command_buffer.push_str(&String::from(c));
                        },
                        _ => (),
                    }
                }
                /*
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                },
                KeyCode::Char('j') => {
                    if selected_idx < passwords.len() - 1 {
                        selected_idx += 1
                    }
                },
                KeyCode::Char('k') => {
                    if selected_idx > 0 {
                        selected_idx -= 1
                    }
                },
                KeyCode::Char('y') => {
                    let key = passwords.keys().nth(selected_idx as usize).unwrap();
                    let pwd = passwords.get(key).unwrap();
                    to_clipboard(pwd);
                    footer_message = "Password copied for 10 seconds";
                },
                KeyCode::Char('d') => {
                    let key = passwords.keys().nth(selected_idx as usize).unwrap();
                    passwords.remove(&key.clone()).unwrap();
                    footer_message = "Entry has been destroyed";
                    // TODO: Save the changes.
                    // save_database(&passwords);
                },
                _ => ()
                */
            },
            Event::Tick => {}
        }
    }
    Ok(())
} 

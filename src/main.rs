use std::collections::HashMap;
use std::fs;
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
use arboard::Clipboard;
use keyring::{Entry, Result as KeyringResult};


enum Event<I> {
    Input(I),
    Tick,
}

struct SelectedItem {
    background: Color,
}

impl SelectedItem {
    fn update_background(&mut self, color: Color) -> () {
        self.background = color;
    }
}

fn set_materpass() -> () {
    let entry = Entry::new("vikeypass", "current_user").unwrap();
    entry.set_password("masterkey").unwrap();
}

fn get_masterkey() -> KeyringResult<String> {
    let entry = Entry::new("vikeypass", "current_user").unwrap();
    entry.get_password()
}

fn to_clipboard(text: &str) {
    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_text(text).unwrap();

    thread::spawn(move || {
        let ten_millis = time::Duration::from_millis(10000);
        thread::sleep(ten_millis);
        clipboard.set_text("").unwrap();
    });
}

fn load_database() -> Result<HashMap<String, String>, Box<dyn Error>> {
    let data = fs::read_to_string(".vikeypass.json")
        .expect("Should have been able to read the file");
    let map: HashMap<String, String> = serde_json::from_str(&data).unwrap();
    Ok(map)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

            let items: Vec<ListItem> = passwords.keys().enumerate().map(|(idx, keyname)| {
                match idx {
                    i if i == selected_idx => ListItem::new(keyname.clone()).style(Style::default().bg(Color::Yellow)),
                    _ => ListItem::new(keyname.clone()).style(Style::default().bg(Color::Black)),
                }
            }).collect();

            let list = List::new(items)
                .block(Block::default().title("Passwords").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                .highlight_symbol(">>");

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

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

            rect.render_widget(tabs, chunks[0]);
            rect.render_widget(list, chunks[1]);
            rect.render_widget(footer, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
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
                },
                _ => ()
            },
            Event::Tick => {}
        }

    }
    Ok(())
} 

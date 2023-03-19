use std::collections::HashMap;
use std::{env, fs};
use std::path::Path;
use serde_json::json;
use std::error::Error;
use std::{thread, time};
use std::time::{Duration, Instant};
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

pub fn load_database() -> Result<HashMap<String, String>, Box<dyn Error>> {
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

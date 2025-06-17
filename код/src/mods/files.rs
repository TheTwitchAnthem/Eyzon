use std::io::Write;
use std::fs;
use std::io::Error;
use std::path::Path;
use chrono::Local;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = r"config.yml";
const DEFAULT_CONFIG: &str = r"";

const DIR_LOGS_PATH: &str = r"logs";
const LOG_PATH: &str = r"logs\log.txt";
const USER_LOG_PATH: &str = r"logs\user_log.txt";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    logout: bool,
    watch: bool,
    excluded_dirs: Vec<String>,
    excluded_extensions: Vec<String>
}

pub fn load_config() -> Result<Config, String> {
    let content = fs::read_to_string(CONFIG_PATH)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    serde_yaml::from_str::<Config>(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

pub fn create_files() {
    create_all_files().unwrap();
}
fn create_all_files() -> Result<(), Error> {
    if !Path::new(CONFIG_PATH).exists() {
        fs::write(CONFIG_PATH, DEFAULT_CONFIG)?;
    }

    if !Path::new(DIR_LOGS_PATH).exists() {
        fs::create_dir(DIR_LOGS_PATH)?;
    }

    if !Path::new(LOG_PATH).exists() {
        fs::write(LOG_PATH, "")?;
    }

    if !Path::new(USER_LOG_PATH).exists() {
        fs::write(USER_LOG_PATH, "")?;
    }
    
    Ok(())
}

pub fn debug(message: String) {
    debug_log(message).unwrap();
}
fn debug_log(message: String) -> Result<(), Error> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_message = format!("[{}] {}\n", timestamp, message);

    println!("{}", log_message.trim());

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)?;

    writeln!(file, "{}", log_message.trim())?;
    
    Ok(())
}

pub fn encryption(text: &String, key: &u32) -> String {
    let mut result = String::new();

    for c in text.chars() {
        let encrypted_char_code = c as u32 ^ key;
        if let Some(encrypted_char) = char::from_u32(encrypted_char_code) {
            result.push(encrypted_char);
        } else {
            result.push('?');
        }
    }

    result
}
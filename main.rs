#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use std::process::Command;
use std::io::{self, Write};
use serde::{Deserialize, Serialize};
use chrono::Local;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON, VK_MBUTTON
};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use notify::{Watcher, RecursiveMode, RecommendedWatcher, Config as NotifyConfig, EventKind, Error as NotifyError};
use std::sync::mpsc;

const CONFIG_PATH: &str = r"saves\config.yml";
const LOG_PATH: &str = r"saves\log.txt";

const DEFAULT_CONFIG: &str = r#"
update: 10
timeout: "1m"
monitor:
  create: true
  remove: true
process:
  - "msedge"
"#;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    update: i32,
    timeout: String,
    monitor: MonitorConfig,
    process: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MonitorConfig {
    create: bool,
    remove: bool,
}

fn main() -> io::Result<()> {
    init_environment()?;

    debug_log("Application started")?;

    let config = load_config().map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Config error: {}", e))
    })?;

    debug_log(&format!("Load config {:#?}", config))?;

    let timeout_seconds = parse_timeout(&config.timeout);
    let update_interval = Duration::from_millis(1000 / config.update as u64);

    let mut last_activity = Instant::now();
    let inactivity_duration = Duration::from_secs(timeout_seconds);

    debug_log(&format!("Starting monitoring with timeout: {}s", timeout_seconds))?;

    let mut last_pos = get_cursor_pos()?;

    let watch_path = Path::new(r"C:\").to_path_buf();

    debug_log(&format!("Monitoring directory: {}", watch_path.display()))?;

    let (tx, rx) = mpsc::channel();
    let notify_config = NotifyConfig::default().with_poll_interval(Duration::from_secs(1));
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.send(res) {
                eprintln!("Failed to send watch event: {}", e);
            }
        },
        notify_config
    ).map_err(|e: NotifyError| {
        io::Error::new(io::ErrorKind::Other, format!("Watcher creation error: {}", e))
    })?;

    watcher.watch(&watch_path, RecursiveMode::Recursive).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Watch error: {}", e))
    })?;

    thread::spawn(move || {
        for event in rx {
            match event {
                Ok(notify::Event { kind, paths, .. }) => {
                    for path in paths {
                        if let Err(e) = match kind {
                            EventKind::Create(_) if config.monitor.create => {
                                if path.is_dir() {
                                    debug_log(&format!("📂 Folder created: {}", path.display()))
                                } else {
                                    debug_log(&format!("📄 File created: {}", path.display()))
                                }
                            }
                            EventKind::Remove(_) if config.monitor.remove => {
                                if path.is_dir() {
                                    debug_log(&format!("🗑️ Folder deleted: {}", path.display()))
                                } else {
                                    debug_log(&format!("🗑️ File deleted: {}", path.display()))
                                }
                            }
                            _ => Ok(())
                        } {
                            eprintln!("Failed to log file event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    let _ = debug_log(&format!("Ошибка watcher: {}", e));
                    break;
                }
            }
        }
    });

    loop {
        thread::sleep(update_interval);

        if is_user_active(&mut last_pos)? {
            last_activity = Instant::now();
        }

        if last_activity.elapsed() >= inactivity_duration {
            debug_log(&format!("You've been out of work for {} seconds!", inactivity_duration.as_secs_f64()))?;
            kill_processes(&config.process);
            last_activity = Instant::now();
        }
    }
}

fn is_user_active(last_pos: &mut (i32, i32)) -> io::Result<bool> {
    let buttons_active = unsafe {
        GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0 ||
            GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000 != 0 ||
            GetAsyncKeyState(VK_MBUTTON.0 as i32) as u16 & 0x8000 != 0
    };

    let current_pos = get_cursor_pos()?;
    let mouse_moved = current_pos != *last_pos;
    *last_pos = current_pos;

    Ok(buttons_active || mouse_moved)
}

fn get_cursor_pos() -> io::Result<(i32, i32)> {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to get cursor position: {}", e))
        })?;
    }
    Ok((point.x, point.y))
}

fn init_environment() -> io::Result<()> {
    if !Path::new("saves").exists() {
        fs::create_dir("saves")?;
    }
    if !Path::new(CONFIG_PATH).exists() {
        fs::write(CONFIG_PATH, DEFAULT_CONFIG)?;
    }
    Ok(())
}

fn load_config() -> Result<Config, String> {
    let content = fs::read_to_string(CONFIG_PATH)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    serde_yaml::from_str::<Config>(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

fn parse_timeout(timeout_str: &str) -> u64 {
    let timeout_str = timeout_str.trim();

    if timeout_str.ends_with('s') {
        timeout_str.trim_end_matches('s').parse().unwrap_or(10)
    } else if timeout_str.ends_with('m') {
        timeout_str.trim_end_matches('m').parse::<u64>().unwrap_or(10) * 60
    } else if timeout_str.ends_with('h') {
        timeout_str.trim_end_matches('h').parse::<u64>().unwrap_or(10) * 3600
    } else {
        timeout_str.parse().unwrap_or(10)
    }
}

fn kill_processes(process_names: &[String]) {
    for name in process_names {
        let result = Command::new("taskkill")
            .args(["/F", "/IM", &format!("{}.exe", name)])
            .output();

        match result {
            Ok(output) => {
                let status = if output.status.success() {
                    "success"
                } else {
                    "failed"
                };
                let _ = debug_log(&format!("Killing {}: {}, output: {}", name, status,
                                           String::from_utf8_lossy(&output.stdout)));
            }
            Err(e) => {
                let _ = debug_log(&format!("Failed to kill {}: {}", name, e));
            }
        }
    }
}

fn debug_log<T: std::fmt::Display>(message: T) -> io::Result<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_entry = format!("[{}] {}\n", timestamp, message);

    println!("{}", log_entry.trim());

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)?;

    writeln!(file, "{}", log_entry.trim())?;
    Ok(())
}
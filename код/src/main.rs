#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mods;

use std::thread;
use std::time::Duration;
use mods::files::{create_files, debug, load_config, Config};
use mods::watch;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    create_files();
    
    debug(format!("Application start by version {}", VERSION));

    let config: Config = load_config().unwrap();
}
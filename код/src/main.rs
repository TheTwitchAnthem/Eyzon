#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mods;

use mods::files::{create_files, debug, config, Config};
use mods::watch::FileWatcher;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    create_files();
    
    debug(format!("Application start by version {}", VERSION));

    watch(config());
}

fn watch(config: Config) {
    let watcher: FileWatcher = FileWatcher::new(config);

    watcher.run().unwrap();
}
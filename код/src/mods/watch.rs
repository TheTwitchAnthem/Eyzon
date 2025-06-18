use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::Config as notifyConfig;
use std::path::{Path};
use std::time::{Duration, SystemTime};
use chrono::Local;
use std::collections::HashMap;
use crate::mods::files::{Config, debug};

pub struct FileWatcher {
    config: Config
}

impl FileWatcher {
    pub fn new(config: Config) -> Self {
        FileWatcher { config }
    }

    fn hidden(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        let is_temporary = self.config.excluded_extensions.iter()
            .any(|ext| path_str.ends_with(ext));
        let is_excluded_dir = path.iter()
            .any(|comp| self.config.excluded_dirs.contains(&comp.to_string_lossy().as_ref().to_string()));

        is_excluded_dir || is_temporary
    }

    pub fn run(&self) -> notify::Result<()> {
        let path = Path::new(r"C:\");
        let config = notifyConfig::default()
            .with_poll_interval(Duration::from_millis(250));

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: RecommendedWatcher = Watcher::new(tx, config)?;

        watcher.watch(&path, RecursiveMode::Recursive)?;

        let mut last_events: HashMap<String, SystemTime> = HashMap::new();
        let debounce_duration = Duration::from_secs(1);

        for res in rx {
            match res {
                Ok(event) => {
                    if event.paths.iter().any(|p| self.hidden(p)) {
                        continue;
                    }

                    let now = SystemTime::now();
                    let event_key = match event.kind {
                        EventKind::Create(_) => format!("Create:{:?}", event.paths),
                        EventKind::Modify(_) => format!("Modify:{:?}", event.paths),
                        EventKind::Remove(_) => format!("Remove:{:?}", event.paths),
                        EventKind::Access(_) => continue,
                        EventKind::Other => continue,
                        _ => continue,
                    };

                    let should_print = match last_events.get(&event_key) {
                        Some(last_time) => now.duration_since(*last_time).unwrap() >= debounce_duration,
                        None => true,
                    };

                    if should_print {
                        last_events.insert(event_key.clone(), now);

                        match event.kind {
                            EventKind::Create(_) => debug(format!("{} ➕ Create: {:?}",
                                                             Local::now().format("%Y-%m-%d %H:%M:%S"), event.paths)),
                            EventKind::Modify(_) => debug(format!("{} ✏️ Modify: {:?}",
                                                             Local::now().format("%Y-%m-%d %H:%M:%S"), event.paths)),
                            EventKind::Remove(_) => debug(format!("{} 🗑️ Remove: {:?}",
                                                             Local::now().format("%Y-%m-%d %H:%M:%S"), event.paths)),
                            _ => (),
                        }
                    }
                }
                Err(e) => debug(format!("⚠️ Error: {:?}", e)),
            }

            if last_events.len() > 1000 {
                let now = SystemTime::now();
                last_events.retain(|_, time| now.duration_since(*time).unwrap() < debounce_duration * 2);
            }
        }
        Ok(())
    }  
}
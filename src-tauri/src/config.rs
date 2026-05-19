use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_separator")]
    pub separator: String,
    #[serde(default)]
    pub repos: Vec<Repo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub path: String,
}

fn default_shortcut() -> String {
    "Cmd+Shift+P".to_string()
}

fn default_separator() -> String {
    "\n\n---\n\n".to_string()
}

pub fn config_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    home.join(".config").join("prompt-picker")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Expand ~ in a path to the actual home directory.
/// Used by the indexer in Phase 3.
#[allow(dead_code)]
pub fn expand_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(stripped)
    } else if path == "~" {
        dirs::home_dir().expect("Could not determine home directory")
    } else {
        PathBuf::from(path)
    }
}

const DEFAULT_CONFIG: &str = r#"# Global keyboard shortcut to open the picker
shortcut = "Cmd+Shift+P"

# Separator inserted between prompt components when concatenating
separator = "\n\n---\n\n"

# Prompt repositories — folders containing markdown files
# Uncomment and edit the examples below:
#
# [[repos]]
# name = "My Prompts"
# path = "~/Documents/prompts"
#
# [[repos]]
# name = "Work"
# path = "~/work/prompt-library"
"#;

pub fn create_default_config() -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let path = config_path();
    fs::write(&path, DEFAULT_CONFIG).map_err(|e| format!("Failed to write default config: {e}"))?;
    Ok(())
}

pub fn load_config() -> Result<Config, String> {
    let path = config_path();
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read config file: {e}"))?;
    let config: Config =
        toml::from_str(&content).map_err(|e| format!("Failed to parse config: {e}"))?;
    Ok(config)
}

pub fn ensure_config() -> Result<Config, String> {
    let path = config_path();
    if !path.exists() {
        create_default_config()?;
    }
    load_config()
}

pub fn watch_config(app_handle: AppHandle) {
    let path = config_path();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut debouncer =
            new_debouncer(Duration::from_millis(500), tx).expect("Failed to create config watcher");

        debouncer
            .watcher()
            .watch(path.parent().unwrap(), notify::RecursiveMode::NonRecursive)
            .expect("Failed to watch config directory");

        // Keep debouncer alive and process events
        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    let config_changed = events.iter().any(|e| e.path == path);
                    if config_changed {
                        match load_config() {
                            Ok(config) => {
                                println!("Config reloaded: {:?}", config);
                                let _ = app_handle.emit("config-changed", &config);
                            }
                            Err(e) => {
                                eprintln!("Failed to reload config: {e}");
                            }
                        }
                    }
                }
                Ok(Err(error)) => {
                    eprintln!("Config watch error: {error:?}");
                }
                Err(_) => break, // Channel closed
            }
        }
    });
}

pub fn open_config_file() -> Result<(), String> {
    let path = config_path();
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open config file: {e}"))?;
    Ok(())
}

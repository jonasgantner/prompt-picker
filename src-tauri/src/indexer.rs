use crate::config::{self, Config};
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub path: String,
    pub repo: String,
    pub name: String,
    #[serde(rename = "type")]
    pub prompt_type: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub section: Option<String>,
    #[serde(rename = "sectionName")]
    pub section_name: Option<String>,
    #[serde(rename = "sectionIcon")]
    pub section_icon: Option<String>,
    #[serde(rename = "sectionOrder")]
    pub section_order: Option<i64>,
    pub order: Option<i64>,
    pub extends: Vec<String>,
    #[serde(rename = "hasExtends")]
    pub has_extends: bool,
    #[serde(rename = "extendsCount")]
    pub extends_count: usize,
}

#[derive(Deserialize, Default)]
struct Frontmatter {
    #[serde(rename = "type")]
    prompt_type: Option<String>,
    name: Option<String>,
    extends: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    pinned: Option<bool>,
    section: Option<String>,
    section_name: Option<String>,
    section_icon: Option<String>,
    section_order: Option<i64>,
    order: Option<i64>,
}

/// Derive a display name from a filename: strip .md, replace - and _ with spaces, title-case
fn name_from_filename(filename: &str) -> String {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    stem.split(|c: char| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract YAML frontmatter from markdown content.
/// Returns (frontmatter_yaml, body) if frontmatter exists.
fn extract_frontmatter(content: &str) -> Option<(&str, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    // Find the end of the opening ---
    let after_first = &trimmed[3..];
    let after_first = after_first.strip_prefix('\r').unwrap_or(after_first);
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    // Find the closing ---
    if let Some(end_pos) = after_first.find("\n---") {
        let yaml = &after_first[..end_pos];
        let rest_start = end_pos + 4; // skip \n---
        let rest = if rest_start < after_first.len() {
            let r = &after_first[rest_start..];
            // Skip past the newline after closing ---
            let r = r.strip_prefix('\r').unwrap_or(r);
            r.strip_prefix('\n').unwrap_or(r)
        } else {
            ""
        };
        Some((yaml, rest))
    } else {
        None
    }
}

fn parse_prompt_file(repo_name: &str, repo_root: &Path, file_path: &Path) -> Option<Prompt> {
    let content = std::fs::read_to_string(file_path).ok()?;
    let rel_path = file_path
        .strip_prefix(repo_root)
        .ok()?
        .to_string_lossy()
        .to_string();

    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let frontmatter = extract_frontmatter(&content)
        .and_then(|(yaml, _)| serde_yaml::from_str::<Frontmatter>(yaml).ok())
        .unwrap_or_default();

    let display_name = frontmatter
        .name
        .unwrap_or_else(|| name_from_filename(&filename));

    let extends = frontmatter.extends.unwrap_or_default();
    let has_extends = !extends.is_empty();
    let extends_count = extends.len(); // Placeholder: Phase 5 computes transitive count

    Some(Prompt {
        path: rel_path,
        repo: repo_name.to_string(),
        name: display_name,
        prompt_type: frontmatter.prompt_type,
        tags: frontmatter.tags.unwrap_or_default(),
        pinned: frontmatter.pinned.unwrap_or(false),
        section: frontmatter.section,
        section_name: frontmatter.section_name,
        section_icon: frontmatter.section_icon,
        section_order: frontmatter.section_order,
        order: frontmatter.order,
        extends,
        has_extends,
        extends_count,
    })
}

pub fn scan(config: &Config) -> Vec<Prompt> {
    let mut prompts = Vec::new();

    for repo in &config.repos {
        let repo_path = config::expand_path(&repo.path);
        if !repo_path.exists() {
            eprintln!("Repo path does not exist, skipping: {}", repo.path);
            continue;
        }

        for entry in WalkDir::new(&repo_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                if let Some(prompt) = parse_prompt_file(&repo.name, &repo_path, path) {
                    prompts.push(prompt);
                }
            }
        }
    }

    prompts
}

pub fn watch_repos(config: &Config, app_handle: AppHandle) {
    let repo_paths: Vec<std::path::PathBuf> = config
        .repos
        .iter()
        .map(|r| config::expand_path(&r.path))
        .filter(|p| p.exists())
        .collect();

    if repo_paths.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut debouncer =
            new_debouncer(Duration::from_millis(200), tx).expect("Failed to create repo watcher");

        for path in &repo_paths {
            if let Err(e) = debouncer
                .watcher()
                .watch(path, notify::RecursiveMode::Recursive)
            {
                eprintln!("Failed to watch repo {}: {e}", path.display());
            }
        }

        loop {
            match rx.recv() {
                Ok(Ok(events)) => {
                    let md_changed = events.iter().any(|e| {
                        e.path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                    });
                    if md_changed {
                        match config::load_config() {
                            Ok(cfg) => {
                                let prompts = scan(&cfg);
                                let _ = app_handle.emit("prompts-changed", &prompts);
                            }
                            Err(e) => {
                                eprintln!("Failed to reload config for rescan: {e}");
                            }
                        }
                    }
                }
                Ok(Err(error)) => {
                    eprintln!("Repo watch error: {error:?}");
                }
                Err(_) => break,
            }
        }
    });
}

/// Strip YAML frontmatter from content, returning just the body text trimmed.
pub fn strip_frontmatter(content: &str) -> String {
    match extract_frontmatter(content) {
        Some((_, body)) => body.trim().to_string(),
        None => content.trim().to_string(),
    }
}

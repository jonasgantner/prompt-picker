use crate::config::{self, Config};
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
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
    #[serde(rename = "useWhen")]
    pub use_when: Option<String>,
    #[serde(rename = "suggestedFollowUp")]
    pub suggested_follow_up: Option<String>,
    pub alternatives: Vec<String>,
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
    use_when: Option<String>,
    suggested_follow_up: Option<String>,
    alternatives: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct SectionMetadata {
    key: Option<String>,
    folder: Option<String>,
    name: Option<String>,
    icon: Option<String>,
    order: Option<i64>,
}

#[derive(Deserialize, Default)]
struct RepoManifest {
    sections: Option<Vec<SectionMetadata>>,
    categories: Option<Vec<SectionMetadata>>,
    ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct SectionInfo {
    key: String,
    name: Option<String>,
    icon: Option<String>,
    order: Option<i64>,
}

#[derive(Default)]
struct RepoMetadata {
    by_key: HashMap<String, SectionInfo>,
    by_folder: HashMap<String, SectionInfo>,
    ignore: Vec<String>,
}

fn normalize_category_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_folder(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('/');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.replace('\\', "/"))
    }
}

fn split_ordered_name(value: &str) -> (Option<i64>, String) {
    let mut digits = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }

    if digits.is_empty() {
        return (None, value.to_string());
    }

    let rest = &value[digits.len()..];
    let rest = rest.trim_start_matches(['-', '_', ' ']);
    if rest.is_empty() {
        return (None, value.to_string());
    }

    (digits.parse::<i64>().ok(), rest.to_string())
}

fn title_from_key(value: &str) -> String {
    value
        .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
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

fn section_info_from_folder(folder: &str) -> Option<SectionInfo> {
    let folder_name = Path::new(folder).file_name()?.to_string_lossy();
    let (order, key_part) = split_ordered_name(&folder_name);
    let key = normalize_category_key(&key_part)?;
    Some(SectionInfo {
        name: Some(title_from_key(&key)),
        key,
        icon: None,
        order,
    })
}

fn category_manifest_paths(repo_root: &Path) -> [PathBuf; 2] {
    [
        repo_root.join("prompt-picker.toml"),
        repo_root.join(".prompt-picker.toml"),
    ]
}

fn load_repo_metadata(repo_root: &Path) -> RepoMetadata {
    for path in category_manifest_paths(repo_root) {
        if !path.exists() {
            continue;
        }

        match fs::read_to_string(&path)
            .ok()
            .and_then(|content| toml::from_str::<RepoManifest>(&content).ok())
        {
            Some(manifest) => {
                let mut metadata = RepoMetadata {
                    ignore: manifest.ignore.unwrap_or_default(),
                    ..RepoMetadata::default()
                };

                for section in manifest
                    .sections
                    .unwrap_or_default()
                    .into_iter()
                    .chain(manifest.categories.unwrap_or_default())
                {
                    let folder = section.folder.as_deref().and_then(normalize_folder);
                    let folder_info = folder.as_deref().and_then(section_info_from_folder);
                    let key = section
                        .key
                        .as_deref()
                        .and_then(normalize_category_key)
                        .or_else(|| folder_info.as_ref().map(|info| info.key.clone()));

                    if let Some(key) = key {
                        let info = SectionInfo {
                            key: key.clone(),
                            name: section.name.clone().or_else(|| {
                                folder_info.as_ref().and_then(|info| info.name.clone())
                            }),
                            icon: section.icon.clone(),
                            order: section
                                .order
                                .or_else(|| folder_info.as_ref().and_then(|info| info.order)),
                        };

                        metadata.by_key.insert(key, info.clone());
                        if let Some(folder) = folder {
                            metadata.by_folder.insert(folder, info);
                        }
                    }
                }

                return metadata;
            }
            None => {
                eprintln!("Failed to parse category manifest: {}", path.display());
                return RepoMetadata::default();
            }
        }
    }

    RepoMetadata::default()
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

fn is_placeholder_name(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    normalized.is_empty() || normalized == "untitled"
}

fn relative_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn first_folder(rel_path: &Path) -> Option<String> {
    let parent = rel_path.parent()?;
    let mut components = parent.components();
    let first = components.next()?;

    match first {
        std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
        _ => None,
    }
}

fn matches_ignore_pattern(pattern: &str, rel_path: &str, filename: &str) -> bool {
    let pattern = pattern.trim().trim_start_matches("./");
    if pattern.is_empty() {
        return false;
    }

    if let Some(suffix) = pattern.strip_prefix("**/") {
        return rel_path == suffix || rel_path.ends_with(&format!("/{suffix}"));
    }

    if !pattern.contains('/') && filename == pattern {
        return true;
    }

    rel_path == pattern
}

fn is_ignored(metadata: &RepoMetadata, rel_path: &str, filename: &str) -> bool {
    metadata
        .ignore
        .iter()
        .any(|pattern| matches_ignore_pattern(pattern, rel_path, filename))
}

fn resolve_section(
    rel_path: &Path,
    metadata: &RepoMetadata,
    frontmatter: &Frontmatter,
) -> (Option<String>, Option<String>, Option<String>, Option<i64>) {
    if let Some(folder) = first_folder(rel_path) {
        let derived = section_info_from_folder(&folder);
        let folder_info = metadata.by_folder.get(&folder);
        let key = folder_info
            .map(|info| info.key.clone())
            .or_else(|| derived.as_ref().map(|info| info.key.clone()));

        if let Some(key) = key {
            let key_info = metadata.by_key.get(&key);
            let info = folder_info.or(key_info);
            let name = info
                .and_then(|info| info.name.clone())
                .or_else(|| derived.as_ref().and_then(|info| info.name.clone()));
            let icon = info.and_then(|info| info.icon.clone());
            let order = info
                .and_then(|info| info.order)
                .or_else(|| derived.as_ref().and_then(|info| info.order));

            return (Some(key), name, icon, order);
        }
    }

    let section = frontmatter
        .section
        .as_deref()
        .and_then(normalize_category_key);
    let info = section.as_ref().and_then(|key| metadata.by_key.get(key));
    let section_name = info
        .and_then(|info| info.name.clone())
        .or_else(|| frontmatter.section_name.clone());
    let section_icon = info
        .and_then(|info| info.icon.clone())
        .or_else(|| frontmatter.section_icon.clone());
    let section_order = info
        .and_then(|info| info.order)
        .or(frontmatter.section_order);

    (section, section_name, section_icon, section_order)
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

fn parse_prompt_file(
    repo_name: &str,
    repo_root: &Path,
    file_path: &Path,
    metadata: &RepoMetadata,
) -> Option<Prompt> {
    let content = std::fs::read_to_string(file_path).ok()?;
    let rel_path_buf = file_path.strip_prefix(repo_root).ok()?;
    let rel_path = relative_path_string(rel_path_buf);

    let filename = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let frontmatter = extract_frontmatter(&content)
        .and_then(|(yaml, _)| serde_yaml::from_str::<Frontmatter>(yaml).ok())
        .unwrap_or_default();

    if is_ignored(metadata, &rel_path, &filename) {
        return None;
    }

    let (section, section_name, section_icon, section_order) =
        resolve_section(rel_path_buf, metadata, &frontmatter);
    let filename_name = name_from_filename(&filename);
    let display_name = if is_placeholder_name(&filename_name) {
        frontmatter.name.unwrap_or(filename_name)
    } else {
        filename_name
    };

    let extends = frontmatter.extends.unwrap_or_default();
    let has_extends = !extends.is_empty();
    let extends_count = extends.len(); // Placeholder: Phase 5 computes transitive count

    Some(Prompt {
        path: rel_path,
        repo: repo_name.to_string(),
        name: display_name,
        prompt_type: frontmatter
            .prompt_type
            .or_else(|| section.as_ref().map(|_| "prompt".to_string())),
        tags: frontmatter.tags.unwrap_or_default(),
        pinned: frontmatter.pinned.unwrap_or(false),
        section,
        section_name,
        section_icon,
        section_order,
        order: frontmatter.order,
        use_when: frontmatter.use_when,
        suggested_follow_up: frontmatter.suggested_follow_up,
        alternatives: frontmatter.alternatives.unwrap_or_default(),
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
        let metadata = load_repo_metadata(&repo_path);

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
                if let Some(prompt) = parse_prompt_file(&repo.name, &repo_path, path, &metadata) {
                    prompts.push(prompt);
                }
            }
        }
    }

    prompts
}

fn update_repo_watches(
    watcher: &mut dyn notify::Watcher,
    config: &Config,
    watched: &mut HashSet<PathBuf>,
) {
    let current: HashSet<_> = config
        .repos
        .iter()
        .map(|repo| config::expand_path(&repo.path))
        .filter(|path| path.is_dir())
        .collect();

    for path in watched.difference(&current) {
        let _ = watcher.unwatch(path);
    }
    watched.retain(|path| current.contains(path));

    for path in current {
        if !watched.contains(&path) {
            match watcher.watch(&path, notify::RecursiveMode::Recursive) {
                Ok(()) => {
                    watched.insert(path);
                }
                Err(error) => eprintln!("Failed to watch {}: {error}", path.display()),
            }
        }
    }
}

pub fn watch_sources(mut config: Config, app: AppHandle) {
    let config_path = config::config_path();
    let config_dir = config::config_dir();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut debouncer = match new_debouncer(Duration::from_millis(200), tx) {
            Ok(debouncer) => debouncer,
            Err(error) => {
                eprintln!("Failed to start the prompt watcher: {error}");
                return;
            }
        };
        if let Err(error) = debouncer
            .watcher()
            .watch(&config_dir, notify::RecursiveMode::NonRecursive)
        {
            eprintln!("Failed to watch the configuration directory: {error}");
        }
        let mut watched = HashSet::new();
        update_repo_watches(debouncer.watcher(), &config, &mut watched);

        while let Ok(result) = rx.recv() {
            let events = match result {
                Ok(events) => events,
                Err(error) => {
                    eprintln!("Prompt watch error: {error:?}");
                    continue;
                }
            };

            let config_changed = events.iter().any(|event| event.path == config_path);
            if config_changed {
                match config::load_config() {
                    Ok(updated) => {
                        config = updated;
                        update_repo_watches(debouncer.watcher(), &config, &mut watched);
                        let _ = app.emit("config-changed", &config);
                    }
                    Err(error) => eprintln!("Failed to reload config: {error}"),
                }
            }
            let prompts_changed = events.iter().any(|event| {
                let path = &event.path;
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                    || path.file_name().is_some_and(|name| {
                        name == "prompt-picker.toml" || name == ".prompt-picker.toml"
                    })
            });
            if config_changed || prompts_changed {
                crate::publish_prompts(&app, &config);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Repo};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo() -> PathBuf {
        static NEXT_REPO: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT_REPO.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "prompt-picker-indexer-test-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_uses_repo_category_manifest_before_prompt_category_fields() {
        let repo_dir = temp_repo();
        fs::write(
            repo_dir.join(".prompt-picker.toml"),
            r#"
[[categories]]
key = "start"
name = "Start Here"
icon = "play-circle"
order = 10
"#,
        )
        .unwrap();
        fs::write(
            repo_dir.join("prompt.md"),
            r#"---
type: prompt
name: "Prompt"
section: start
section_name: Wrong
section_icon: search
section_order: 99
order: 10
pinned: true
---
Body
"#,
        )
        .unwrap();

        let config = Config {
            shortcut: "Cmd+Shift+P".to_string(),
            separator: "\n\n---\n\n".to_string(),
            repos: vec![Repo {
                name: "test".to_string(),
                path: repo_dir.to_string_lossy().to_string(),
            }],
        };

        let prompts = scan(&config);
        let prompt = prompts
            .iter()
            .find(|prompt| prompt.path == "prompt.md")
            .unwrap();

        assert_eq!(prompt.section.as_deref(), Some("start"));
        assert_eq!(prompt.section_name.as_deref(), Some("Start Here"));
        assert_eq!(prompt.section_icon.as_deref(), Some("play-circle"));
        assert_eq!(prompt.section_order, Some(10));

        fs::remove_dir_all(repo_dir).unwrap();
    }

    #[test]
    fn scan_derives_section_from_prompt_folder_and_manifest() {
        let repo_dir = temp_repo();
        let start_dir = repo_dir.join("01-start");
        fs::create_dir_all(&start_dir).unwrap();
        fs::write(
            repo_dir.join("prompt-picker.toml"),
            r#"
ignore = ["README.md", "**/Scratch.md"]

[[sections]]
folder = "01-start"
key = "start"
name = "Start Here"
icon = "play-circle"
order = 10
"#,
        )
        .unwrap();
        fs::write(repo_dir.join("README.md"), "Docs").unwrap();
        fs::write(start_dir.join("Scratch.md"), "Scratch").unwrap();
        fs::write(
            start_dir.join("Filename Prompt.md"),
            r#"---
name: "Frontmatter Prompt"
order: 10
---
Body
"#,
        )
        .unwrap();

        let config = Config {
            shortcut: "Cmd+Shift+P".to_string(),
            separator: "\n\n---\n\n".to_string(),
            repos: vec![Repo {
                name: "test".to_string(),
                path: repo_dir.to_string_lossy().to_string(),
            }],
        };

        let prompts = scan(&config);
        assert_eq!(prompts.len(), 1);

        let prompt = &prompts[0];
        assert_eq!(prompt.path, "01-start/Filename Prompt.md");
        assert_eq!(prompt.name, "Filename Prompt");
        assert_eq!(prompt.prompt_type.as_deref(), Some("prompt"));
        assert_eq!(prompt.section.as_deref(), Some("start"));
        assert_eq!(prompt.section_name.as_deref(), Some("Start Here"));
        assert_eq!(prompt.section_icon.as_deref(), Some("play-circle"));
        assert_eq!(prompt.section_order, Some(10));

        fs::remove_dir_all(repo_dir).unwrap();
    }

    #[test]
    fn scan_uses_frontmatter_name_for_untitled_files() {
        let repo_dir = temp_repo();
        let start_dir = repo_dir.join("01-start");
        fs::create_dir_all(&start_dir).unwrap();
        fs::write(
            repo_dir.join("prompt-picker.toml"),
            r#"
[[sections]]
folder = "01-start"
key = "start"
name = "Start"
order = 10
"#,
        )
        .unwrap();
        fs::write(
            start_dir.join("Untitled.md"),
            r#"---
name: "Drafted Prompt"
---
Body
"#,
        )
        .unwrap();

        let config = Config {
            shortcut: "Cmd+Shift+P".to_string(),
            separator: "\n\n---\n\n".to_string(),
            repos: vec![Repo {
                name: "test".to_string(),
                path: repo_dir.to_string_lossy().to_string(),
            }],
        };

        let prompts = scan(&config);
        let prompt = prompts
            .iter()
            .find(|prompt| prompt.path == "01-start/Untitled.md")
            .unwrap();

        assert_eq!(prompt.name, "Drafted Prompt");
        assert_eq!(prompt.prompt_type.as_deref(), Some("prompt"));
        assert_eq!(prompt.section.as_deref(), Some("start"));

        fs::remove_dir_all(repo_dir).unwrap();
    }

    #[test]
    fn scan_derives_section_from_folder_without_manifest() {
        let repo_dir = temp_repo();
        let review_dir = repo_dir.join("02-review");
        fs::create_dir_all(&review_dir).unwrap();
        fs::write(review_dir.join("prompt.md"), "Body").unwrap();

        let config = Config {
            shortcut: "Cmd+Shift+P".to_string(),
            separator: "\n\n---\n\n".to_string(),
            repos: vec![Repo {
                name: "test".to_string(),
                path: repo_dir.to_string_lossy().to_string(),
            }],
        };

        let prompts = scan(&config);
        let prompt = prompts
            .iter()
            .find(|prompt| prompt.path == "02-review/prompt.md")
            .unwrap();

        assert_eq!(prompt.section.as_deref(), Some("review"));
        assert_eq!(prompt.section_name.as_deref(), Some("Review"));
        assert_eq!(prompt.section_order, Some(2));

        fs::remove_dir_all(repo_dir).unwrap();
    }
}

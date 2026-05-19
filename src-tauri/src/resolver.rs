use crate::indexer::Prompt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedChain {
    pub items: Vec<ChainItem>,
    pub errors: Vec<ChainError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainItem {
    pub path: String,
    pub repo: String,
    pub name: String,
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    pub paths: Vec<String>,
}

pub fn resolve_chain(path: &str, repo: &str, prompts: &[Prompt]) -> ResolvedChain {
    // Build lookup map: (repo, path) -> &Prompt
    let lookup: HashMap<(&str, &str), &Prompt> = prompts
        .iter()
        .map(|p| ((p.repo.as_str(), p.path.as_str()), p))
        .collect();

    let mut result_items: Vec<ChainItem> = Vec::new();
    let mut errors: Vec<ChainError> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();

    // DFS to collect dependencies in topological order
    fn dfs(
        current_path: &str,
        repo: &str,
        lookup: &HashMap<(&str, &str), &Prompt>,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
        result: &mut Vec<ChainItem>,
        errors: &mut Vec<ChainError>,
        is_root: bool,
    ) {
        let key = format!("{repo}:{current_path}");

        // Cycle detection
        if stack.contains(&key) {
            let cycle_start = stack.iter().position(|s| s == &key).unwrap();
            let cycle_path: Vec<String> = stack[cycle_start..]
                .iter()
                .map(|s| s.split(':').nth(1).unwrap_or(s).to_string())
                .collect();
            let mut display = cycle_path.clone();
            display.push(current_path.to_string());
            errors.push(ChainError {
                error_type: "cycle".to_string(),
                message: format!("Cycle detected: {}", display.join(" → ")),
                paths: cycle_path,
            });
            return;
        }

        // Already fully processed
        if visited.contains(&key) {
            return;
        }

        stack.push(key.clone());

        if let Some(prompt) = lookup.get(&(repo, current_path)) {
            // Recursively resolve dependencies first
            for dep_path in &prompt.extends {
                dfs(
                    dep_path, repo, lookup, visited, stack, result, errors, false,
                );
            }
        } else if !is_root {
            errors.push(ChainError {
                error_type: "missing".to_string(),
                message: format!("Missing dependency: {current_path}"),
                paths: vec![current_path.to_string()],
            });
            stack.pop();
            return;
        }

        stack.pop();
        visited.insert(key);

        // Add this item if not already in result (deduplication)
        if !result
            .iter()
            .any(|item| item.path == current_path && item.repo == repo)
        {
            let (name, item_repo) = if let Some(prompt) = lookup.get(&(repo, current_path)) {
                (prompt.name.clone(), prompt.repo.clone())
            } else {
                (current_path.to_string(), repo.to_string())
            };

            result.push(ChainItem {
                path: current_path.to_string(),
                repo: item_repo,
                name,
                auto: !is_root,
            });
        }
    }

    dfs(
        path,
        repo,
        &lookup,
        &mut visited,
        &mut stack,
        &mut result_items,
        &mut errors,
        true,
    );

    ResolvedChain {
        items: result_items,
        errors,
    }
}

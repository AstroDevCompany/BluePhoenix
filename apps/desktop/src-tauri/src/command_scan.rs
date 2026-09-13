use crate::error::{AppError, AppResult};
use crate::project_facts;
use serde::{Deserialize, Serialize};
use std::path::Path;

const COMMAND_FILE_CAP: usize = 64_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundCommand {
    pub name: String,
    pub command: String,
    pub description: String,
    pub working_directory: Option<String>,
    pub reasoning: String,
}

pub fn extract_found_commands(path: &Path) -> AppResult<Vec<FoundCommand>> {
    if !path.exists() || !path.is_dir() {
        return Err(AppError::FolderMissing);
    }
    let root = path.canonicalize().map_err(|_| AppError::FolderMissing)?;
    let tree = project_facts::walk_rel_files(&root);
    let mut out = Vec::new();
    for rel in &tree {
        let file_name = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
        let Some(body) = project_facts::read_rel(&root, rel, COMMAND_FILE_CAP) else {
            continue;
        };
        let dir_rel = rel.rsplit_once('/').map(|(dir, _)| dir.to_string());
        match file_name.as_str() {
            "package.json" => out.extend(from_package_json(&body, &root, dir_rel.as_deref(), rel)),
            "composer.json" => out.extend(from_composer_json(&body, dir_rel.as_deref(), rel)),
            "makefile" | "gnumakefile" => {
                out.extend(from_make_targets(&body, dir_rel.as_deref(), rel))
            }
            "justfile" => out.extend(from_just_recipes(&body, dir_rel.as_deref(), rel)),
            _ => {}
        }
    }
    dedup_found(&mut out);
    Ok(out)
}

fn from_package_json(
    body: &str,
    root: &Path,
    dir_rel: Option<&str>,
    rel: &str,
) -> Vec<FoundCommand> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|s| s.as_object()) else {
        return Vec::new();
    };
    let pkg_dir = match dir_rel {
        Some(dir) => root.join(dir),
        None => root.to_path_buf(),
    };
    let pm = package_manager(&pkg_dir, root);
    let wd = nested_workdir(dir_rel);
    scripts
        .keys()
        .filter_map(|name| {
            let name = name.trim();
            if !is_script_name(name) {
                return None;
            }
            Some(FoundCommand {
                name: display_name(name),
                command: format!("{pm} run {name}"),
                description: String::new(),
                working_directory: wd.clone(),
                reasoning: format!("{rel} scripts.{name}"),
            })
        })
        .collect()
}

fn from_composer_json(body: &str, dir_rel: Option<&str>, rel: &str) -> Vec<FoundCommand> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|s| s.as_object()) else {
        return Vec::new();
    };
    let wd = nested_workdir(dir_rel);
    scripts
        .keys()
        .filter_map(|name| {
            let name = name.trim();
            if !is_script_name(name) {
                return None;
            }
            Some(FoundCommand {
                name: display_name(name),
                command: format!("composer run-script {name}"),
                description: String::new(),
                working_directory: wd.clone(),
                reasoning: format!("{rel} scripts.{name}"),
            })
        })
        .collect()
}

fn from_make_targets(body: &str, dir_rel: Option<&str>, rel: &str) -> Vec<FoundCommand> {
    let wd = nested_workdir(dir_rel);
    makefile_targets(body)
        .into_iter()
        .map(|name| FoundCommand {
            name: display_name(&name),
            command: format!("make {name}"),
            description: String::new(),
            working_directory: wd.clone(),
            reasoning: format!("{rel} target {name}"),
        })
        .collect()
}

fn from_just_recipes(body: &str, dir_rel: Option<&str>, rel: &str) -> Vec<FoundCommand> {
    let wd = nested_workdir(dir_rel);
    just_recipes(body)
        .into_iter()
        .map(|name| FoundCommand {
            name: display_name(&name),
            command: format!("just {name}"),
            description: String::new(),
            working_directory: wd.clone(),
            reasoning: format!("{rel} recipe {name}"),
        })
        .collect()
}

fn package_manager(dir: &Path, root: &Path) -> &'static str {
    let mut cur = dir.to_path_buf();
    loop {
        if cur.join("pnpm-lock.yaml").is_file() {
            return "pnpm";
        }
        if cur.join("yarn.lock").is_file() {
            return "yarn";
        }
        if cur.join("bun.lock").is_file() || cur.join("bun.lockb").is_file() {
            return "bun";
        }
        if cur.join("package-lock.json").is_file() {
            return "npm";
        }
        if cur == root {
            break;
        }
        if !cur.pop() {
            break;
        }
    }
    "npm"
}

fn nested_workdir(dir_rel: Option<&str>) -> Option<String> {
    dir_rel
        .map(|dir| dir.replace('\\', "/"))
        .filter(|dir| !dir.is_empty() && dir != ".")
}

fn makefile_targets(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        if line.starts_with('\t') || line.starts_with(' ') || line.starts_with('#') {
            continue;
        }
        let trimmed = line.trim_end();
        let Some((name, rest)) = trimmed.split_once(':') else {
            continue;
        };
        if rest.starts_with('=') {
            continue;
        }
        let name = name.trim();
        if name.is_empty()
            || name.starts_with('.')
            || name.contains('%')
            || name.contains('$')
            || name.contains(char::is_whitespace)
            || !is_script_name(name)
        {
            continue;
        }
        if !out.iter().any(|existing| existing == name) {
            out.push(name.to_string());
        }
    }
    out
}

fn just_recipes(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        if line.starts_with('\t') || line.starts_with(' ') || line.starts_with('#') {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("alias ") || trimmed.contains(":=") {
            continue;
        }
        let Some((head, _)) = trimmed.split_once(':') else {
            continue;
        };
        let Some(name) = head.split_whitespace().next() else {
            continue;
        };
        if !is_script_name(name) {
            continue;
        }
        if !out.iter().any(|existing| existing == name) {
            out.push(name.to_string());
        }
    }
    out
}

fn is_script_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

fn display_name(script: &str) -> String {
    let mut out = String::new();
    let mut cap = true;
    for ch in script.replace(['-', '_', ':', '/'], " ").chars() {
        if ch.is_whitespace() {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            cap = true;
            continue;
        }
        if cap {
            for upper in ch.to_uppercase() {
                out.push(upper);
            }
            cap = false;
        } else {
            out.push(ch);
        }
    }
    let name = out.trim().to_string();
    if name.is_empty() {
        script.to_string()
    } else {
        name
    }
}

fn dedup_found(items: &mut Vec<FoundCommand>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| {
        let key = format!(
            "{}|{}",
            normalize_command(&item.command),
            item.working_directory.as_deref().unwrap_or("")
        );
        seen.insert(key)
    });
}

pub fn normalize_command(command: &str) -> String {
    command
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn package_json_uses_pnpm_and_nested_workdir() {
        let dir = std::env::temp_dir().join(format!("bp-cmds-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("apps").join("web")).unwrap();
        fs::write(dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
        fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"dev":"vite","test:unit":"vitest"}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("apps").join("web").join("package.json"),
            r#"{"scripts":{"build":"vite build"}}"#,
        )
        .unwrap();
        let found = extract_found_commands(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        let root_dev = found
            .iter()
            .find(|c| c.command == "pnpm run dev")
            .expect("root dev");
        assert_eq!(root_dev.name, "Dev");
        assert!(root_dev.working_directory.is_none());
        assert!(root_dev.reasoning.contains("scripts.dev"));
        let nested = found
            .iter()
            .find(|c| c.command == "pnpm run build")
            .expect("nested build");
        assert_eq!(nested.working_directory.as_deref(), Some("apps/web"));
        assert!(found.iter().any(|c| c.command == "pnpm run test:unit"));
    }

    #[test]
    fn makefile_and_justfile_targets() {
        let dir = std::env::temp_dir().join(format!("bp-make-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Makefile"),
            "CC=gcc\n.PHONY: all\nall: build\nbuild:\n\tcargo build\n",
        )
        .unwrap();
        fs::write(
            dir.join("justfile"),
            "# comment\ntest:\n  cargo test\nalias t := test\n",
        )
        .unwrap();
        let found = extract_found_commands(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(found.iter().any(|c| c.command == "make all"));
        assert!(found.iter().any(|c| c.command == "make build"));
        assert!(!found.iter().any(|c| c.command.contains("PHONY")));
        assert!(found.iter().any(|c| c.command == "just test"));
        assert!(!found.iter().any(|c| c.command == "just t"));
    }

    #[test]
    fn composer_scripts() {
        let dir = std::env::temp_dir().join(format!("bp-php-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("composer.json"),
            r#"{"scripts":{"test":"phpunit","serve":"php -S localhost:8000"}}"#,
        )
        .unwrap();
        let found = extract_found_commands(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(found
            .iter()
            .any(|c| c.command == "composer run-script test"));
        assert!(found
            .iter()
            .any(|c| c.command == "composer run-script serve"));
    }

    #[test]
    fn display_name_titleizes_script() {
        assert_eq!(display_name("dev"), "Dev");
        assert_eq!(display_name("test:unit"), "Test Unit");
    }
}

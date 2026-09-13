use crate::error::{AppError, AppResult};
use crate::git;
use crate::native;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const EXCERPT_BUDGET: usize = 28_000;
const FILE_EXCERPT_CAP: usize = 3_500;
const EXTRA_FILE_CAP: usize = 3_000;
const MAX_TREE_PATHS: usize = 180;
const MAX_WALK_DEPTH: usize = 5;
const MAX_EXTRA_FILES: usize = 6;

const MANIFESTS: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "go.mod",
    "composer.json",
    "pubspec.yaml",
];

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    "Pods",
    "coverage",
    ".turbo",
    "out",
    "bin",
    "obj",
    ".cache",
];

const SKIP_NAMES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "bun.lock",
    "bun.lockb",
    "composer.lock",
    "poetry.lock",
];

const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "pdf", "zip", "gz", "tgz", "wasm", "so",
    "dylib", "dll", "exe", "bin", "woff", "woff2", "ttf", "eot", "mp4", "mp3", "mov", "wav", "map",
    "lock",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFacts {
    pub folder_name: String,
    pub file_names: Vec<String>,
    pub evidence: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub github_url: Option<String>,
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareFolderDraft {
    pub name: String,
    pub description: String,
    pub github_url: String,
    pub website_url: String,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
}

pub fn collect(path: &Path, git_bin: &str) -> AppResult<ProjectFacts> {
    if !path.exists() || !path.is_dir() {
        return Err(AppError::FolderMissing);
    }
    let root = path.canonicalize().map_err(|_| AppError::FolderMissing)?;
    let folder_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let listing = native::list_dir_shallow(&root)?;
    let file_names: Vec<String> = listing.iter().map(|e| e.name.clone()).collect();
    let tree = walk_rel_files(&root);

    let mut parsed = StructuredFields::default();
    let mut excerpts = Vec::new();
    let mut used = 0usize;

    for rel in ranked_excerpt_paths(&tree) {
        if used >= EXCERPT_BUDGET {
            break;
        }
        let cap = FILE_EXCERPT_CAP.min(EXCERPT_BUDGET - used);
        let Some(body) = read_rel(&root, &rel, cap) else {
            continue;
        };
        if let Some(file_name) = rel.rsplit('/').next() {
            if MANIFESTS.iter().any(|m| m.eq_ignore_ascii_case(file_name)) {
                merge_structured(&mut parsed, file_name, &body);
            }
        }
        let block = format!("{rel}:\n{body}");
        used += block.len();
        excerpts.push(block);
    }

    let remote = git::origin_url(git_bin, &root);
    if let Some(url) = remote.as_deref() {
        excerpts.push(format!("Git remote origin: {url}"));
        if parsed.github_url.is_none() {
            parsed.github_url = git::github_https_url(url);
        }
    }

    let mut evidence = String::new();
    evidence.push_str(&format!("Folder: {folder_name}\n"));
    evidence.push_str("Tree:\n");
    for rel in tree.iter().take(MAX_TREE_PATHS) {
        evidence.push_str(rel);
        evidence.push('\n');
    }
    if tree.len() > MAX_TREE_PATHS {
        evidence.push_str(&format!(
            "… {} more files omitted\n",
            tree.len() - MAX_TREE_PATHS
        ));
    }
    for block in &excerpts {
        evidence.push('\n');
        evidence.push_str(block);
        evidence.push('\n');
    }

    Ok(ProjectFacts {
        folder_name,
        file_names,
        evidence,
        name: parsed.name,
        description: parsed.description,
        github_url: parsed.github_url,
        website_url: parsed.website_url,
    })
}

pub fn merge_draft(facts: &ProjectFacts, model: SoftwareFolderDraft) -> SoftwareFolderDraft {
    SoftwareFolderDraft {
        name: first_nonempty([
            &model.name,
            &facts.name.clone().unwrap_or_default(),
            &facts.folder_name,
        ]),
        description: first_nonempty([
            &model.description,
            &facts.description.clone().unwrap_or_default(),
        ]),
        github_url: prefer_nonempty(
            facts.github_url.clone(),
            evidence_url(&model.github_url, &facts.evidence).unwrap_or_default(),
        )
        .unwrap_or_default(),
        website_url: prefer_nonempty(
            facts.website_url.clone(),
            evidence_url(&model.website_url, &facts.evidence).unwrap_or_default(),
        )
        .unwrap_or_default(),
        languages: model.languages,
        frameworks: model.frameworks,
    }
}

/// Read extra relative paths the model asked for. Stay inside `root`, read-only.
pub fn read_extra_files(root: &Path, rels: &[String]) -> String {
    let Ok(root) = root.canonicalize() else {
        return String::new();
    };
    let mut out = String::new();
    let mut used = 0usize;
    for rel in rels.iter().take(MAX_EXTRA_FILES) {
        if used >= EXCERPT_BUDGET / 2 {
            break;
        }
        let cap = EXTRA_FILE_CAP.min(EXCERPT_BUDGET / 2 - used);
        let Some(body) = read_rel(&root, rel, cap) else {
            continue;
        };
        let block = format!("{rel}:\n{body}\n");
        used += block.len();
        out.push('\n');
        out.push_str(&block);
    }
    out
}

pub fn url_supported_by_evidence(url: &str, evidence: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return false;
    }
    if evidence.contains(url) {
        return true;
    }
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim_end_matches('/')
        .trim_end_matches(".git");
    if stripped.is_empty() {
        return false;
    }
    if evidence.contains(stripped) {
        return true;
    }
    if let Some(rest) = stripped.strip_prefix("github.com/") {
        evidence.contains(&format!("github.com:{rest}")) || evidence.contains(rest)
    } else {
        false
    }
}

fn evidence_url(url: &str, evidence: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    url_supported_by_evidence(url, evidence).then(|| url.to_string())
}

fn first_nonempty<const N: usize>(values: [&str; N]) -> String {
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    String::new()
}

fn prefer_nonempty(structured: Option<String>, model: String) -> Option<String> {
    if let Some(value) = structured
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return Some(value);
    }
    let model = model.trim();
    if model.is_empty() {
        None
    } else {
        Some(model.to_string())
    }
}

#[derive(Default)]
struct StructuredFields {
    name: Option<String>,
    description: Option<String>,
    github_url: Option<String>,
    website_url: Option<String>,
}

fn merge_structured(parsed: &mut StructuredFields, filename: &str, body: &str) {
    match filename {
        "package.json" | "composer.json" => apply_json_manifest(parsed, body),
        "Cargo.toml" => apply_toml_section(parsed, body, "[package]"),
        "pyproject.toml" => {
            apply_toml_section(parsed, body, "[project]");
            if parsed.website_url.is_none() {
                parsed.website_url = toml_table_url(body, "[project.urls]", "Homepage")
                    .or_else(|| toml_table_url(body, "[project.urls]", "homepage"));
            }
            if parsed.github_url.is_none() {
                parsed.github_url = toml_table_url(body, "[project.urls]", "Repository")
                    .or_else(|| toml_table_url(body, "[project.urls]", "repository"))
                    .and_then(|u| {
                        git::github_https_url(&u)
                            .or(Some(u))
                            .filter(|u| u.contains("github.com"))
                    });
            }
        }
        "go.mod" => {
            if parsed.github_url.is_none() {
                parsed.github_url = go_module_github(body);
            }
            if parsed.name.is_none() {
                parsed.name = go_module_name(body);
            }
        }
        "pubspec.yaml" => apply_yaml_manifest(parsed, body),
        _ => {}
    }
}

fn apply_json_manifest(parsed: &mut StructuredFields, body: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return;
    };
    if parsed.name.is_none() {
        parsed.name = json_string(&value, "name");
    }
    if parsed.description.is_none() {
        parsed.description = json_string(&value, "description");
    }
    if parsed.website_url.is_none() {
        parsed.website_url = json_string(&value, "homepage").filter(|u| looks_like_url(u));
    }
    if parsed.github_url.is_none() {
        parsed.github_url = json_repo_url(&value).and_then(|u| git::github_https_url(&u));
    }
}

fn apply_toml_section(parsed: &mut StructuredFields, body: &str, section: &str) {
    if parsed.name.is_none() {
        parsed.name = toml_field(body, section, "name");
    }
    if parsed.description.is_none() {
        parsed.description = toml_field(body, section, "description");
    }
    if parsed.website_url.is_none() {
        parsed.website_url = toml_field(body, section, "homepage").filter(|u| looks_like_url(u));
    }
    if parsed.github_url.is_none() {
        parsed.github_url =
            toml_field(body, section, "repository").and_then(|u| git::github_https_url(&u));
    }
}

fn apply_yaml_manifest(parsed: &mut StructuredFields, body: &str) {
    if parsed.name.is_none() {
        parsed.name = yaml_field(body, "name");
    }
    if parsed.description.is_none() {
        parsed.description = yaml_field(body, "description");
    }
    if parsed.website_url.is_none() {
        parsed.website_url = yaml_field(body, "homepage").filter(|u| looks_like_url(u));
    }
    if parsed.github_url.is_none() {
        parsed.github_url = yaml_field(body, "repository").and_then(|u| git::github_https_url(&u));
    }
}

fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn json_repo_url(value: &serde_json::Value) -> Option<String> {
    let repo = value.get("repository")?;
    if let Some(url) = repo.as_str() {
        return Some(url.to_string());
    }
    json_string(repo, "url")
}

fn toml_field(body: &str, section: &str, field: &str) -> Option<String> {
    let mut in_section = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.eq_ignore_ascii_case(section);
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(value) = toml_key_value(trimmed, field) {
            return Some(value);
        }
    }
    None
}

fn toml_table_url(body: &str, section: &str, field: &str) -> Option<String> {
    toml_field(body, section, field).filter(|u| looks_like_url(u))
}

fn toml_key_value(line: &str, field: &str) -> Option<String> {
    let (key, rest) = line.split_once('=')?;
    if !key.trim().eq_ignore_ascii_case(field) {
        return None;
    }
    let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn yaml_field(body: &str, field: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        let (key, rest) = trimmed.split_once(':')?;
        if key.trim() != field {
            continue;
        }
        let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn go_module_github(body: &str) -> Option<String> {
    let module = go_module_path(body)?;
    git::github_https_url(&format!("https://{module}"))
}

fn go_module_name(body: &str) -> Option<String> {
    go_module_path(body).and_then(|m| m.rsplit('/').next().map(ToOwned::to_owned))
}

fn go_module_path(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("module ") {
            let module = rest.trim();
            if !module.is_empty() {
                return Some(module.to_string());
            }
        }
    }
    None
}

fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn walk_rel_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_WALK_DEPTH || out.len() >= MAX_TREE_PATHS {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                if skip_dir_name(&name) {
                    continue;
                }
                dirs.push(path);
                continue;
            }
            if !meta.is_file() || skip_file_name(&name) {
                continue;
            }
            let Ok(canon) = path.canonicalize() else {
                continue;
            };
            if !is_within(root, &canon) {
                continue;
            }
            if let Ok(rel) = canon.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
            if out.len() >= MAX_TREE_PATHS {
                break;
            }
        }
        if out.len() >= MAX_TREE_PATHS {
            break;
        }
        for path in dirs.into_iter().rev() {
            stack.push((path, depth + 1));
        }
    }
    out.sort();
    out
}

fn ranked_excerpt_paths(tree: &[String]) -> Vec<String> {
    let mut ranked: Vec<(i32, String)> = tree
        .iter()
        .cloned()
        .map(|rel| (excerpt_score(&rel), rel))
        .filter(|(score, _)| *score > 0)
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, rel)| rel).collect()
}

fn excerpt_score(rel: &str) -> i32 {
    let depth = rel.bytes().filter(|b| *b == b'/').count() as i32;
    let name = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
    let lower = rel.to_ascii_lowercase();
    let mut score = 0;
    if name.starts_with("readme") {
        score = 120;
    } else if MANIFESTS.iter().any(|m| name.eq_ignore_ascii_case(m)) {
        score = 110;
    } else if name == "about.md" || name == "description.md" || name == "intro.md" {
        score = 95;
    } else if matches!(
        name.as_str(),
        "app.tsx"
            | "app.vue"
            | "main.rs"
            | "lib.rs"
            | "main.py"
            | "app.py"
            | "index.ts"
            | "index.tsx"
            | "index.js"
            | "main.go"
            | "main.swift"
    ) {
        score = 80;
    } else if lower.ends_with("src/main.rs")
        || lower.ends_with("src/lib.rs")
        || lower.ends_with("src/index.ts")
        || lower.ends_with("src/index.tsx")
        || lower.ends_with("src/app.tsx")
        || lower.ends_with("src/main.py")
        || lower.ends_with("cmd/main.go")
    {
        score = 85;
    } else if name.ends_with(".md") && depth <= 1 {
        score = 60;
    } else if name.ends_with(".md") && depth <= 2 {
        score = 40;
    }
    if score == 0 {
        return 0;
    }
    score - depth * 4
}

fn skip_dir_name(name: &str) -> bool {
    SKIP_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
}

fn skip_file_name(name: &str) -> bool {
    if SKIP_NAMES.iter().any(|n| name.eq_ignore_ascii_case(n)) {
        return true;
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    name.contains('.') && SKIP_EXTS.contains(&ext.as_str())
}

fn read_rel(root: &Path, rel: &str, max: usize) -> Option<String> {
    let canon = resolve_inside(root, rel)?;
    if !canon.is_file() {
        return None;
    }
    let bytes = fs::read(&canon).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Some(cap_bytes(text, max))
}

fn resolve_inside(root: &Path, rel: &str) -> Option<std::path::PathBuf> {
    let rel = rel.trim().replace('\\', "/");
    if rel.is_empty() || rel.contains('\0') {
        return None;
    }
    let path = Path::new(&rel);
    if path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    let joined = root.join(path);
    let canon = joined.canonicalize().ok()?;
    is_within(root, &canon).then_some(canon)
}

fn is_within(root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(root)
}

fn cap_bytes(text: String, max: usize) -> String {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_json_and_readme_fill_structured_fields() {
        let dir = std::env::temp_dir().join(format!("bp-facts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("package.json"),
            r#"{"name":"demo-app","description":"A demo","homepage":"https://example.com","repository":"https://github.com/org/demo"}"#,
        )
        .unwrap();
        fs::write(dir.join("README.md"), "# Demo\nThe best demo.\n").unwrap();
        let facts = collect(&dir, "").unwrap();
        assert_eq!(facts.name.as_deref(), Some("demo-app"));
        assert_eq!(facts.description.as_deref(), Some("A demo"));
        assert_eq!(facts.website_url.as_deref(), Some("https://example.com"));
        assert_eq!(
            facts.github_url.as_deref(),
            Some("https://github.com/org/demo")
        );
        assert!(facts.evidence.contains("The best demo"));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("src").join("lib.rs"),
            "//! Nested identity crate.\n",
        )
        .unwrap();
        let nested = collect(&dir, "").unwrap();
        assert!(nested.evidence.contains("src/lib.rs"));
        assert!(nested.evidence.contains("Nested identity crate"));
        assert!(read_extra_files(&dir, &["src/lib.rs".into()]).contains("Nested identity crate"));
        assert!(read_extra_files(&dir, &["../etc/passwd".into(), "/etc/passwd".into()]).is_empty());
        assert!(dir.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn merge_keeps_model_title_and_drops_invented_urls() {
        let facts = ProjectFacts {
            folder_name: "demo".into(),
            file_names: vec!["README.md".into()],
            evidence: "Folder: demo\nhttps://github.com/org/demo\n".into(),
            name: Some("demo-app".into()),
            description: Some("from package".into()),
            github_url: Some("https://github.com/org/demo".into()),
            website_url: None,
        };
        let model = SoftwareFolderDraft {
            name: "Demo App".into(),
            description: "A tool that helps you ship.".into(),
            github_url: "https://evil.example".into(),
            website_url: "https://also-evil.example".into(),
            languages: vec!["TypeScript".into()],
            frameworks: vec![],
        };
        let merged = merge_draft(&facts, model);
        assert_eq!(merged.name, "Demo App");
        assert_eq!(merged.description, "A tool that helps you ship.");
        assert_eq!(merged.github_url, "https://github.com/org/demo");
        assert!(merged.website_url.is_empty());
        assert_eq!(merged.languages, vec!["TypeScript"]);
    }

    #[test]
    fn evidence_accepts_ssh_remote_shape() {
        let evidence = "Git remote origin: git@github.com:org/repo.git\n";
        assert!(url_supported_by_evidence(
            "https://github.com/org/repo",
            evidence
        ));
        assert!(!url_supported_by_evidence(
            "https://invented.example",
            evidence
        ));
    }
}

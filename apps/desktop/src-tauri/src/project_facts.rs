use crate::error::{AppError, AppResult};
use crate::git;
use crate::native;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const README_LIMIT: usize = 8_192;
const MANIFEST_LIMIT: usize = 4_096;

const MANIFESTS: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "go.mod",
    "composer.json",
    "pubspec.yaml",
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

    let mut excerpts = Vec::new();
    let mut parsed = StructuredFields::default();

    if let Some((name, body)) = read_readme(&root, &file_names) {
        excerpts.push(format!("{name}:\n{body}"));
    }
    for manifest in MANIFESTS {
        if let Some(body) = read_under(&root, &file_names, manifest, MANIFEST_LIMIT) {
            merge_structured(&mut parsed, manifest, &body);
            excerpts.push(format!("{manifest}:\n{body}"));
        }
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
    evidence.push_str(&format!("Files: {}\n", file_names.join(", ")));
    for block in &excerpts {
        evidence.push('\n');
        evidence.push_str(block);
        evidence.push('\n');
    }

    if parsed.name.is_none() && !folder_name.is_empty() {
        parsed.name = Some(folder_name.clone());
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
        name: prefer_nonempty(facts.name.clone(), model.name).unwrap_or_else(|| facts.folder_name.clone()),
        description: prefer_nonempty(facts.description.clone(), model.description).unwrap_or_default(),
        github_url: prefer_nonempty(facts.github_url.clone(), evidence_url(&model.github_url, &facts.evidence).unwrap_or_default())
            .unwrap_or_default(),
        website_url: prefer_nonempty(facts.website_url.clone(), evidence_url(&model.website_url, &facts.evidence).unwrap_or_default())
            .unwrap_or_default(),
        languages: model.languages,
        frameworks: model.frameworks,
    }
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

fn prefer_nonempty(structured: Option<String>, model: String) -> Option<String> {
    if let Some(value) = structured.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
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
                    .and_then(|u| git::github_https_url(&u).or(Some(u)).filter(|u| u.contains("github.com")));
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
        parsed.github_url = toml_field(body, section, "repository").and_then(|u| git::github_https_url(&u));
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
    value.get(key)?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(ToOwned::to_owned)
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

fn read_readme(root: &Path, names: &[String]) -> Option<(String, String)> {
    let name = names.iter().find(|n| {
        let lower = n.to_lowercase();
        lower == "readme" || lower.starts_with("readme.")
    })?;
    let body = read_under(root, names, name, README_LIMIT)?;
    Some((name.clone(), body))
}

fn read_under(root: &Path, names: &[String], wanted: &str, max: usize) -> Option<String> {
    let actual = names.iter().find(|n| n.eq_ignore_ascii_case(wanted))?;
    let candidate = root.join(actual);
    let canon = candidate.canonicalize().ok()?;
    if !is_within(root, &canon) || !canon.is_file() {
        return None;
    }
    let bytes = fs::read(&canon).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Some(cap_bytes(text, max))
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
        assert_eq!(facts.github_url.as_deref(), Some("https://github.com/org/demo"));
        assert!(facts.evidence.contains("The best demo"));
        assert!(dir.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn merge_drops_invented_urls_and_prefers_structured() {
        let facts = ProjectFacts {
            folder_name: "demo".into(),
            file_names: vec!["README.md".into()],
            evidence: "Folder: demo\nhttps://github.com/org/demo\n".into(),
            name: Some("demo".into()),
            description: Some("from package".into()),
            github_url: Some("https://github.com/org/demo".into()),
            website_url: None,
        };
        let model = SoftwareFolderDraft {
            name: "hallucinated".into(),
            description: "invented blurb".into(),
            github_url: "https://evil.example".into(),
            website_url: "https://also-evil.example".into(),
            languages: vec!["TypeScript".into()],
            frameworks: vec![],
        };
        let merged = merge_draft(&facts, model);
        assert_eq!(merged.name, "demo");
        assert_eq!(merged.description, "from package");
        assert_eq!(merged.github_url, "https://github.com/org/demo");
        assert!(merged.website_url.is_empty());
        assert_eq!(merged.languages, vec!["TypeScript"]);
    }

    #[test]
    fn evidence_accepts_ssh_remote_shape() {
        let evidence = "Git remote origin: git@github.com:org/repo.git\n";
        assert!(url_supported_by_evidence("https://github.com/org/repo", evidence));
        assert!(!url_supported_by_evidence("https://invented.example", evidence));
    }
}

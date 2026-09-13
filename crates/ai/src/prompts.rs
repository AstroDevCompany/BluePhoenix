use bluephoenix_domain::context::AiProjectBundle;
use bluephoenix_domain::ids::CategoryKind;

use crate::documents::system_prompt;

pub fn chat_system(kind: Option<CategoryKind>) -> String {
    match kind {
        Some(k) => system_prompt(k).to_string(),
        None => {
            "You are BluePhoenix, a local-first command center assistant. There is no open project. Answer generally. Do not invent the user's projects, files, or documents.".into()
        }
    }
}

pub fn bundle_to_prompt(bundle: &AiProjectBundle) -> String {
    let mut out = String::new();
    if let Some(snap) = &bundle.snapshot {
        out.push_str(&format!(
            "Project: {} ({})\nDescription: {}\nStatus: {:?}\nTracked seconds: {}\nOpen TODOs: {}\n",
            snap.name,
            snap.kind.as_str(),
            snap.description,
            snap.status,
            snap.tracked_seconds,
            snap.open_todos
        ));
        if let Some(v) = &snap.current_version {
            out.push_str(&format!("Current version: {v}\n"));
        }
        if let Some(g) = snap.github_url.as_ref() {
            out.push_str(&format!("GitHub: {g}\n"));
        }
    }
    if !bundle.tags.is_empty() {
        out.push_str(&format!("Tags: {}\n", bundle.tags.join(", ")));
    }
    if !bundle.todos.is_empty() {
        out.push_str("TODOs:\n");
        for t in bundle.todos.iter().take(40) {
            out.push_str(&format!(
                "- [{}] {} ({}/{}) {}\n",
                t.id, t.title, t.status, t.priority, t.description
            ));
        }
    }
    if !bundle.versions.is_empty() {
        out.push_str("Versions / changelog snippets:\n");
        for v in bundle.versions.iter().take(8) {
            out.push_str(&format!(
                "- {} {}: {}\n",
                v.version,
                v.title.clone().unwrap_or_default(),
                truncate(&v.changelog, 400)
            ));
        }
    }
    if !bundle.links.is_empty() {
        out.push_str("Links:\n");
        for l in bundle.links.iter().take(12) {
            out.push_str(&format!("- {} ({})\n", l.title, l.url));
        }
    }
    if !bundle.topics.is_empty() {
        out.push_str("Topics:\n");
        for t in &bundle.topics {
            out.push_str(&format!("- {} [{}]\n", t.title, t.status));
        }
    }
    if !bundle.exams.is_empty() {
        out.push_str("Exams:\n");
        for e in &bundle.exams {
            out.push_str(&format!(
                "- {} {} {}\n",
                e.date.clone().unwrap_or_else(|| "undated".into()),
                e.status,
                e.grade.clone().unwrap_or_default()
            ));
        }
    }
    if !bundle.activity.is_empty() {
        out.push_str("Recent activity:\n");
        for a in bundle.activity.iter().take(12) {
            out.push_str(&format!(
                "- {} {} {}\n",
                a.created_at, a.event_type, a.summary
            ));
        }
    }
    out
}

pub fn search_prompt(index_json: &str, query: &str) -> String {
    format!(
        "Select the most relevant project IDs for this search. Return JSON only: {{\"projectIds\":[...],\"explanation\":\"...\"}}.\nQuery: {query}\nIndex:\n{index_json}"
    )
}

pub fn prioritize_prompt(todos_json: &str) -> String {
    format!(
        "Propose a TODO order. Return JSON only: {{\"order\":[{{\"id\":\"...\",\"reason\":\"...\"}}],\"summary\":\"...\"}}. Use only the given ids.\nTODOs:\n{todos_json}"
    )
}

pub fn changelog_summarize_prompt(body: &str) -> String {
    format!("Summarize this changelog for a developer. Be concise.\n\n{body}")
}

pub fn changelog_generate_prompt(git_log: &str, versions: &str) -> String {
    format!(
        "Draft a changelog from git history since the last recorded version. Return JSON only: {{\"title\":\"...\",\"body\":\"...\"}}.\nGit log:\n{git_log}\nExisting versions:\n{versions}"
    )
}

pub fn todos_from_changelog_prompt(body: &str) -> String {
    format!(
        "Propose TODOs implied by this changelog. Return JSON only: {{\"todos\":[{{\"title\":\"...\",\"description\":\"...\",\"priority\":\"medium\",\"reasoning\":\"...\"}}]}}.\nChangelog:\n{body}"
    )
}

pub fn agent_prompt_builder(bundle: &str, extra: &str) -> String {
    format!(
        "Write a copyable coding-agent prompt. Return JSON only: {{\"prompt\":\"...\",\"known\":[\"...\"],\"assumptions\":[\"...\"]}}. Label known facts vs assumptions. Do not invent files.\nContext:\n{bundle}\nExtra:\n{extra}"
    )
}

pub fn commit_prompt(diff: &str, log: &str, follow_style: bool) -> String {
    let style = if follow_style && !log.trim().is_empty() {
        format!("Match the style of these recent subjects:\n{log}\n")
    } else {
        "Use a clear conventional subject line.\n".into()
    };
    format!(
        "Suggest a git commit message. Never claim you committed. Return JSON only: {{\"message\":\"...\",\"rationale\":\"...\"}}.\n{style}\nDiff:\n{diff}"
    )
}

pub fn inspect_software_folder_prompt(evidence: &str, languages: &str, frameworks: &str) -> String {
    format!(
        "Read the folder tree and file excerpts, then fill software project metadata. Return JSON only: {{\"name\":\"\",\"description\":\"\",\"githubUrl\":\"\",\"websiteUrl\":\"\",\"languages\":[],\"frameworks\":[],\"needFiles\":[]}}.\n\
Reason from the files: prefer a README title or human product name over a package slug. Write a 1-2 sentence description of what the project does, grounded in README, manifests, and source comments. Do not leave name or description empty if the files explain the project. Do not invent URLs or facts that are not supported by the files. Never write or modify files.\n\
If a listed relative path would materially improve name/description and was not excerpted, put it in needFiles (max 6). languages and frameworks must be chosen only from the allowed lists.\n\
Allowed languages: {languages}\n\
Allowed frameworks: {frameworks}\n\
Files:\n{evidence}"
    )
}

pub fn scan_commands_prompt(evidence: &str, found_json: &str, existing_json: &str) -> String {
    format!(
        "Propose runnable project commands from configs and the folder tree. Return JSON only: {{\"commands\":[{{\"name\":\"\",\"command\":\"\",\"description\":\"\",\"workingDirectory\":null,\"source\":\"found\",\"reasoning\":\"\"}}],\"needFiles\":[]}}.\n\
Found scripts already extracted:\n{found_json}\n\
Commands already saved in BluePhoenix (do not repeat):\n{existing_json}\n\
source must be \"found\" (from those scripts or an explicit Makefile/just/package script) or \"hypothesized\" (ordinary stack tasks implied by the files, such as cargo test, pytest, docker compose up).\n\
Do not invent scripts that the tree does not support. Never write or modify files. Never run commands.\n\
Each command must be a simple argv line with no shell operators (no &&, ||, |, ;, redirects, backticks, env assignments).\n\
workingDirectory is a relative folder or null for the project root.\n\
If a listed relative path would materially improve the list and was not excerpted, put it in needFiles (max 6).\n\
Files:\n{evidence}"
    )
}

pub fn document_qa_suffix(chunks: &str, empty: bool) -> String {
    if empty {
        "No document excerpts were retrieved. Do not invent document content. Say you do not have that text.".into()
    } else {
        format!("Use only these excerpts. Cite document/page/section when present. If they are insufficient, say so.\n{chunks}")
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bluephoenix_domain::context::ProjectContext;
    use bluephoenix_domain::ids::{CategoryKind, ProjectStatus};

    #[test]
    fn prompts_are_kind_aware() {
        assert!(chat_system(Some(CategoryKind::Software)).contains("software"));
        assert!(chat_system(Some(CategoryKind::University)).contains("university"));
        assert!(chat_system(Some(CategoryKind::Generic)).contains("personal project"));
        assert!(chat_system(None).contains("no open project"));
    }

    #[test]
    fn bundle_includes_todos_not_full_db() {
        let bundle = AiProjectBundle {
            snapshot: Some(ProjectContext {
                project_id: "p".into(),
                category_id: "c".into(),
                kind: CategoryKind::Software,
                capabilities: Default::default(),
                name: "App".into(),
                description: "d".into(),
                status: ProjectStatus::Active,
                tracked_seconds: 10,
                xp: 1,
                open_todos: 1,
                github_url: None,
                website_url: None,
                current_version: Some("1.0.0".into()),
                final_grade: None,
                cfu: None,
            }),
            todos: vec![bluephoenix_domain::context::AiTodoSlice {
                id: "t1".into(),
                title: "Fix sync".into(),
                description: "".into(),
                priority: "high".into(),
                status: "open".into(),
            }],
            ..AiProjectBundle::default()
        };
        let text = bundle_to_prompt(&bundle);
        assert!(text.contains("Fix sync"));
        assert!(text.contains("1.0.0"));
        assert!(!text.contains("SELECT "));
    }

    #[test]
    fn empty_retrieval_does_not_invent() {
        let suffix = document_qa_suffix("", true);
        assert!(suffix.contains("Do not invent"));
    }

    #[test]
    fn commit_style_toggle() {
        let on = commit_prompt("diff", "feat: foo\nfix: bar", true);
        assert!(on.contains("feat: foo"));
        let off = commit_prompt("diff", "feat: foo", false);
        assert!(off.contains("conventional"));
        assert!(!off.contains("feat: foo"));
    }

    #[test]
    fn inspect_folder_prompt_forbids_inventing() {
        let prompt = inspect_software_folder_prompt(
            "Folder: demo\nREADME.md:\nhello",
            "Rust, TypeScript",
            "React",
        );
        assert!(prompt.contains("Do not invent URLs"));
        assert!(prompt.contains("human product name"));
        assert!(prompt.contains("Allowed languages: Rust, TypeScript"));
        assert!(prompt.contains("hello"));
    }

    #[test]
    fn scan_commands_prompt_forbids_inventing_and_shell() {
        let prompt = scan_commands_prompt("Tree:\nCargo.toml\n", "[{\"command\":\"cargo test\"}]", "[]");
        assert!(prompt.contains("Do not invent scripts"));
        assert!(prompt.contains("no &&"));
        assert!(prompt.contains("hypothesized"));
        assert!(prompt.contains("cargo test"));
        assert!(prompt.contains("Never run commands"));
    }
}

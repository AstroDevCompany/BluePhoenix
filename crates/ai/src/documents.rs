use bluephoenix_domain::ids::CategoryKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentChunk {
    pub id: String,
    pub heading: Option<String>,
    pub page: Option<i64>,
    pub section: Option<String>,
    pub text: String,
}

pub fn chunk_sections(text: &str, max_chars: usize) -> Vec<(Option<String>, String)> {
    let max_chars = max_chars.max(240);
    let mut chunks = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut buffer = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let is_heading = trimmed.starts_with('#')
            || (trimmed.len() < 80 && trimmed.ends_with(':') && !trimmed.contains('.'))
            || looks_like_heading(trimmed);
        if is_heading && !buffer.trim().is_empty() {
            chunks.push((current_heading.clone(), buffer.trim().to_string()));
            buffer.clear();
            current_heading = Some(trimmed.trim_start_matches('#').trim().to_string());
            continue;
        }
        if is_heading {
            current_heading = Some(trimmed.trim_start_matches('#').trim().to_string());
            continue;
        }
        if !trimmed.is_empty() {
            if !buffer.is_empty() {
                buffer.push('\n');
            }
            buffer.push_str(trimmed);
        }
        if buffer.len() >= max_chars {
            chunks.push((current_heading.clone(), buffer.trim().to_string()));
            buffer.clear();
        }
    }
    if !buffer.trim().is_empty() {
        chunks.push((current_heading, buffer.trim().to_string()));
    }
    if chunks.is_empty() && !text.trim().is_empty() {
        for piece in split_chars(text, max_chars) {
            chunks.push((None, piece));
        }
    }
    chunks
}

fn looks_like_heading(line: &str) -> bool {
    if line.is_empty() || line.len() > 90 {
        return false;
    }
    let letters: String = line.chars().filter(|c| c.is_alphabetic()).collect();
    !letters.is_empty() && letters.chars().all(|c| c.is_uppercase())
}

fn split_chars(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        out.push(chars[start..end].iter().collect());
        start = end;
    }
    out
}

pub fn system_prompt(kind: CategoryKind) -> &'static str {
    match kind {
        CategoryKind::Software => {
            "You are BluePhoenix, a local-first software project command center. Reason only from the structured project context provided. Never invent files, APIs, or git history. If information is missing, say so. Prefer concise, practical answers."
        }
        CategoryKind::University => {
            "You are BluePhoenix, helping with a university course. Use only the provided course, topic, exam, attendance, and document excerpts. Do not invent syllabus content. Cite page/section when document excerpts include them."
        }
        CategoryKind::Generic => {
            "You are BluePhoenix, assisting with a personal project. Use only the provided project data. Do not assume it is software or a university course."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_on_headings() {
        let text = "# Intro\nHello world.\n# Next\nMore text that explains things.";
        let chunks = chunk_sections(text, 400);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].0.as_deref(), Some("Intro"));
    }
}

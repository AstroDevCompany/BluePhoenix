use std::io::Read;
use std::path::Path;

use crate::documents::chunk_sections;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDocument {
    pub text: String,
    pub headings: Vec<String>,
    pub tables: Vec<String>,
    pub parser: String,
    pub limitation: Option<String>,
    pub page_count: Option<i64>,
}

pub trait DocumentParser: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, path: &Path, bytes: &[u8]) -> bool;
    fn parse(&self, path: &Path, bytes: &[u8]) -> ParsedDocument;
}

pub struct TxtParser;
pub struct MarkdownParser;
pub struct HtmlParser;
pub struct PdfParser;
pub struct DocxParser;

impl DocumentParser for TxtParser {
    fn name(&self) -> &'static str {
        "txt"
    }
    fn matches(&self, path: &Path, _bytes: &[u8]) -> bool {
        ext(path) == "txt"
    }
    fn parse(&self, _path: &Path, bytes: &[u8]) -> ParsedDocument {
        ParsedDocument {
            text: String::from_utf8_lossy(bytes).to_string(),
            headings: Vec::new(),
            tables: Vec::new(),
            parser: self.name().into(),
            limitation: None,
            page_count: None,
        }
    }
}

impl DocumentParser for MarkdownParser {
    fn name(&self) -> &'static str {
        "markdown"
    }
    fn matches(&self, path: &Path, _bytes: &[u8]) -> bool {
        matches!(ext(path).as_str(), "md" | "markdown")
    }
    fn parse(&self, _path: &Path, bytes: &[u8]) -> ParsedDocument {
        let src = String::from_utf8_lossy(bytes);
        let mut headings = Vec::new();
        let mut tables = Vec::new();
        let parser = pulldown_cmark::Parser::new(&src);
        let mut text = String::new();
        for event in parser {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { .. }) => {}
                pulldown_cmark::Event::Text(t) => {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push(' ');
                    }
                    text.push_str(&t);
                }
                pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => text.push('\n'),
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                    if let Some(line) = text.lines().last() {
                        headings.push(line.trim().to_string());
                    }
                    text.push('\n');
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Paragraph) => text.push('\n'),
                _ => {}
            }
        }
        for line in src.lines() {
            if line.contains('|') && line.matches('|').count() >= 2 {
                tables.push(line.trim().to_string());
            }
            if line.starts_with('#') {
                let h = line.trim_start_matches('#').trim();
                if !h.is_empty() && !headings.iter().any(|x| x == h) {
                    headings.push(h.to_string());
                }
            }
        }
        if text.trim().is_empty() {
            text = src.to_string();
        }
        ParsedDocument {
            text,
            headings,
            tables,
            parser: self.name().into(),
            limitation: None,
            page_count: None,
        }
    }
}

impl DocumentParser for HtmlParser {
    fn name(&self) -> &'static str {
        "html"
    }
    fn matches(&self, path: &Path, bytes: &[u8]) -> bool {
        ext(path) == "html" || ext(path) == "htm" || looks_like_html(bytes)
    }
    fn parse(&self, _path: &Path, bytes: &[u8]) -> ParsedDocument {
        let src = String::from_utf8_lossy(bytes);
        let doc = scraper::Html::parse_document(&src);
        let mut headings = Vec::new();
        if let Ok(sel) = scraper::Selector::parse("h1,h2,h3,h4") {
            for el in doc.select(&sel) {
                let t = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !t.is_empty() {
                    headings.push(t);
                }
            }
        }
        let mut tables = Vec::new();
        if let Ok(sel) = scraper::Selector::parse("table") {
            for el in doc.select(&sel) {
                tables.push(el.text().collect::<Vec<_>>().join(" ").trim().to_string());
            }
        }
        let text = doc.root_element().text().collect::<Vec<_>>().join(" ");
        ParsedDocument {
            text,
            headings,
            tables,
            parser: self.name().into(),
            limitation: None,
            page_count: None,
        }
    }
}

impl DocumentParser for PdfParser {
    fn name(&self) -> &'static str {
        "pdf"
    }
    fn matches(&self, path: &Path, bytes: &[u8]) -> bool {
        ext(path) == "pdf" || bytes.starts_with(b"%PDF")
    }
    fn parse(&self, _path: &Path, bytes: &[u8]) -> ParsedDocument {
        match pdf_extract::extract_text_from_mem(bytes) {
            Ok(text) if !text.trim().is_empty() => ParsedDocument {
                text,
                headings: Vec::new(),
                tables: Vec::new(),
                parser: self.name().into(),
                limitation: None,
                page_count: None,
            },
            Ok(_) => ParsedDocument {
                text: String::new(),
                headings: Vec::new(),
                tables: Vec::new(),
                parser: self.name().into(),
                limitation: Some("PDF contained no extractable text".into()),
                page_count: None,
            },
            Err(_) => ParsedDocument {
                text: String::new(),
                headings: Vec::new(),
                tables: Vec::new(),
                parser: self.name().into(),
                limitation: Some("PDF could not be parsed; original file is still available".into()),
                page_count: None,
            },
        }
    }
}

impl DocumentParser for DocxParser {
    fn name(&self) -> &'static str {
        "docx"
    }
    fn matches(&self, path: &Path, bytes: &[u8]) -> bool {
        ext(path) == "docx" || (bytes.starts_with(b"PK") && ext(path) == "docx")
    }
    fn parse(&self, _path: &Path, bytes: &[u8]) -> ParsedDocument {
        match extract_docx(bytes) {
            Ok(text) => ParsedDocument {
                text,
                headings: Vec::new(),
                tables: Vec::new(),
                parser: self.name().into(),
                limitation: None,
                page_count: None,
            },
            Err(_) => ParsedDocument {
                text: String::new(),
                headings: Vec::new(),
                tables: Vec::new(),
                parser: self.name().into(),
                limitation: Some("DOCX could not be parsed; original file is still available".into()),
                page_count: None,
            },
        }
    }
}

fn extract_docx(bytes: &[u8]) -> Result<String, ()> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|_| ())?;
    let mut file = archive.by_name("word/document.xml").map_err(|_| ())?;
    let mut xml = String::new();
    file.read_to_string(&mut xml).map_err(|_| ())?;
    Ok(strip_xml_text(&xml))
}

fn strip_xml_text(xml: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !out.ends_with('\n') {
                    out.push(' ');
                }
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ext(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn looks_like_html(bytes: &[u8]) -> bool {
    let s = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]).to_ascii_lowercase();
    s.contains("<html") || s.contains("<!doctype html")
}

pub fn parsers() -> Vec<Box<dyn DocumentParser>> {
    vec![
        Box::new(MarkdownParser),
        Box::new(HtmlParser),
        Box::new(PdfParser),
        Box::new(DocxParser),
        Box::new(TxtParser),
    ]
}

pub fn parse_document(path: &Path, bytes: &[u8]) -> ParsedDocument {
    for parser in parsers() {
        if parser.matches(path, bytes) {
            return parser.parse(path, bytes);
        }
    }
    ParsedDocument {
        text: String::new(),
        headings: Vec::new(),
        tables: Vec::new(),
        parser: "unsupported".into(),
        limitation: Some("This file type is not indexed. The original file remains available.".into()),
        page_count: None,
    }
}

pub fn semantic_chunks(parsed: &ParsedDocument, max_chars: usize, max_chunks: usize) -> Vec<(Option<String>, String)> {
    let mut chunks = chunk_sections(&parsed.text, max_chars);
    if chunks.len() > max_chunks {
        chunks.truncate(max_chunks);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn markdown_headings_and_tables() {
        let md = "# Intro\nHello.\n\n## Next\n| A | B |\n| 1 | 2 |\n";
        let parsed = MarkdownParser.parse(&PathBuf::from("n.md"), md.as_bytes());
        assert!(parsed.headings.iter().any(|h| h.contains("Intro")));
        assert!(!parsed.tables.is_empty());
        let chunks = semantic_chunks(&parsed, 400, 20);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn html_extracts_text() {
        let html = "<html><body><h1>Syllabus</h1><p>Week one</p><table><tr><td>A</td></tr></table></body></html>";
        let parsed = HtmlParser.parse(&PathBuf::from("n.html"), html.as_bytes());
        assert!(parsed.text.contains("Week one"));
        assert_eq!(parsed.headings[0], "Syllabus");
        assert!(!parsed.tables.is_empty());
    }

    #[test]
    fn txt_roundtrip() {
        let parsed = TxtParser.parse(&PathBuf::from("n.txt"), b"plain notes");
        assert_eq!(parsed.text, "plain notes");
    }

    #[test]
    fn unsupported_keeps_limitation() {
        let parsed = parse_document(&PathBuf::from("photo.bin"), b"\x00\x01\x02");
        assert!(parsed.limitation.is_some());
        assert!(parsed.text.is_empty());
    }

    #[test]
    fn large_text_is_chunked() {
        let text = "para\n".repeat(400);
        let parsed = TxtParser.parse(&PathBuf::from("big.txt"), text.as_bytes());
        let chunks = semantic_chunks(&parsed, 200, 8);
        assert!(chunks.len() <= 8);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn empty_retrieval_instruction() {
        // Covered by prompts; ensure unanswerable text stays empty.
        assert!(parse_document(&PathBuf::from("x.bin"), b"").text.is_empty());
    }
}

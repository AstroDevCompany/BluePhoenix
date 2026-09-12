use serde::{Deserialize, Serialize};

use crate::ids::CategoryKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Terminology {
    pub workspace: String,
    pub item_singular: String,
    pub item_plural: String,
    pub time_label: String,
    pub timer_start: String,
}

impl Terminology {
    pub fn for_kind(kind: CategoryKind) -> Self {
        match kind {
            CategoryKind::Software => Self {
                workspace: "Software".into(),
                item_singular: "Project".into(),
                item_plural: "Projects".into(),
                time_label: "Development time".into(),
                timer_start: "Start development timer".into(),
            },
            CategoryKind::University => Self {
                workspace: "University".into(),
                item_singular: "Course".into(),
                item_plural: "Courses".into(),
                time_label: "Study time".into(),
                timer_start: "Start study timer".into(),
            },
            CategoryKind::Generic => Self {
                workspace: "Workspace".into(),
                item_singular: "Project".into(),
                item_plural: "Projects".into(),
                time_label: "Time worked".into(),
                timer_start: "Start timer".into(),
            },
        }
    }

    pub fn for_slug(kind: CategoryKind, slug: &str) -> Self {
        let mut terms = Self::for_kind(kind);
        if kind == CategoryKind::Generic {
            match slug {
                "fitness" => {
                    terms.workspace = "Fitness".into();
                    terms.time_label = "Training time".into();
                    terms.timer_start = "Start training timer".into();
                }
                "photography" => {
                    terms.workspace = "Photography".into();
                }
                "personal" => {
                    terms.workspace = "Personal".into();
                }
                _ => {}
            }
        }
        terms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavItem {
    pub id: String,
    pub label: String,
    pub route: String,
    pub capability: Option<String>,
}

pub fn nav_for_kind(kind: CategoryKind) -> Vec<NavItem> {
    match kind {
        CategoryKind::Software => vec![
            item("overview", "Overview", ""),
            item("items", "Projects", ""),
            cap("todos", "Todos", "todos", "todos"),
            cap("commands", "Commands", "commands", "development_commands"),
        ],
        CategoryKind::University => vec![
            item("overview", "Overview", ""),
            item("items", "Courses", ""),
            cap("todos", "Todos", "todos", "todos"),
            cap("exams", "Exams", "exams", "exams"),
        ],
        CategoryKind::Generic => vec![
            item("overview", "Overview", ""),
            item("items", "Projects", ""),
            cap("todos", "Todos", "todos", "todos"),
        ],
    }
}

fn item(id: &str, label: &str, route: &str) -> NavItem {
    NavItem {
        id: id.into(),
        label: label.into(),
        route: route.into(),
        capability: None,
    }
}

fn cap(id: &str, label: &str, route: &str, capability: &str) -> NavItem {
    NavItem {
        id: id.into(),
        label: label.into(),
        route: route.into(),
        capability: Some(capability.into()),
    }
}

pub fn default_todo_kinds(kind: CategoryKind) -> Vec<(&'static str, &'static str)> {
    match kind {
        CategoryKind::Software => vec![
            ("features", "Features"),
            ("fixes", "Fixes"),
            ("ui", "UI"),
            ("performance", "Performance"),
            ("refactoring", "Refactoring"),
            ("documentation", "Documentation"),
            ("security", "Security"),
            ("other", "Other"),
        ],
        CategoryKind::University => vec![
            ("study", "Study"),
            ("exercises", "Exercises"),
            ("revision", "Revision"),
            ("assignments", "Assignments"),
            ("exams", "Exams"),
            ("administration", "Administration"),
            ("other", "Other"),
        ],
        CategoryKind::Generic => vec![
            ("tasks", "Tasks"),
            ("ideas", "Ideas"),
            ("fixes", "Fixes"),
            ("other", "Other"),
        ],
    }
}

pub const LANGUAGES: &[&str] = &[
    "Rust",
    "TypeScript",
    "JavaScript",
    "Python",
    "C",
    "C++",
    "C#",
    "Java",
    "Go",
    "Swift",
    "Kotlin",
    "HTML",
    "CSS",
    "SQL",
];

pub const FRAMEWORKS: &[&str] = &[
    "React",
    "Next.js",
    "Vue",
    "Svelte",
    "Tauri",
    "Electron",
    "React Native",
    "Expo",
    "Node.js",
    "Bun",
    "Deno",
    "Tailwind",
    "Vite",
    ".NET",
    "Unity",
    "Unreal Engine",
    "Docker",
    "PostgreSQL",
    "SQLite",
];

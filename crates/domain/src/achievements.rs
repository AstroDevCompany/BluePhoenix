use serde::{Deserialize, Serialize};

use crate::ids::{CategoryKind, ExamStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl AchievementRarity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Epic => "epic",
            Self::Legendary => "legendary",
        }
    }

    pub fn display_order(self) -> u8 {
        match self {
            Self::Common => 0,
            Self::Uncommon => 1,
            Self::Rare => 2,
            Self::Epic => 3,
            Self::Legendary => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementScope {
    Project,
    User,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AchievementDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub rarity: AchievementRarity,
    pub scope: AchievementScope,
    pub display_order: u32,
    pub kinds: &'static [CategoryKind],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AchievementSnapshot {
    pub kind: Option<CategoryKind>,
    pub tracked_seconds: i64,
    pub study_seconds: i64,
    pub completed_todos: i64,
    pub has_release: bool,
    pub has_version_one: bool,
    pub commit_count: i64,
    pub has_website: bool,
    pub has_github: bool,
    pub project_count: i64,
    pub completed_courses: i64,
    pub passed_exams: i64,
    pub perfect_exam: bool,
    pub attended_lessons: i64,
    pub completed_cfu: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnlockedAchievement {
    pub id: String,
}

const ALL_KINDS: &[CategoryKind] = &[
    CategoryKind::Software,
    CategoryKind::University,
    CategoryKind::Generic,
];
const SOFTWARE: &[CategoryKind] = &[CategoryKind::Software];
const UNIVERSITY: &[CategoryKind] = &[CategoryKind::University];

pub const ACHIEVEMENTS: &[AchievementDefinition] = &[
    AchievementDefinition {
        id: "first-project",
        name: "First Project",
        description: "Create your first project or course.",
        icon: "trophy-first-project.png",
        rarity: AchievementRarity::Common,
        scope: AchievementScope::User,
        display_order: 10,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "hours-1",
        name: "First Hour",
        description: "Spend 1 hour working on this project.",
        icon: "trophy-1-hour.png",
        rarity: AchievementRarity::Common,
        scope: AchievementScope::Project,
        display_order: 20,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "hours-10",
        name: "10 Hours",
        description: "Spend 10 hours working on this project.",
        icon: "trophy-10-hours.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 30,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "hours-25",
        name: "25 Hours",
        description: "Spend 25 hours working on this project.",
        icon: "trophy-25-hours.png",
        rarity: AchievementRarity::Rare,
        scope: AchievementScope::Project,
        display_order: 40,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "hours-50",
        name: "50 Hours",
        description: "Spend 50 hours working on this project.",
        icon: "trophy-50-hours.png",
        rarity: AchievementRarity::Epic,
        scope: AchievementScope::Project,
        display_order: 50,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "hours-100",
        name: "100 Hours",
        description: "Spend 100 hours working on this project.",
        icon: "trophy-100-hours.png",
        rarity: AchievementRarity::Legendary,
        scope: AchievementScope::Project,
        display_order: 60,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "todos-10",
        name: "10 TODOs",
        description: "Complete 10 TODOs on this project.",
        icon: "trophy-10-todos.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 70,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "todos-25",
        name: "25 TODOs",
        description: "Complete 25 TODOs on this project.",
        icon: "trophy-25-todos.png",
        rarity: AchievementRarity::Rare,
        scope: AchievementScope::Project,
        display_order: 80,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "todos-100",
        name: "100 TODOs",
        description: "Complete 100 TODOs on this project.",
        icon: "trophy-100-todos.png",
        rarity: AchievementRarity::Legendary,
        scope: AchievementScope::Project,
        display_order: 90,
        kinds: ALL_KINDS,
    },
    AchievementDefinition {
        id: "first-release",
        name: "First Release",
        description: "Publish the first release of this software project.",
        icon: "trophy-first-release.png",
        rarity: AchievementRarity::Rare,
        scope: AchievementScope::Project,
        display_order: 100,
        kinds: SOFTWARE,
    },
    AchievementDefinition {
        id: "version-1-0",
        name: "Version 1.0",
        description: "Reach version 1.0.0 on this software project.",
        icon: "trophy-first-version.png",
        rarity: AchievementRarity::Epic,
        scope: AchievementScope::Project,
        display_order: 110,
        kinds: SOFTWARE,
    },
    AchievementDefinition {
        id: "commits-100",
        name: "100 Commits",
        description: "Record 100 commits on this software project.",
        icon: "trophy-100-commits.png",
        rarity: AchievementRarity::Epic,
        scope: AchievementScope::Project,
        display_order: 120,
        kinds: SOFTWARE,
    },
    AchievementDefinition {
        id: "first-website",
        name: "First Public Website",
        description: "Connect a public website to this software project.",
        icon: "trophy-first-website.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 130,
        kinds: SOFTWARE,
    },
    AchievementDefinition {
        id: "first-github",
        name: "Source Connected",
        description: "Connect a GitHub repository to this software project.",
        icon: "trophy-first-github.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 140,
        kinds: SOFTWARE,
    },
    AchievementDefinition {
        id: "first-course-completed",
        name: "First Course Completed",
        description: "Mark a university course as completed.",
        icon: "trophy-first-course.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::User,
        display_order: 200,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "first-exam-passed",
        name: "First Exam Passed",
        description: "Pass your first exam.",
        icon: "trophy-first-exam.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 210,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "perfect-exam",
        name: "Excellent Exam",
        description: "Score a perfect exam grade.",
        icon: "trophy-perfect-exam.png",
        rarity: AchievementRarity::Legendary,
        scope: AchievementScope::Project,
        display_order: 220,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "study-hours-10",
        name: "10 Study Hours",
        description: "Study a course for 10 hours.",
        icon: "trophy-10-study-hours.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 230,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "study-hours-100",
        name: "100 Study Hours",
        description: "Study a course for 100 hours.",
        icon: "trophy-100-study-hours.png",
        rarity: AchievementRarity::Legendary,
        scope: AchievementScope::Project,
        display_order: 240,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "courses-10",
        name: "10 Courses",
        description: "Create 10 university courses.",
        icon: "trophy-10-courses.png",
        rarity: AchievementRarity::Rare,
        scope: AchievementScope::User,
        display_order: 250,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "cfu-30",
        name: "30 CFU",
        description: "Complete 30 CFU.",
        icon: "trophy-30-cfu.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::User,
        display_order: 260,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "cfu-60",
        name: "60 CFU",
        description: "Complete 60 CFU.",
        icon: "trophy-60-cfu.png",
        rarity: AchievementRarity::Rare,
        scope: AchievementScope::User,
        display_order: 270,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "cfu-100",
        name: "100 CFU",
        description: "Complete 100 CFU.",
        icon: "trophy-100-cfu.png",
        rarity: AchievementRarity::Epic,
        scope: AchievementScope::User,
        display_order: 280,
        kinds: UNIVERSITY,
    },
    AchievementDefinition {
        id: "lessons-10",
        name: "10 Lessons",
        description: "Attend 10 lessons for this course.",
        icon: "trophy-10-lessons.png",
        rarity: AchievementRarity::Uncommon,
        scope: AchievementScope::Project,
        display_order: 290,
        kinds: UNIVERSITY,
    },
];

pub fn definition(id: &str) -> Option<&'static AchievementDefinition> {
    ACHIEVEMENTS.iter().find(|a| a.id == id)
}

pub fn asset_path(icon: &str) -> String {
    format!("/assets/achievements/{icon}")
}

pub fn evaluate(snapshot: &AchievementSnapshot, already: &[String]) -> Vec<UnlockedAchievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|def| applies_to(def, snapshot.kind))
        .filter(|def| !already.iter().any(|id| id == def.id))
        .filter(|def| condition_met(def.id, snapshot))
        .map(|def| UnlockedAchievement {
            id: def.id.to_string(),
        })
        .collect()
}

fn applies_to(def: &AchievementDefinition, kind: Option<CategoryKind>) -> bool {
    match kind {
        None => true,
        Some(k) => def.kinds.iter().any(|candidate| *candidate == k),
    }
}

fn condition_met(id: &str, s: &AchievementSnapshot) -> bool {
    match id {
        "first-project" => s.project_count >= 1,
        "hours-1" => s.tracked_seconds >= 3600,
        "hours-10" => s.tracked_seconds >= 36_000,
        "hours-25" => s.tracked_seconds >= 90_000,
        "hours-50" => s.tracked_seconds >= 180_000,
        "hours-100" => s.tracked_seconds >= 360_000,
        "todos-10" => s.completed_todos >= 10,
        "todos-25" => s.completed_todos >= 25,
        "todos-100" => s.completed_todos >= 100,
        "first-release" => s.has_release,
        "version-1-0" => s.has_version_one,
        "commits-100" => s.commit_count >= 100,
        "first-website" => s.has_website,
        "first-github" => s.has_github,
        "first-course-completed" => s.completed_courses >= 1,
        "first-exam-passed" => s.passed_exams >= 1,
        "perfect-exam" => s.perfect_exam,
        "study-hours-10" => s.study_seconds >= 36_000,
        "study-hours-100" => s.study_seconds >= 360_000,
        "courses-10" => s.project_count >= 10,
        "cfu-30" => s.completed_cfu >= 30.0,
        "cfu-60" => s.completed_cfu >= 60.0,
        "cfu-100" => s.completed_cfu >= 100.0,
        "lessons-10" => s.attended_lessons >= 10,
        _ => false,
    }
}

pub fn exam_is_perfect(status: ExamStatus, grade: Option<f64>, max: f64) -> bool {
    status == ExamStatus::Passed && grade.map(|g| (g - max).abs() < f64::EPSILON).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlocks_hours_progressively() {
        let snap = AchievementSnapshot {
            kind: Some(CategoryKind::Software),
            tracked_seconds: 36_000,
            ..AchievementSnapshot::default()
        };
        let unlocked = evaluate(&snap, &[]);
        let ids: Vec<_> = unlocked.iter().map(|u| u.id.as_str()).collect();
        assert!(ids.contains(&"hours-1"));
        assert!(ids.contains(&"hours-10"));
        assert!(!ids.contains(&"hours-25"));
        assert!(!ids.contains(&"first-exam-passed"));
    }

    #[test]
    fn university_achievements_hidden_from_software() {
        let snap = AchievementSnapshot {
            kind: Some(CategoryKind::University),
            passed_exams: 1,
            perfect_exam: true,
            ..AchievementSnapshot::default()
        };
        let unlocked = evaluate(&snap, &[]);
        let ids: Vec<_> = unlocked.iter().map(|u| u.id.as_str()).collect();
        assert!(ids.contains(&"first-exam-passed"));
        assert!(ids.contains(&"perfect-exam"));
        assert!(!ids.contains(&"first-release"));
    }

    #[test]
    fn already_unlocked_not_repeated() {
        let snap = AchievementSnapshot {
            tracked_seconds: 360_000,
            kind: Some(CategoryKind::Generic),
            ..AchievementSnapshot::default()
        };
        let unlocked = evaluate(&snap, &["hours-1".into(), "hours-10".into()]);
        let ids: Vec<_> = unlocked.iter().map(|u| u.id.as_str()).collect();
        assert!(!ids.contains(&"hours-1"));
        assert!(ids.contains(&"hours-100"));
    }
}

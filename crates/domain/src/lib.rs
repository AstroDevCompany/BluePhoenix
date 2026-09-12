pub mod achievements;
pub mod attendance;
pub mod capabilities;
pub mod commands_safety;
pub mod context;
pub mod error;
pub mod exams;
pub mod gpa;
pub mod ids;
pub mod lww;
pub mod semver_check;
pub mod terminology;
pub mod time;
pub mod validation;
pub mod xp;

pub use achievements::{
    evaluate as evaluate_achievements, AchievementDefinition, AchievementSnapshot, ACHIEVEMENTS,
};
pub use capabilities::{CapabilityOverrides, CapabilitySet};
pub use error::{DomainError, DomainResult};
pub use ids::{CategoryKind, ProjectStatus, SOFTWARE_CATEGORY_ID, UNIVERSITY_CATEGORY_ID};
pub use terminology::{nav_for_kind, Terminology};
pub use xp::{DefaultXpFormula, XpFormula};

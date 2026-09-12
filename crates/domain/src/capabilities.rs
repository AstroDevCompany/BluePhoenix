use serde::{Deserialize, Serialize};

use crate::ids::CategoryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySet {
    pub git: bool,
    pub github: bool,
    pub development_commands: bool,
    pub languages: bool,
    pub frameworks: bool,
    pub vscode: bool,
    pub terminal: bool,
    pub versions: bool,
    pub files: bool,
    pub links: bool,
    pub todos: bool,
    pub time_tracking: bool,
    pub study_tracking: bool,
    pub attendance: bool,
    pub exams: bool,
    pub grades: bool,
    pub topics: bool,
    pub primary_file: bool,
    pub xp: bool,
    pub achievements: bool,
    pub activity: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityOverrides {
    pub git: Option<bool>,
    pub github: Option<bool>,
    pub development_commands: Option<bool>,
    pub languages: Option<bool>,
    pub frameworks: Option<bool>,
    pub vscode: Option<bool>,
    pub terminal: Option<bool>,
    pub versions: Option<bool>,
    pub files: Option<bool>,
    pub links: Option<bool>,
    pub todos: Option<bool>,
    pub time_tracking: Option<bool>,
    pub study_tracking: Option<bool>,
    pub attendance: Option<bool>,
    pub exams: Option<bool>,
    pub grades: Option<bool>,
    pub topics: Option<bool>,
    pub primary_file: Option<bool>,
    pub xp: Option<bool>,
    pub achievements: Option<bool>,
    pub activity: Option<bool>,
}

impl CapabilitySet {
    pub fn software() -> Self {
        Self {
            git: true,
            github: true,
            development_commands: true,
            languages: true,
            frameworks: true,
            vscode: true,
            terminal: true,
            versions: true,
            files: true,
            links: true,
            todos: true,
            time_tracking: true,
            study_tracking: false,
            attendance: false,
            exams: false,
            grades: false,
            topics: false,
            primary_file: false,
            xp: true,
            achievements: true,
            activity: true,
        }
    }

    pub fn university() -> Self {
        Self {
            git: false,
            github: false,
            development_commands: false,
            languages: false,
            frameworks: false,
            vscode: false,
            terminal: false,
            versions: false,
            files: true,
            links: true,
            todos: true,
            time_tracking: true,
            study_tracking: true,
            attendance: true,
            exams: true,
            grades: true,
            topics: true,
            primary_file: true,
            xp: true,
            achievements: true,
            activity: true,
        }
    }

    pub fn generic() -> Self {
        Self {
            git: false,
            github: false,
            development_commands: false,
            languages: false,
            frameworks: false,
            vscode: false,
            terminal: false,
            versions: false,
            files: true,
            links: true,
            todos: true,
            time_tracking: true,
            study_tracking: false,
            attendance: false,
            exams: false,
            grades: false,
            topics: false,
            primary_file: true,
            xp: true,
            achievements: true,
            activity: true,
        }
    }

    pub fn from_kind(kind: CategoryKind) -> Self {
        match kind {
            CategoryKind::Software => Self::software(),
            CategoryKind::University => Self::university(),
            CategoryKind::Generic => Self::generic(),
        }
    }

    /// Specialized tables (git, exams, …) can never be enabled on a generic kind.
    /// Overrides may only disable universal features, or toggle primary_file.
    pub fn apply_overrides(self, kind: CategoryKind, overrides: &CapabilityOverrides) -> Self {
        let mut set = self;
        let universal_only = kind != CategoryKind::Software && kind != CategoryKind::University;

        apply_flag(&mut set.files, overrides.files);
        apply_flag(&mut set.links, overrides.links);
        apply_flag(&mut set.todos, overrides.todos);
        apply_flag(&mut set.time_tracking, overrides.time_tracking);
        apply_flag(&mut set.primary_file, overrides.primary_file);
        apply_flag(&mut set.xp, overrides.xp);
        apply_flag(&mut set.achievements, overrides.achievements);
        apply_flag(&mut set.activity, overrides.activity);

        if !universal_only {
            apply_flag(&mut set.git, overrides.git);
            apply_flag(&mut set.github, overrides.github);
            apply_flag(&mut set.development_commands, overrides.development_commands);
            apply_flag(&mut set.languages, overrides.languages);
            apply_flag(&mut set.frameworks, overrides.frameworks);
            apply_flag(&mut set.vscode, overrides.vscode);
            apply_flag(&mut set.terminal, overrides.terminal);
            apply_flag(&mut set.versions, overrides.versions);
            apply_flag(&mut set.study_tracking, overrides.study_tracking);
            apply_flag(&mut set.attendance, overrides.attendance);
            apply_flag(&mut set.exams, overrides.exams);
            apply_flag(&mut set.grades, overrides.grades);
            apply_flag(&mut set.topics, overrides.topics);
        } else {
            set.git = false;
            set.github = false;
            set.development_commands = false;
            set.languages = false;
            set.frameworks = false;
            set.vscode = false;
            set.terminal = false;
            set.versions = false;
            set.study_tracking = false;
            set.attendance = false;
            set.exams = false;
            set.grades = false;
            set.topics = false;
        }
        set
    }

    pub fn requires(&self, name: &str) -> Result<(), crate::error::DomainError> {
        let enabled = match name {
            "git" => self.git,
            "github" => self.github,
            "development_commands" => self.development_commands,
            "languages" => self.languages,
            "frameworks" => self.frameworks,
            "vscode" => self.vscode,
            "terminal" => self.terminal,
            "versions" => self.versions,
            "files" => self.files,
            "links" => self.links,
            "todos" => self.todos,
            "time_tracking" => self.time_tracking,
            "study_tracking" => self.study_tracking,
            "attendance" => self.attendance,
            "exams" => self.exams,
            "grades" => self.grades,
            "topics" => self.topics,
            "primary_file" => self.primary_file,
            "xp" => self.xp,
            "achievements" => self.achievements,
            "activity" => self.activity,
            _ => false,
        };
        if enabled {
            Ok(())
        } else {
            Err(crate::error::DomainError::Capability(name.to_string()))
        }
    }
}

fn apply_flag(target: &mut bool, override_value: Option<bool>) {
    if let Some(value) = override_value {
        *target = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_has_git_university_does_not() {
        assert!(CapabilitySet::software().git);
        assert!(!CapabilitySet::university().git);
        assert!(CapabilitySet::university().exams);
        assert!(!CapabilitySet::software().exams);
    }

    #[test]
    fn generic_cannot_enable_exams_via_overrides() {
        let overrides = CapabilityOverrides {
            exams: Some(true),
            git: Some(true),
            time_tracking: Some(false),
            ..CapabilityOverrides::default()
        };
        let set = CapabilitySet::generic().apply_overrides(CategoryKind::Generic, &overrides);
        assert!(!set.exams);
        assert!(!set.git);
        assert!(!set.time_tracking);
    }
}

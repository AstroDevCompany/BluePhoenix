use serde::{Deserialize, Serialize};

use crate::ids::ExamStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExamAttemptView {
    pub status: ExamStatus,
    pub grade: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExamSummary {
    pub attempts: usize,
    pub failed: usize,
    pub passed: usize,
    pub withdrawn: usize,
    pub no_show: usize,
    pub average_grade: Option<f64>,
    pub final_grade: Option<f64>,
}

/// Final grade is an explicit stored value, never max(attempts).
pub fn summarize_exams(attempts: &[ExamAttemptView], final_grade: Option<f64>) -> ExamSummary {
    let mut failed = 0;
    let mut passed = 0;
    let mut withdrawn = 0;
    let mut no_show = 0;
    let mut grades: Vec<f64> = Vec::new();

    for attempt in attempts {
        match attempt.status {
            ExamStatus::Failed => failed += 1,
            ExamStatus::Passed => passed += 1,
            ExamStatus::Withdrawn => withdrawn += 1,
            ExamStatus::NoShow => no_show += 1,
            ExamStatus::Scheduled | ExamStatus::Attempted => {}
        }
        if let Some(grade) = attempt.grade {
            if attempt.status.has_grade() {
                grades.push(grade);
            }
        }
    }

    let average_grade = if grades.is_empty() {
        None
    } else {
        Some(grades.iter().sum::<f64>() / grades.len() as f64)
    };

    ExamSummary {
        attempts: attempts.len(),
        failed,
        passed,
        withdrawn,
        no_show,
        average_grade,
        final_grade,
    }
}

pub fn validate_grade(grade: f64, max: f64) -> Result<(), crate::error::DomainError> {
    if grade < 0.0 || grade > max {
        Err(crate::error::DomainError::Validation(format!(
            "Invalid exam grade: {grade} (max {max})"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_infer_final_from_best_attempt() {
        let attempts = vec![
            ExamAttemptView {
                status: ExamStatus::Failed,
                grade: None,
            },
            ExamAttemptView {
                status: ExamStatus::Passed,
                grade: Some(18.0),
            },
            ExamAttemptView {
                status: ExamStatus::Passed,
                grade: Some(27.0),
            },
        ];
        let summary = summarize_exams(&attempts, Some(27.0));
        assert_eq!(summary.attempts, 3);
        assert_eq!(summary.failed, 1);
        assert!((summary.average_grade.unwrap() - 22.5).abs() < 1e-9);
        assert_eq!(summary.final_grade, Some(27.0));

        let without_final = summarize_exams(&attempts, None);
        assert_eq!(without_final.final_grade, None);
    }

    #[test]
    fn scheduled_attempts_need_no_grade() {
        let attempts = vec![ExamAttemptView {
            status: ExamStatus::Scheduled,
            grade: None,
        }];
        let summary = summarize_exams(&attempts, None);
        assert_eq!(summary.attempts, 1);
        assert!(summary.average_grade.is_none());
    }
}

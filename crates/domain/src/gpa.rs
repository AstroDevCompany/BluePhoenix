use serde::{Deserialize, Serialize};

use crate::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CourseGradeInput {
    pub name: String,
    pub cfu: Option<f64>,
    pub final_grade: Option<f64>,
    pub honors: bool,
    pub included_explicitly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpaConfig {
    pub include_failed: bool,
    pub passing_grade: f64,
    pub max_grade: f64,
}

impl Default for GpaConfig {
    fn default() -> Self {
        Self {
            include_failed: false,
            passing_grade: 18.0,
            max_grade: 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IncludedCourse {
    pub name: String,
    pub cfu: Option<f64>,
    pub final_grade: f64,
    pub honors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpaResult {
    pub simple_average: Option<f64>,
    pub weighted_average: Option<f64>,
    pub used_weighted: bool,
    pub included: Vec<IncludedCourse>,
    pub excluded_missing_final: Vec<String>,
    pub excluded_failed: Vec<String>,
    pub total_cfu: f64,
    pub completed_cfu: f64,
    pub passed_courses: usize,
}

/// GPA uses the explicitly stored final grade, never inferred from exam attempts.
pub fn compute_gpa(courses: &[CourseGradeInput], config: &GpaConfig) -> DomainResult<GpaResult> {
    if config.max_grade <= 0.0 {
        return Err(DomainError::Validation("max grade must be positive".into()));
    }

    let mut included = Vec::new();
    let mut excluded_missing_final = Vec::new();
    let mut excluded_failed = Vec::new();
    let mut total_cfu = 0.0;
    let mut completed_cfu = 0.0;
    let mut passed_courses = 0usize;

    for course in courses {
        if let Some(cfu) = course.cfu {
            if cfu < 0.0 {
                return Err(DomainError::Validation(format!(
                    "CFU must be >= 0 for {}",
                    course.name
                )));
            }
            total_cfu += cfu;
        }

        let Some(grade) = course.final_grade else {
            excluded_missing_final.push(course.name.clone());
            continue;
        };

        if grade < 0.0 || grade > config.max_grade {
            return Err(DomainError::Validation(format!(
                "grade {} is outside 0–{}",
                grade, config.max_grade
            )));
        }

        let passed = grade + f64::EPSILON >= config.passing_grade || course.honors;
        if passed {
            passed_courses += 1;
            if let Some(cfu) = course.cfu {
                completed_cfu += cfu;
            }
        } else if !config.include_failed {
            excluded_failed.push(course.name.clone());
            continue;
        }

        included.push(IncludedCourse {
            name: course.name.clone(),
            cfu: course.cfu,
            final_grade: grade,
            honors: course.honors,
        });
    }

    if included.is_empty() {
        return Ok(GpaResult {
            simple_average: None,
            weighted_average: None,
            used_weighted: false,
            included,
            excluded_missing_final,
            excluded_failed,
            total_cfu,
            completed_cfu,
            passed_courses,
        });
    }

    let simple = included.iter().map(|c| c.final_grade).sum::<f64>() / included.len() as f64;

    let all_have_cfu = included.iter().all(|c| c.cfu.unwrap_or(0.0) > 0.0);
    let weighted = if all_have_cfu {
        let num: f64 = included
            .iter()
            .map(|c| c.final_grade * c.cfu.unwrap_or(0.0))
            .sum();
        let den: f64 = included.iter().map(|c| c.cfu.unwrap_or(0.0)).sum();
        if den > 0.0 {
            Some(num / den)
        } else {
            None
        }
    } else {
        None
    };

    Ok(GpaResult {
        simple_average: Some(simple),
        weighted_average: weighted,
        used_weighted: weighted.is_some(),
        included,
        excluded_missing_final,
        excluded_failed,
        total_cfu,
        completed_cfu,
        passed_courses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_italian_average() {
        let courses = vec![
            CourseGradeInput {
                name: "Calculus".into(),
                cfu: Some(12.0),
                final_grade: Some(27.0),
                honors: false,
                included_explicitly: true,
            },
            CourseGradeInput {
                name: "Geometry".into(),
                cfu: Some(6.0),
                final_grade: Some(18.0),
                honors: false,
                included_explicitly: true,
            },
        ];
        let result = compute_gpa(&courses, &GpaConfig::default()).unwrap();
        let expected = (27.0 * 12.0 + 18.0 * 6.0) / 18.0;
        assert!(result.used_weighted);
        assert!((result.weighted_average.unwrap() - expected).abs() < 1e-9);
        assert_eq!(result.completed_cfu, 18.0);
        assert_eq!(result.passed_courses, 2);
    }

    #[test]
    fn failed_courses_excluded_by_default() {
        let courses = vec![
            CourseGradeInput {
                name: "Algorithms".into(),
                cfu: Some(9.0),
                final_grade: Some(30.0),
                honors: false,
                included_explicitly: true,
            },
            CourseGradeInput {
                name: "Physics".into(),
                cfu: Some(6.0),
                final_grade: Some(12.0),
                honors: false,
                included_explicitly: true,
            },
        ];
        let result = compute_gpa(&courses, &GpaConfig::default()).unwrap();
        assert_eq!(result.included.len(), 1);
        assert_eq!(result.excluded_failed, vec!["Physics".to_string()]);
        assert_eq!(result.weighted_average.unwrap(), 30.0);
    }

    #[test]
    fn missing_final_grade_not_inferred() {
        let courses = vec![CourseGradeInput {
            name: "Calculus".into(),
            cfu: Some(12.0),
            final_grade: None,
            honors: false,
            included_explicitly: true,
        }];
        let result = compute_gpa(&courses, &GpaConfig::default()).unwrap();
        assert!(result.included.is_empty());
        assert_eq!(result.excluded_missing_final, vec!["Calculus".to_string()]);
    }

    #[test]
    fn missing_cfu_falls_back_to_simple_average() {
        let courses = vec![
            CourseGradeInput {
                name: "A".into(),
                cfu: None,
                final_grade: Some(30.0),
                honors: false,
                included_explicitly: true,
            },
            CourseGradeInput {
                name: "B".into(),
                cfu: Some(6.0),
                final_grade: Some(18.0),
                honors: false,
                included_explicitly: true,
            },
        ];
        let result = compute_gpa(&courses, &GpaConfig::default()).unwrap();
        assert!(!result.used_weighted);
        assert!((result.simple_average.unwrap() - 24.0).abs() < 1e-9);
    }
}

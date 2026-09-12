use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LessonView {
    pub attended: bool,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttendanceStats {
    pub lessons: usize,
    pub attended: usize,
    pub missed: usize,
    pub attendance_percent: f64,
    pub attended_seconds: i64,
}

pub fn attendance_stats(lessons: &[LessonView]) -> AttendanceStats {
    let lessons_count = lessons.len();
    let attended = lessons.iter().filter(|l| l.attended).count();
    let missed = lessons_count.saturating_sub(attended);
    let attended_seconds = lessons
        .iter()
        .filter(|l| l.attended)
        .map(|l| l.duration_seconds.max(0))
        .sum();
    let attendance_percent = if lessons_count == 0 {
        0.0
    } else {
        (attended as f64 / lessons_count as f64) * 100.0
    };
    AttendanceStats {
        lessons: lessons_count,
        attended,
        missed,
        attendance_percent,
        attended_seconds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attendance_percentage() {
        let lessons = vec![
            LessonView {
                attended: true,
                duration_seconds: 3600,
            },
            LessonView {
                attended: true,
                duration_seconds: 3600,
            },
            LessonView {
                attended: false,
                duration_seconds: 3600,
            },
        ];
        let stats = attendance_stats(&lessons);
        assert_eq!(stats.lessons, 3);
        assert_eq!(stats.attended, 2);
        assert_eq!(stats.missed, 1);
        assert!((stats.attendance_percent - 66.666).abs() < 0.01);
        assert_eq!(stats.attended_seconds, 7200);
    }
}

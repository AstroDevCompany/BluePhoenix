use serde::{Deserialize, Serialize};

pub trait XpFormula {
    fn xp_for_duration(&self, seconds: i64) -> i64;
    fn level_for_xp(&self, xp: i64) -> LevelProgress;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct LevelProgress {
    pub level: i64,
    pub total_xp: i64,
    pub xp_into_level: i64,
    pub xp_for_level: i64,
}

impl LevelProgress {
    pub fn ratio(&self) -> f64 {
        if self.xp_for_level <= 0 {
            1.0
        } else {
            self.xp_into_level as f64 / self.xp_for_level as f64
        }
    }
}

/// v1 formula: 1 minute = 1 XP.
/// Level n requires `60 * n` XP to clear (1 hour × level).
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultXpFormula;

impl XpFormula for DefaultXpFormula {
    fn xp_for_duration(&self, seconds: i64) -> i64 {
        seconds.max(0) / 60
    }

    fn level_for_xp(&self, xp: i64) -> LevelProgress {
        let total = xp.max(0);
        let mut remaining = total;
        let mut level = 1_i64;
        loop {
            let need = 60 * level;
            if remaining < need {
                return LevelProgress {
                    level,
                    total_xp: total,
                    xp_into_level: remaining,
                    xp_for_level: need,
                };
            }
            remaining -= need;
            level += 1;
            if level > 10_000 {
                return LevelProgress {
                    level,
                    total_xp: total,
                    xp_into_level: 0,
                    xp_for_level: 60 * level,
                };
            }
        }
    }
}

pub fn format_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_minute_is_one_xp() {
        let f = DefaultXpFormula;
        assert_eq!(f.xp_for_duration(59), 0);
        assert_eq!(f.xp_for_duration(60), 1);
        assert_eq!(f.xp_for_duration(3600), 60);
    }

    #[test]
    fn level_curve_uses_increasing_hours() {
        let f = DefaultXpFormula;
        let l1 = f.level_for_xp(0);
        assert_eq!(l1.level, 1);
        assert_eq!(l1.xp_for_level, 60);
        let l2 = f.level_for_xp(60);
        assert_eq!(l2.level, 2);
        let l3 = f.level_for_xp(60 + 120);
        assert_eq!(l3.level, 3);
    }
}

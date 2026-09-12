use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeRange {
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Source of truth is timestamps, never a JS interval counter.
pub fn elapsed_seconds(range: &TimeRange, now: DateTime<Utc>) -> i64 {
    let end = range.ended_at.unwrap_or(now);
    (end - range.started_at).num_seconds().max(0)
}

pub fn clamp_non_negative(seconds: i64) -> i64 {
    seconds.max(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeriodTotals {
    pub today_seconds: i64,
    pub week_seconds: i64,
    pub month_seconds: i64,
    pub total_seconds: i64,
}

pub fn in_period(
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> i64 {
    let end = ended_at.unwrap_or(now);
    let overlap_start = started_at.max(period_start);
    let overlap_end = end.min(period_end);
    (overlap_end - overlap_start).num_seconds().max(0)
}

pub fn overlapping_seconds(
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> i64 {
    in_period(started_at, ended_at, window_start, window_end, now)
}

pub fn start_of_day(now: DateTime<Utc>) -> DateTime<Utc> {
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid hms")
        .and_utc()
}

pub fn add_days(start: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    start + Duration::days(days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn running_timer_uses_now_minus_start() {
        let started = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 14, 0).unwrap();
        let range = TimeRange {
            started_at: started,
            ended_at: None,
        };
        assert_eq!(elapsed_seconds(&range, now), 2 * 3600 + 14 * 60);
    }

    #[test]
    fn negative_clock_skew_clamps() {
        let started = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();
        let range = TimeRange {
            started_at: started,
            ended_at: None,
        };
        assert_eq!(elapsed_seconds(&range, now), 0);
    }

    #[test]
    fn closed_entry_ignores_now() {
        let started = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let ended = Utc.with_ymd_and_hms(2026, 1, 1, 11, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 20, 0, 0).unwrap();
        let range = TimeRange {
            started_at: started,
            ended_at: Some(ended),
        };
        assert_eq!(elapsed_seconds(&range, now), 3600);
    }
}

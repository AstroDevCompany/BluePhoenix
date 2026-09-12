use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LwwDecision {
    AcceptRemote,
    KeepLocal,
}

/// Entity-level last-write-wins. Higher revision wins; equal revision uses
/// `updated_at`, and the remote/server timestamp wins ties.
pub fn decide(
    local_revision: i64,
    local_updated: DateTime<Utc>,
    remote_revision: i64,
    remote_updated: DateTime<Utc>,
) -> LwwDecision {
    if remote_revision > local_revision {
        LwwDecision::AcceptRemote
    } else if remote_revision < local_revision {
        LwwDecision::KeepLocal
    } else if remote_updated >= local_updated {
        LwwDecision::AcceptRemote
    } else {
        LwwDecision::KeepLocal
    }
}

pub fn push_is_fresh(server_revision: i64, client_base_revision: i64) -> bool {
    client_base_revision == server_revision
}

pub fn validate_revision(revision: i64) -> DomainResult<()> {
    if revision < 1 {
        Err(DomainError::Validation("revision must be >= 1".into()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn higher_remote_revision_wins() {
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(decide(1, t, 2, t), LwwDecision::AcceptRemote);
        assert_eq!(decide(3, t, 2, t), LwwDecision::KeepLocal);
    }

    #[test]
    fn equal_revision_server_timestamp_wins() {
        let earlier = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap();
        assert_eq!(decide(4, earlier, 4, later), LwwDecision::AcceptRemote);
        assert_eq!(decide(4, later, 4, earlier), LwwDecision::KeepLocal);
        assert_eq!(decide(4, later, 4, later), LwwDecision::AcceptRemote);
    }

    #[test]
    fn stale_push_rejected() {
        assert!(push_is_fresh(3, 3));
        assert!(!push_is_fresh(4, 3));
    }
}

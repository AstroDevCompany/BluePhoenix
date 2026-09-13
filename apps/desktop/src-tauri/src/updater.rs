use crate::models::UpdateCheckDto;
use bluephoenix_domain::semver_check::{is_newer, parse_remote_version};

pub const VERSION_CHECK_URL: &str =
    "https://raw.githubusercontent.com/AstroDevCompany/mint-update-links/refs/heads/main/bluephoenix.v";

pub async fn check_raw(url: &str, local: &str) -> UpdateCheckDto {
    if url.trim().is_empty() {
        return UpdateCheckDto {
            local: local.to_string(),
            remote: None,
            newer: false,
            error: Some("Update check is not configured".into()),
        };
    }
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            return UpdateCheckDto {
                local: local.to_string(),
                remote: None,
                newer: false,
                error: Some(err.to_string()),
            };
        }
    };
    match client.get(url).send().await {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return UpdateCheckDto {
                    local: local.to_string(),
                    remote: None,
                    newer: false,
                    error: Some("Rate limited while checking for updates".into()),
                };
            }
            if !response.status().is_success() {
                return UpdateCheckDto {
                    local: local.to_string(),
                    remote: None,
                    newer: false,
                    error: Some(format!("Update check failed ({})", response.status())),
                };
            }
            match response.text().await {
                Ok(body) => match parse_remote_version(&body) {
                    Ok(remote) => {
                        let newer = is_newer(&remote, local).unwrap_or(false);
                        UpdateCheckDto {
                            local: local.to_string(),
                            remote: Some(remote),
                            newer,
                            error: None,
                        }
                    }
                    Err(err) => UpdateCheckDto {
                        local: local.to_string(),
                        remote: None,
                        newer: false,
                        error: Some(err.to_string()),
                    },
                },
                Err(_) => UpdateCheckDto {
                    local: local.to_string(),
                    remote: None,
                    newer: false,
                    error: Some("Network unavailable".into()),
                },
            }
        }
        Err(_) => UpdateCheckDto {
            local: local.to_string(),
            remote: None,
            newer: false,
            error: Some("Network unavailable".into()),
        },
    }
}

pub fn local_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

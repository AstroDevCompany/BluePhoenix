use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::SyncStatusDto;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::json;

const SERVICE: &str = "BluePhoenix";

#[derive(Serialize)]
struct AuthBody<'a> {
    email: &'a str,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none", rename = "displayName")]
    display_name: Option<&'a str>,
}

#[derive(Deserialize)]
struct TokenRes {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    user: Option<serde_json::Value>,
}

fn keyring_set(key: &str, value: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE, key).map_err(|e| AppError::msg(e.to_string()))?;
    entry
        .set_password(value)
        .map_err(|e| AppError::msg(e.to_string()))
}

fn keyring_get(key: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, key)
        .ok()
        .and_then(|e| e.get_password().ok())
}

fn keyring_delete(key: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
        let _ = entry.delete_credential();
    }
}

fn api_base(state: &AppState) -> String {
    state
        .db
        .with(|c| Ok(db::load_settings(c).api_base))
        .unwrap_or_else(|_| "http://127.0.0.1:8787".into())
}

pub async fn register(
    state: &AppState,
    email: String,
    password: String,
    display_name: Option<String>,
) -> AppResult<serde_json::Value> {
    let base = api_base(state);
    let res = state
        .http
        .post(format!("{base}/v1/auth/register"))
        .json(&AuthBody {
            email: &email,
            password: &password,
            display_name: display_name.as_deref(),
        })
        .send()
        .await
        .map_err(|_| AppError::Network)?;
    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(AppError::msg(if text.is_empty() {
            "Unable to create account".into()
        } else {
            text
        }));
    }
    let tokens: TokenRes = res.json().await.map_err(|_| AppError::Sync)?;
    persist_tokens(&tokens)?;
    claim_local(state, &tokens.access_token).await?;
    let _ = crate::ai::try_unwrap_after_auth(state, &email, &password);
    Ok(json!({"ok": true, "user": tokens.user}))
}

pub async fn login(state: &AppState, email: String, password: String) -> AppResult<serde_json::Value> {
    let base = api_base(state);
    let res = state
        .http
        .post(format!("{base}/v1/auth/login"))
        .json(&AuthBody {
            email: &email,
            password: &password,
            display_name: None,
        })
        .send()
        .await
        .map_err(|_| AppError::Network)?;
    if !res.status().is_success() {
        return Err(AppError::msg("Invalid email or password"));
    }
    let tokens: TokenRes = res.json().await.map_err(|_| AppError::Sync)?;
    persist_tokens(&tokens)?;
    claim_local(state, &tokens.access_token).await?;
    let _ = crate::ai::try_unwrap_after_auth(state, &email, &password);
    Ok(json!({"ok": true, "user": tokens.user}))
}

pub fn logout(state: &AppState) -> AppResult<()> {
    let _ = state;
    keyring_delete("access_token");
    keyring_delete("refresh_token");
    Ok(())
}

pub async fn forgot(state: &AppState, email: String) -> AppResult<()> {
    let base = api_base(state);
    let _ = state
        .http
        .post(format!("{base}/v1/auth/forgot"))
        .json(&json!({ "email": email }))
        .send()
        .await
        .map_err(|_| AppError::Network)?;
    Ok(())
}

fn persist_tokens(tokens: &TokenRes) -> AppResult<()> {
    keyring_set("access_token", &tokens.access_token)?;
    keyring_set("refresh_token", &tokens.refresh_token)?;
    Ok(())
}

async fn claim_local(state: &AppState, access: &str) -> AppResult<()> {
    let base = api_base(state);
    let _ = state
        .http
        .post(format!("{base}/v1/account/claim-local"))
        .bearer_auth(access)
        .json(&json!({ "deviceId": state.db.device_id }))
        .send()
        .await;
    Ok(())
}

pub async fn push_and_pull(state: &AppState) -> AppResult<SyncStatusDto> {
    let Some(token) = keyring_get("access_token") else {
        return state.db.with(|c| {
            let mut status = db::sync_status(c);
            if status.pending == 0 {
                status.status = "offline".into();
                status.label = "Offline".into();
            }
            Ok(status)
        });
    };
    let base = api_base(state);
    let device_id = state.db.device_id.clone();

    let batch = state.db.with(|c| db::pending_outbox(c))?;
    if !batch.is_empty() {
        let changes: Vec<serde_json::Value> = batch
            .iter()
            .map(|(_, table, row_id, payload)| {
                json!({
                    "table": table,
                    "rowId": row_id,
                    "op": "upsert",
                    "payload": serde_json::from_str::<serde_json::Value>(payload).unwrap_or(json!({})),
                    "revision": 1,
                    "baseRevision": 1,
                })
            })
            .collect();
        let res = state
            .http
            .post(format!("{base}/v1/sync/push"))
            .bearer_auth(&token)
            .json(&json!({ "deviceId": device_id, "changes": changes }))
            .send()
            .await
            .map_err(|_| AppError::Network)?;
        if res.status().is_success() {
            let ids: Vec<String> = batch.into_iter().map(|b| b.0).collect();
            state.db.with(|c| db::ack_outbox(c, &ids))?;
        } else {
            state.db.with(|c| db::set_sync_cursor(c, "metadata", None, Some("push failed")))?;
            return Err(AppError::Sync);
        }
    }

    let secrets = state.db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, payload FROM secret_sync_outbox ORDER BY created_at LIMIT 50",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        Ok(rows)
    })?;
    if !secrets.is_empty() {
        let envelopes: Vec<serde_json::Value> = secrets
            .iter()
            .filter_map(|(_, p)| serde_json::from_str(p).ok())
            .collect();
        let res = state
            .http
            .post(format!("{base}/v1/secrets/push"))
            .bearer_auth(&token)
            .json(&json!({ "deviceId": device_id, "secrets": envelopes }))
            .send()
            .await;
        if let Ok(res) = res {
            if res.status().is_success() {
                let ids: Vec<String> = secrets.into_iter().map(|s| s.0).collect();
                state.db.with(|c| {
                    for id in ids {
                        c.execute("DELETE FROM secret_sync_outbox WHERE id=?1", [id])?;
                    }
                    Ok(())
                })?;
            }
        }
    }

    let cursor = state
        .db
        .with(|c| Ok(db::sync_cursor(c, "metadata").unwrap_or_else(|| "1970-01-01T00:00:00Z".into())))?;
    if let Ok(res) = state
        .http
        .get(format!("{base}/v1/sync/pull"))
        .bearer_auth(&token)
        .query(&[("cursor", cursor.as_str())])
        .send()
        .await
    {
        if res.status().is_success() {
            if let Ok(body) = res.json::<serde_json::Value>().await {
                let changes = body
                    .get("changes")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                state.db.with(|c| db::apply_pull_changes(c, &changes))?;
                let next = body
                    .get("cursor")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&cursor);
                state.db.with(|c| db::set_sync_cursor(c, "metadata", Some(next), None))?;
            }
        }
    }

    if let Ok(res) = state
        .http
        .get(format!("{base}/v1/secrets/pull"))
        .bearer_auth(&token)
        .send()
        .await
    {
        if res.status().is_success() {
            if let Ok(body) = res.json::<serde_json::Value>().await {
                let secrets = body
                    .get("secrets")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                state.db.with(|c| db::apply_secret_envelopes(c, &secrets))?;
                state.db.with(|c| db::set_sync_cursor(c, "secrets", Some("now"), None))?;
                let _ = crate::ai::try_unwrap_with_stored_key(state);
            }
        }
    }

    state.db.with(|c| Ok(db::sync_status(c)))
}

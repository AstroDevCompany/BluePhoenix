use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Query, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    jwt_secret: String,
    invalidate: broadcast::Sender<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Deserialize)]
struct AuthReq {
    email: String,
    password: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct ForgotReq {
    email: String,
}

#[derive(Deserialize)]
struct ResetReq {
    token: String,
    password: String,
}

#[derive(Deserialize)]
struct RefreshReq {
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

#[derive(Deserialize)]
struct PushBody {
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
    changes: Vec<Change>,
}

#[derive(Deserialize)]
struct Change {
    table: String,
    #[serde(rename = "rowId")]
    row_id: String,
    op: String,
    payload: Value,
    revision: Option<i64>,
    #[serde(rename = "baseRevision")]
    base_revision: Option<i64>,
}

#[derive(Deserialize)]
struct SecretPush {
    secrets: Vec<SecretEnvelope>,
}

#[derive(Deserialize, Serialize, Clone)]
struct SecretEnvelope {
    id: String,
    kind: String,
    ciphertext: String,
    nonce: String,
    #[serde(rename = "wrapParams", default)]
    wrap_params: Value,
    revision: Option<i64>,
}

#[derive(Deserialize)]
struct PullQuery {
    cursor: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("bluephoenix_api=info,tower_http=info")
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://bluephoenix:bluephoenix@127.0.0.1:5432/bluephoenix".into()
    });
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-only-change-me".into());

    let pool = PgPool::connect(&database_url).await.unwrap_or_else(|err| {
        tracing::error!("database connection failed: {err}");
        std::process::exit(1);
    });
    if let Err(err) = sqlx::raw_sql(include_str!("../migrations/001_init.sql"))
        .execute(&pool)
        .await
    {
        tracing::warn!("migration: {err}");
    }

    let (invalidate, _) = broadcast::channel::<(String, String)>(256);
    spawn_pg_listener(database_url.clone(), invalidate.clone());

    let state = Arc::new(AppState {
        pool,
        jwt_secret,
        invalidate,
    });
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/auth/forgot", post(forgot))
        .route("/v1/auth/reset", post(reset))
        .route("/v1/me", get(me))
        .route("/v1/account/claim-local", post(claim_local))
        .route("/v1/sync/push", post(sync_push))
        .route("/v1/sync/pull", get(sync_pull))
        .route("/v1/sync/status", get(sync_status))
        .route("/v1/sync/ws", get(sync_ws))
        .route("/v1/secrets/push", post(secrets_push))
        .route("/v1/secrets/pull", get(secrets_pull))
        .route("/v1/openapi.json", get(openapi))
        .with_state(state)
        .layer(cors);

    let addr = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    tracing::info!("BluePhoenix API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

fn hash_password(password: &str) -> Result<String, StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .and_then(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).ok())
        .is_some()
}

fn sha(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn token(secret: &str, user_id: &str, hours: i64) -> String {
    let exp = (Utc::now() + Duration::hours(hours)).timestamp() as usize;
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.into(),
            exp,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap_or_default()
}

fn user_from_header(headers: &HeaderMap, secret: &str) -> Result<Uuid, StatusCode> {
    let header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let raw = header.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?;
    let data = decode::<Claims>(
        raw,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Uuid::parse_str(&data.claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthReq>,
) -> Result<impl IntoResponse, StatusCode> {
    if body.email.trim().is_empty() || body.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let id = Uuid::now_v7();
    let hash = hash_password(&body.password)?;
    sqlx::query("INSERT INTO users (id, email, password_hash, display_name) VALUES ($1,$2,$3,$4)")
        .bind(id)
        .bind(body.email.trim().to_lowercase())
        .bind(hash)
        .bind(body.display_name.clone())
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    Ok(Json(issue_tokens(&state, id, &body.email, body.display_name.as_deref()).await?))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let row = sqlx::query("SELECT id, password_hash, display_name, email FROM users WHERE email=$1 AND deleted_at IS NULL")
        .bind(body.email.trim().to_lowercase())
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let hash: String = row.get("password_hash");
    if !verify_password(&body.password, &hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let id: Uuid = row.get("id");
    let email: String = row.get("email");
    let name: Option<String> = row.get("display_name");
    Ok(Json(issue_tokens(&state, id, &email, name.as_deref()).await?))
}

async fn issue_tokens(
    state: &AppState,
    id: Uuid,
    email: &str,
    name: Option<&str>,
) -> Result<Value, StatusCode> {
    let access = token(&state.jwt_secret, &id.to_string(), 1);
    let refresh_raw = format!("{}{}", Uuid::now_v7(), Uuid::now_v7());
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::now_v7())
    .bind(id)
    .bind(sha(&refresh_raw))
    .bind(Utc::now() + Duration::days(30))
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(json!({
        "accessToken": access,
        "refreshToken": refresh_raw,
        "user": { "id": id, "email": email, "displayName": name }
    }))
}

async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let hash = sha(&body.refresh_token);
    let row = sqlx::query(
        "SELECT id, user_id FROM refresh_tokens WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(&hash)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let token_id: Uuid = row.get("id");
    let user_id: Uuid = row.get("user_id");
    sqlx::query("UPDATE refresh_tokens SET revoked_at=now() WHERE id=$1")
        .bind(token_id)
        .execute(&state.pool)
        .await
        .ok();
    let user = sqlx::query("SELECT email, display_name FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let email: String = user.get("email");
    let name: Option<String> = user.get("display_name");
    Ok(Json(issue_tokens(&state, user_id, &email, name.as_deref()).await?))
}

async fn logout() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

async fn forgot(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ForgotReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let row = sqlx::query("SELECT id FROM users WHERE email=$1")
        .bind(body.email.trim().to_lowercase())
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(row) = row {
        let user_id: Uuid = row.get("id");
        let raw = Uuid::now_v7().to_string();
        sqlx::query("INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::now_v7())
            .bind(user_id)
            .bind(sha(&raw))
            .bind(Utc::now() + Duration::hours(2))
            .execute(&state.pool)
            .await
            .ok();
        tracing::info!("password reset token for {} (dev): {raw}", body.email);
    }
    Ok(Json(json!({ "ok": true })))
}

async fn reset(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResetReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let hash = sha(&body.token);
    let row = sqlx::query("SELECT user_id FROM password_reset_tokens WHERE token_hash=$1 AND used_at IS NULL AND expires_at > now()")
        .bind(hash)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let user_id: Uuid = row.get("user_id");
    let password_hash = hash_password(&body.password)?;
    sqlx::query("UPDATE users SET password_hash=$1, updated_at=now() WHERE id=$2")
        .bind(password_hash)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("UPDATE password_reset_tokens SET used_at=now() WHERE token_hash=$1")
        .bind(sha(&body.token))
        .execute(&state.pool)
        .await
        .ok();
    Ok(Json(json!({ "ok": true })))
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    let row = sqlx::query("SELECT id, email, display_name FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(json!({
        "id": row.get::<Uuid, _>("id"),
        "email": row.get::<String, _>("email"),
        "displayName": row.get::<Option<String>, _>("display_name")
    })))
}

async fn claim_local(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    if let Some(device) = body.get("deviceId").and_then(|v| v.as_str()) {
        if let Ok(id) = Uuid::parse_str(device) {
            sqlx::query("INSERT INTO devices (id, user_id) VALUES ($1,$2) ON CONFLICT (id) DO NOTHING")
                .bind(id)
                .bind(user_id)
                .execute(&state.pool)
                .await
                .ok();
        }
    }
    Ok(Json(json!({ "ok": true, "userId": user_id })))
}

async fn sync_push(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PushBody>,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    let _ = body.device_id;
    let mut accepted = Vec::new();
    let mut conflicts = Vec::new();
    for change in body.changes {
        let Ok(row_id) = Uuid::parse_str(&change.row_id) else {
            continue;
        };
        let existing = sqlx::query("SELECT revision FROM sync_rows WHERE user_id=$1 AND table_name=$2 AND row_id=$3")
            .bind(user_id)
            .bind(&change.table)
            .bind(row_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(row) = existing {
            let server_rev: i64 = row.get("revision");
            let base = change.base_revision.unwrap_or(0);
            if base != server_rev {
                conflicts.push(json!({
                    "table": change.table,
                    "rowId": change.row_id,
                    "reason": "stale_revision",
                    "serverRevision": server_rev
                }));
                continue;
            }
        }
        let next_rev = change.revision.unwrap_or(1).max(1);
        let deleted = if change.op == "delete" { Some(Utc::now()) } else { None };
        sqlx::query(
            "INSERT INTO sync_rows (user_id, table_name, row_id, payload, revision, updated_at, deleted_at)
             VALUES ($1,$2,$3,$4,$5,now(),$6)
             ON CONFLICT (user_id, table_name, row_id) DO UPDATE SET payload=excluded.payload, revision=excluded.revision, updated_at=now(), deleted_at=excluded.deleted_at",
        )
        .bind(user_id)
        .bind(&change.table)
        .bind(row_id)
        .bind(change.payload)
        .bind(next_rev)
        .bind(deleted)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        accepted.push(change.row_id);
        let _ = state
            .invalidate
            .send(("user_changes".into(), user_id.to_string()));
    }
    Ok(Json(json!({ "accepted": accepted, "conflicts": conflicts })))
}

async fn sync_pull(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<PullQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    let cursor = query
        .cursor
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|| chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now));
    let rows = sqlx::query("SELECT table_name, row_id, payload, revision, updated_at, deleted_at FROM sync_rows WHERE user_id=$1 AND updated_at > $2 ORDER BY updated_at")
        .bind(user_id)
        .bind(cursor)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let changes: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "table": r.get::<String, _>("table_name"),
                "rowId": r.get::<Uuid, _>("row_id"),
                "payload": r.get::<Value, _>("payload"),
                "revision": r.get::<i64, _>("revision"),
                "updatedAt": r.get::<chrono::DateTime<Utc>, _>("updated_at"),
                "deletedAt": r.get::<Option<chrono::DateTime<Utc>>, _>("deleted_at"),
            })
        })
        .collect();
    let new_cursor = Utc::now().to_rfc3339();
    Ok(Json(json!({ "cursor": new_cursor, "changes": changes })))
}

async fn sync_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_rows WHERE user_id=$1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    Ok(Json(json!({ "rows": count })))
}

async fn secrets_push(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SecretPush>,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    for secret in body.secrets {
        let Ok(id) = Uuid::parse_str(&secret.id) else {
            continue;
        };
        sqlx::query(
            "INSERT INTO encrypted_secrets (id, user_id, kind, ciphertext, nonce, wrap_params, revision, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,now())
             ON CONFLICT (id) DO UPDATE SET ciphertext=excluded.ciphertext, nonce=excluded.nonce, wrap_params=excluded.wrap_params, revision=excluded.revision, updated_at=now()",
        )
        .bind(id)
        .bind(user_id)
        .bind(&secret.kind)
        .bind(&secret.ciphertext)
        .bind(&secret.nonce)
        .bind(secret.wrap_params.clone())
        .bind(secret.revision.unwrap_or(1))
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let _ = state
        .invalidate
        .send(("user_secret_changes".into(), user_id.to_string()));
    Ok(Json(json!({ "ok": true })))
}

async fn secrets_pull(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    let rows = sqlx::query("SELECT id, kind, ciphertext, nonce, wrap_params, revision, updated_at, deleted_at FROM encrypted_secrets WHERE user_id=$1")
        .bind(user_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let secrets: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<Uuid, _>("id"),
                "kind": r.get::<String, _>("kind"),
                "ciphertext": r.get::<String, _>("ciphertext"),
                "nonce": r.get::<String, _>("nonce"),
                "wrapParams": r.get::<Value, _>("wrap_params"),
                "revision": r.get::<i64, _>("revision"),
                "updatedAt": r.get::<chrono::DateTime<Utc>, _>("updated_at"),
                "deletedAt": r.get::<Option<chrono::DateTime<Utc>>, _>("deleted_at"),
            })
        })
        .collect();
    Ok(Json(json!({ "secrets": secrets })))
}

async fn sync_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = user_from_header(&headers, &state.jwt_secret)?;
    let rx = state.invalidate.subscribe();
    Ok(ws.on_upgrade(move |socket| socket_loop(socket, user_id, rx)))
}

async fn socket_loop(
    mut socket: WebSocket,
    user_id: Uuid,
    mut rx: broadcast::Receiver<(String, String)>,
) {
    let _ = socket
        .send(Message::Text(
            json!({"kind":"hello","userId": user_id}).to_string().into(),
        ))
        .await;
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok((channel, payload)) if payload == user_id.to_string() => {
                        let kind = if channel == "user_secret_changes" { "secrets" } else { "metadata" };
                        if socket
                            .send(Message::Text(json!({"kind": kind}).to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

fn spawn_pg_listener(database_url: String, tx: broadcast::Sender<(String, String)>) {
    tokio::spawn(async move {
        let mut listener = match sqlx::postgres::PgListener::connect(&database_url).await {
            Ok(l) => l,
            Err(err) => {
                tracing::warn!("LISTEN unavailable: {err}");
                return;
            }
        };
        let _ = listener.listen("user_changes").await;
        let _ = listener.listen("user_secret_changes").await;
        loop {
            match listener.recv().await {
                Ok(notification) => {
                    let _ = tx.send((
                        notification.channel().to_string(),
                        notification.payload().to_string(),
                    ));
                }
                Err(err) => {
                    tracing::warn!("NOTIFY recv: {err}");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    });
}

async fn openapi() -> impl IntoResponse {
    Json(json!({
        "openapi": "3.0.3",
        "info": { "title": "BluePhoenix API", "version": "0.1.0" },
        "paths": {
            "/v1/auth/register": { "post": { "summary": "Register" } },
            "/v1/auth/login": { "post": { "summary": "Login" } },
            "/v1/sync/push": { "post": { "summary": "Push metadata" } },
            "/v1/sync/pull": { "get": { "summary": "Pull metadata" } },
            "/v1/secrets/push": { "post": { "summary": "Push encrypted secret envelopes" } },
            "/v1/secrets/pull": { "get": { "summary": "Pull encrypted secret envelopes" } }
        }
    }))
}

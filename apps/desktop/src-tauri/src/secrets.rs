use crate::error::{AppError, AppResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const SERVICE: &str = "BluePhoenix";
pub const OPENROUTER_KEY: &str = "openrouter_api_key";
const WRAP_STATE: &str = "secret_wrap_state";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapParams {
    pub alg: String,
    pub m: u32,
    pub t: u32,
    pub p: u32,
    pub salt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WrapState {
    key: String,
    params: WrapParams,
}

pub fn keyring_set(key: &str, value: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE, key).map_err(|e| AppError::msg(e.to_string()))?;
    entry
        .set_password(value)
        .map_err(|e| AppError::msg(e.to_string()))
}

pub fn keyring_get(key: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, key)
        .ok()
        .and_then(|e| e.get_password().ok())
}

pub fn keyring_delete(key: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
        let _ = entry.delete_credential();
    }
}

pub fn has_openrouter_key() -> bool {
    keyring_get(OPENROUTER_KEY).map(|s| !s.trim().is_empty()).unwrap_or(false)
}

pub fn openrouter_key() -> Option<String> {
    keyring_get(OPENROUTER_KEY).filter(|s| !s.trim().is_empty())
}

pub fn mask_key(key: &str) -> String {
    bluephoenix_ai::redact(key)
}

pub fn on_login(email: &str, password: &str) -> AppResult<()> {
    let params = existing_wrap_params().unwrap_or_else(new_wrap_params);
    let key = derive_wrap_key(password, email, &params)?;
    store_wrap_state(&key, &params)?;
    Ok(())
}

pub fn wrap_key_available() -> bool {
    keyring_get(WRAP_STATE).is_some()
}

fn existing_wrap_params() -> Option<WrapParams> {
    let raw = keyring_get(WRAP_STATE)?;
    serde_json::from_str::<WrapState>(&raw).ok().map(|s| s.params)
}

fn new_wrap_params() -> WrapParams {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    WrapParams {
        alg: "argon2id".into(),
        m: 19456,
        t: 2,
        p: 1,
        salt: B64.encode(salt),
    }
}

fn store_wrap_state(key: &[u8; 32], params: &WrapParams) -> AppResult<()> {
    let state = WrapState {
        key: hex::encode(key),
        params: params.clone(),
    };
    keyring_set(WRAP_STATE, &serde_json::to_string(&state).unwrap_or_default())
}

fn load_wrap_key() -> AppResult<[u8; 32]> {
    let raw = keyring_get(WRAP_STATE).ok_or_else(|| AppError::msg("Not signed in for secret sync"))?;
    let state: WrapState = serde_json::from_str(&raw).map_err(|_| AppError::msg("Wrap state is invalid"))?;
    let bytes = hex::decode(state.key).map_err(|_| AppError::msg("Wrap state is invalid"))?;
    let mut key = [0u8; 32];
    if bytes.len() != 32 {
        return Err(AppError::msg("Wrap state is invalid"));
    }
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub fn derive_wrap_key(password: &str, email: &str, params: &WrapParams) -> AppResult<[u8; 32]> {
    let salt = B64.decode(&params.salt).map_err(|_| AppError::msg("Invalid wrap salt"))?;
    if salt.len() < 8 {
        return Err(AppError::msg("Invalid wrap salt"));
    }
    let argon_params = Params::new(params.m, params.t, params.p, Some(32)).map_err(|e| AppError::msg(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut out = [0u8; 32];
    let material = format!("{password}{}", email.trim().to_lowercase());
    argon2
        .hash_password_into(material.as_bytes(), &salt, &mut out)
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(out)
}

pub fn encrypt_secret(plaintext: &str) -> AppResult<(String, String, serde_json::Value)> {
    let key = load_wrap_key()?;
    let params = existing_wrap_params().ok_or_else(|| AppError::msg("Wrap parameters missing"))?;
    encrypt_with_key(plaintext.as_bytes(), &key, &params)
}

pub fn decrypt_secret(ciphertext_b64: &str, nonce_b64: &str, wrap_params: &serde_json::Value, password: &str, email: &str) -> AppResult<String> {
    let params: WrapParams = serde_json::from_value(wrap_params.clone()).map_err(|_| AppError::msg("Invalid wrap params"))?;
    let key = derive_wrap_key(password, email, &params)?;
    decrypt_with_key(ciphertext_b64, nonce_b64, &key)
}

pub fn decrypt_with_stored_key(ciphertext_b64: &str, nonce_b64: &str) -> AppResult<String> {
    let key = load_wrap_key()?;
    decrypt_with_key(ciphertext_b64, nonce_b64, &key)
}

fn encrypt_with_key(plaintext: &[u8], key: &[u8; 32], params: &WrapParams) -> AppResult<(String, String, serde_json::Value)> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AppError::msg(e.to_string()))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| AppError::msg("Unable to wrap secret"))?;
    Ok((B64.encode(ct), B64.encode(nonce_bytes), json!(params)))
}

fn decrypt_with_key(ciphertext_b64: &str, nonce_b64: &str, key: &[u8; 32]) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AppError::msg(e.to_string()))?;
    let ct = B64.decode(ciphertext_b64).map_err(|_| AppError::msg("Invalid ciphertext"))?;
    let nonce_bytes = B64.decode(nonce_b64).map_err(|_| AppError::msg("Invalid nonce"))?;
    if nonce_bytes.len() != 12 {
        return Err(AppError::msg("Invalid nonce"));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let pt = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| AppError::msg("Unable to unwrap secret"))?;
    String::from_utf8(pt).map_err(|_| AppError::msg("Secret was not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_gcm_roundtrip_with_argon2() {
        let params = WrapParams {
            alg: "argon2id".into(),
            m: 32,
            t: 1,
            p: 1,
            salt: B64.encode(b"testsalt-16bytes"),
        };
        let key = derive_wrap_key("pw", "user@example.com", &params).unwrap();
        let (ct, nonce, stored) = encrypt_with_key(b"sk-or-test", &key, &params).unwrap();
        let again = derive_wrap_key("pw", "user@example.com", &params).unwrap();
        let pt = decrypt_with_key(&ct, &nonce, &again).unwrap();
        assert_eq!(pt, "sk-or-test");
        assert_eq!(stored["alg"], "argon2id");
        let wrong = derive_wrap_key("nope", "user@example.com", &params).unwrap();
        assert!(decrypt_with_key(&ct, &nonce, &wrong).is_err());
    }
}

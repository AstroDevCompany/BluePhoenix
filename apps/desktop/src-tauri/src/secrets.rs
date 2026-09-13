use crate::error::{AppError, AppResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const OPENROUTER_KEY: &str = "openrouter_api_key";
const WRAP_STATE: &str = "secret_wrap_state";

fn memory() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn vault_dir() -> &'static Mutex<Option<PathBuf>> {
    static DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    DIR.get_or_init(|| Mutex::new(None))
}

fn remember(key: &str, value: &str) {
    if let Ok(mut cache) = memory().lock() {
        cache.insert(key.to_string(), value.to_string());
    }
}

fn recall(key: &str) -> Option<String> {
    memory().lock().ok()?.get(key).cloned()
}

fn forget(key: &str) {
    if let Ok(mut cache) = memory().lock() {
        cache.remove(key);
    }
}

fn usable_secret(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.contains(char::is_whitespace) {
        return None;
    }
    Some(value.to_string())
}

/// Load the encrypted local vault. Never touches the macOS keychain.
pub fn init(dir: &Path) {
    if let Ok(mut slot) = vault_dir().lock() {
        *slot = Some(dir.to_path_buf());
    }
    load_vault_into_memory();
}

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
    remember(key, value);
    persist_vault()
}

pub fn keyring_get(key: &str) -> Option<String> {
    let value = recall(key)?;
    if key == OPENROUTER_KEY {
        usable_secret(&value)
    } else {
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    }
}

pub fn keyring_delete(key: &str) {
    forget(key);
    let _ = persist_vault();
}

fn vault_paths() -> Option<(PathBuf, PathBuf)> {
    let dir = vault_dir().lock().ok()?.clone()?;
    Some((dir.join("secrets.key"), dir.join("secrets.enc")))
}

fn restrict_private(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    let _ = path;
}

fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| AppError::msg(e.to_string()))?;
    restrict_private(&tmp);
    fs::rename(&tmp, path).map_err(|e| AppError::msg(e.to_string()))?;
    restrict_private(path);
    Ok(())
}

fn load_or_create_vault_key(key_path: &Path) -> AppResult<[u8; 32]> {
    if let Ok(bytes) = fs::read(key_path) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::msg(e.to_string()))?;
    }
    write_atomic(key_path, &key)?;
    Ok(key)
}

fn load_vault_into_memory() {
    let Some((key_path, enc_path)) = vault_paths() else {
        return;
    };
    let Ok(key) = load_or_create_vault_key(&key_path) else {
        return;
    };
    let Ok(raw) = fs::read_to_string(&enc_path) else {
        return;
    };
    let Ok(file) = serde_json::from_str::<VaultFile>(&raw) else {
        return;
    };
    let Ok(map) = decrypt_vault_map(&file, &key) else {
        return;
    };
    if let Ok(mut cache) = memory().lock() {
        for (name, value) in map {
            if name == OPENROUTER_KEY && usable_secret(&value).is_none() {
                continue;
            }
            cache.entry(name).or_insert(value);
        }
    }
}

fn persist_vault() -> AppResult<()> {
    let Some((key_path, enc_path)) = vault_paths() else {
        return Ok(());
    };
    let key = load_or_create_vault_key(&key_path)?;
    let snapshot: HashMap<String, String> = memory().lock().map(|c| c.clone()).unwrap_or_default();
    let file = encrypt_vault_map(&snapshot, &key)?;
    let encoded = serde_json::to_vec(&file).map_err(|e| AppError::msg(e.to_string()))?;
    write_atomic(&enc_path, &encoded)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    v: u8,
    nonce: String,
    ct: String,
}

fn encrypt_vault_map(map: &HashMap<String, String>, key: &[u8; 32]) -> AppResult<VaultFile> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AppError::msg(e.to_string()))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = serde_json::to_vec(map).map_err(|e| AppError::msg(e.to_string()))?;
    let ct = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| AppError::msg("Unable to seal secret vault"))?;
    Ok(VaultFile {
        v: 1,
        nonce: B64.encode(nonce_bytes),
        ct: B64.encode(ct),
    })
}

fn decrypt_vault_map(file: &VaultFile, key: &[u8; 32]) -> AppResult<HashMap<String, String>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| AppError::msg(e.to_string()))?;
    let ct = B64
        .decode(&file.ct)
        .map_err(|_| AppError::msg("Invalid vault"))?;
    let nonce_bytes = B64
        .decode(&file.nonce)
        .map_err(|_| AppError::msg("Invalid vault"))?;
    if nonce_bytes.len() != 12 {
        return Err(AppError::msg("Invalid vault"));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let pt = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| AppError::msg("Unable to open secret vault"))?;
    serde_json::from_slice(&pt).map_err(|_| AppError::msg("Invalid vault"))
}

pub fn has_openrouter_key() -> bool {
    openrouter_key().is_some()
}

pub fn openrouter_key() -> Option<String> {
    keyring_get(OPENROUTER_KEY)
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
    serde_json::from_str::<WrapState>(&raw)
        .ok()
        .map(|s| s.params)
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
    keyring_set(
        WRAP_STATE,
        &serde_json::to_string(&state).unwrap_or_default(),
    )
}

fn load_wrap_key() -> AppResult<[u8; 32]> {
    let raw =
        keyring_get(WRAP_STATE).ok_or_else(|| AppError::msg("Not signed in for secret sync"))?;
    let state: WrapState =
        serde_json::from_str(&raw).map_err(|_| AppError::msg("Wrap state is invalid"))?;
    let bytes = hex::decode(state.key).map_err(|_| AppError::msg("Wrap state is invalid"))?;
    let mut key = [0u8; 32];
    if bytes.len() != 32 {
        return Err(AppError::msg("Wrap state is invalid"));
    }
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub fn derive_wrap_key(password: &str, email: &str, params: &WrapParams) -> AppResult<[u8; 32]> {
    let salt = B64
        .decode(&params.salt)
        .map_err(|_| AppError::msg("Invalid wrap salt"))?;
    if salt.len() < 8 {
        return Err(AppError::msg("Invalid wrap salt"));
    }
    let argon_params = Params::new(params.m, params.t, params.p, Some(32))
        .map_err(|e| AppError::msg(e.to_string()))?;
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

pub fn decrypt_secret(
    ciphertext_b64: &str,
    nonce_b64: &str,
    wrap_params: &serde_json::Value,
    password: &str,
    email: &str,
) -> AppResult<String> {
    let params: WrapParams = serde_json::from_value(wrap_params.clone())
        .map_err(|_| AppError::msg("Invalid wrap params"))?;
    let key = derive_wrap_key(password, email, &params)?;
    decrypt_with_key(ciphertext_b64, nonce_b64, &key)
}

pub fn decrypt_with_stored_key(ciphertext_b64: &str, nonce_b64: &str) -> AppResult<String> {
    let key = load_wrap_key()?;
    decrypt_with_key(ciphertext_b64, nonce_b64, &key)
}

fn encrypt_with_key(
    plaintext: &[u8],
    key: &[u8; 32],
    params: &WrapParams,
) -> AppResult<(String, String, serde_json::Value)> {
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
    let ct = B64
        .decode(ciphertext_b64)
        .map_err(|_| AppError::msg("Invalid ciphertext"))?;
    let nonce_bytes = B64
        .decode(nonce_b64)
        .map_err(|_| AppError::msg("Invalid nonce"))?;
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

    #[test]
    fn error_sentences_are_not_secrets() {
        assert!(usable_secret("Add an OpenRouter API key in Settings → AI.").is_none());
        assert!(usable_secret("sk-or-v1-abc").is_some());
    }

    #[test]
    fn memory_roundtrip_without_keychain_read() {
        remember("test-openrouter", "sk-or-v1-cached");
        assert_eq!(
            recall("test-openrouter").as_deref(),
            Some("sk-or-v1-cached")
        );
        forget("test-openrouter");
        assert!(recall("test-openrouter").is_none());
    }

    #[test]
    fn vault_roundtrip_survives_memory_clear() {
        let dir = std::env::temp_dir().join(format!(
            "bp-secrets-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir_all(&dir).unwrap();
        init(&dir);
        keyring_set("vault-test-key", "sk-or-v1-disk").unwrap();
        forget("vault-test-key");
        assert!(keyring_get("vault-test-key").is_none());
        init(&dir);
        assert_eq!(
            keyring_get("vault-test-key").as_deref(),
            Some("sk-or-v1-disk")
        );
        keyring_delete("vault-test-key");
        let _ = fs::remove_dir_all(&dir);
    }
}

use std::path::Path;

use llama_cpp_2::gguf::GgufContext;
use serde::{Deserialize, Serialize};

use crate::error::{AiError, AiFailureKind, AiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GgufInfo {
    pub name: String,
    pub arch: Option<String>,
    pub n_ctx_train: Option<u32>,
    pub size_bytes: u64,
    pub chat_template_present: bool,
}

pub fn inspect_gguf(path: impl AsRef<Path>) -> AiResult<GgufInfo> {
    let path = path.as_ref();
    let size_bytes = std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|err| AiError::Message(format!("Could not read GGUF file: {err}")))?;
    let ctx = GgufContext::from_file(path).ok_or_else(|| {
        AiError::provider(
            AiFailureKind::Validation,
            "That file is not a valid GGUF model",
            None,
        )
    })?;
    let arch = read_str(&ctx, "general.architecture");
    let name = read_str(&ctx, "general.name").unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("GGUF model")
            .to_string()
    });
    let n_ctx_train = arch
        .as_deref()
        .and_then(|a| read_u32(&ctx, &format!("{a}.context_length")))
        .or_else(|| read_u32(&ctx, "llama.context_length"))
        .or_else(|| read_u32(&ctx, "general.context_length"));
    let chat_template_present =
        ctx.find_key("tokenizer.chat_template") >= 0 || ctx.find_key("chat_template") >= 0;
    Ok(GgufInfo {
        name,
        arch,
        n_ctx_train,
        size_bytes,
        chat_template_present,
    })
}

fn read_str(ctx: &GgufContext, key: &str) -> Option<String> {
    let idx = ctx.find_key(key);
    if idx < 0 {
        return None;
    }
    ctx.val_str(idx)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn read_u32(ctx: &GgufContext, key: &str) -> Option<u32> {
    let idx = ctx.find_key(key);
    if idx < 0 {
        return None;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ctx.val_u32(idx)))
        .ok()
        .filter(|v| *v > 0)
        .or_else(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ctx.val_u64(idx)))
                .ok()
                .and_then(|v| u32::try_from(v).ok())
                .filter(|v| *v > 0)
        })
        .or_else(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ctx.val_i32(idx)))
                .ok()
                .and_then(|v| u32::try_from(v).ok())
                .filter(|v| *v > 0)
        })
}

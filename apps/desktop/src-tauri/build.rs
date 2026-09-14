fn main() {
    tauri_build::build();
    #[cfg(windows)]
    copy_cuda_runtime_dlls();
}

#[cfg(windows)]
fn copy_cuda_runtime_dlls() {
    println!("cargo:rerun-if-env-changed=CUDA_PATH");
    let Some(bin) = cuda_bin_dir() else {
        println!(
            "cargo:warning=CUDA_PATH not found; CUDA runtime DLLs were not copied next to the exe"
        );
        return;
    };
    println!("cargo:rerun-if-changed={}", bin.display());
    let Some(dest) = profile_dir_from_out_dir() else {
        println!("cargo:warning=Could not resolve the cargo profile directory for CUDA DLLs");
        return;
    };
    let prefixes = ["cudart64_", "cublas64_", "cublasLt64_"];
    let Ok(entries) = std::fs::read_dir(&bin) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".dll") {
            continue;
        }
        if !prefixes.iter().any(|prefix| lower.starts_with(prefix)) {
            continue;
        }
        let target = dest.join(name);
        if let Err(err) = std::fs::copy(&path, &target) {
            println!(
                "cargo:warning=Failed to copy {} to {}: {err}",
                path.display(),
                target.display()
            );
        }
    }
}

#[cfg(windows)]
fn cuda_bin_dir() -> Option<std::path::PathBuf> {
    if let Some(bin) = nvcc_bin_dir() {
        return Some(bin);
    }
    if let Ok(root) = std::env::var("CUDA_PATH") {
        let bin = std::path::PathBuf::from(root).join("bin");
        if bin.is_dir() {
            return Some(bin);
        }
    }
    let toolkit = std::path::PathBuf::from(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA");
    let mut versions = std::fs::read_dir(toolkit)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    versions.sort();
    let latest = versions.pop()?;
    let bin = latest.join("bin");
    bin.is_dir().then_some(bin)
}

#[cfg(windows)]
fn nvcc_bin_dir() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let nvcc = dir.join("nvcc.exe");
        if nvcc.is_file() {
            return Some(dir);
        }
    }
    None
}

#[cfg(windows)]
fn profile_dir_from_out_dir() -> Option<std::path::PathBuf> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").ok()?);
    let profile = std::env::var("PROFILE").ok()?;
    out_dir
        .ancestors()
        .find(|path| path.file_name().and_then(|n| n.to_str()) == Some(profile.as_str()))
        .map(|path| path.to_path_buf())
}

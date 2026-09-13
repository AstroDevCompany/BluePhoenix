use crate::error::{AppError, AppResult};
use crate::models::FileEntryDto;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn list_dir_shallow(path: &Path) -> AppResult<Vec<FileEntryDto>> {
    if !path.exists() {
        return Err(AppError::FolderMissing);
    }
    if !path.is_dir() {
        return Err(AppError::msg("Path is not a folder"));
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        entries.push(FileEntryDto {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            size_bytes: if meta.is_file() {
                Some(meta.len() as i64)
            } else {
                None
            },
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

pub fn open_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(if path.extension().is_some() {
            AppError::OpenFile
        } else {
            AppError::FolderMissing
        });
    }
    open::that(path).map_err(|_| AppError::OpenFile)
}

pub fn reveal_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(AppError::FolderMissing);
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn()
            .map_err(|_| AppError::OpenFile)?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path.to_string_lossy()])
            .spawn()
            .map_err(|_| AppError::OpenFile)?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(parent) = path.parent() {
            open_path(parent)?;
        } else {
            open_path(path)?;
        }
        Ok(())
    }
}

pub fn open_vscode(configured: &str, folder: &Path) -> AppResult<()> {
    if !folder.exists() {
        return Err(AppError::FolderMissing);
    }
    let candidates: Vec<String> = if !configured.trim().is_empty() {
        vec![configured.trim().to_string()]
    } else {
        vec![
            "code".into(),
            "code.cmd".into(),
            "code.exe".into(),
            #[cfg(target_os = "macos")]
            "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code".into(),
            #[cfg(target_os = "windows")]
            format!(
                "{}\\Microsoft VS Code\\bin\\code.cmd",
                std::env::var("LOCALAPPDATA").unwrap_or_default()
            ),
        ]
    };
    for bin in candidates {
        if Command::new(&bin).arg(folder.as_os_str()).spawn().is_ok() {
            return Ok(());
        }
    }
    Err(AppError::VsCodeNotFound)
}

pub fn open_terminal(preference: &str, folder: &Path) -> AppResult<()> {
    if !folder.exists() {
        return Err(AppError::FolderMissing);
    }
    #[cfg(target_os = "windows")]
    {
        let pref = preference.trim().to_lowercase();
        if pref == "cmd" {
            Command::new("cmd")
                .args(["/K", &format!("cd /d {}", folder.display())])
                .spawn()?;
            return Ok(());
        }
        if Command::new("wt")
            .args(["-d", &folder.to_string_lossy()])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
        Command::new("cmd")
            .args(["/K", &format!("cd /d {}", folder.display())])
            .spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = preference;
        let script = format!(
            "tell application \"Terminal\" to do script \"cd {}\"",
            folder.display()
        );
        Command::new("osascript").args(["-e", &script]).spawn()?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = preference;
        Command::new("x-terminal-emulator")
            .current_dir(folder)
            .spawn()
            .or_else(|_| Command::new("gnome-terminal").current_dir(folder).spawn())?;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn open_url(url: &str) -> AppResult<()> {
    open::that(url).map_err(|e| AppError::msg(e.to_string()))
}

pub fn data_dir() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| AppError::msg("No data directory"))?;
    let dir = base.join("BluePhoenix");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn models_dir() -> AppResult<PathBuf> {
    let dir = data_dir()?.join("models");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

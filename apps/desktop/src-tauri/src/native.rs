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

/// A launchable external app (editor or terminal) shown in the UI.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOption {
    pub id: String,
    pub name: String,
}

struct EditorDef {
    id: &'static str,
    name: &'static str,
    /// macOS application bundle names (without `.app`), checked in /Applications and ~/Applications.
    mac_apps: &'static [&'static str],
    /// Command-line launchers, tried in order.
    cli: &'static [&'static str],
}

const EDITORS: &[EditorDef] = &[
    EditorDef {
        id: "vscode",
        name: "Visual Studio Code",
        mac_apps: &["Visual Studio Code"],
        cli: &["code", "code.cmd"],
    },
    EditorDef {
        id: "cursor",
        name: "Cursor",
        mac_apps: &["Cursor"],
        cli: &["cursor", "cursor.cmd"],
    },
    EditorDef {
        id: "zed",
        name: "Zed",
        mac_apps: &["Zed"],
        cli: &["zed"],
    },
    EditorDef {
        id: "sublime",
        name: "Sublime Text",
        mac_apps: &["Sublime Text"],
        cli: &["subl"],
    },
    EditorDef {
        id: "idea",
        name: "IntelliJ IDEA",
        mac_apps: &["IntelliJ IDEA", "IntelliJ IDEA CE"],
        cli: &["idea", "idea.cmd"],
    },
    EditorDef {
        id: "webstorm",
        name: "WebStorm",
        mac_apps: &["WebStorm"],
        cli: &["webstorm", "webstorm.cmd"],
    },
    EditorDef {
        id: "pycharm",
        name: "PyCharm",
        mac_apps: &["PyCharm", "PyCharm CE"],
        cli: &["pycharm", "pycharm.cmd"],
    },
    EditorDef {
        id: "goland",
        name: "GoLand",
        mac_apps: &["GoLand"],
        cli: &["goland", "goland.cmd"],
    },
    EditorDef {
        id: "rustrover",
        name: "RustRover",
        mac_apps: &["RustRover"],
        cli: &["rustrover", "rustrover.cmd"],
    },
    EditorDef {
        id: "clion",
        name: "CLion",
        mac_apps: &["CLion"],
        cli: &["clion", "clion.cmd"],
    },
    EditorDef {
        id: "phpstorm",
        name: "PhpStorm",
        mac_apps: &["PhpStorm"],
        cli: &["phpstorm", "phpstorm.cmd"],
    },
    EditorDef {
        id: "rider",
        name: "Rider",
        mac_apps: &["Rider"],
        cli: &["rider", "rider.cmd"],
    },
    EditorDef {
        id: "androidstudio",
        name: "Android Studio",
        mac_apps: &["Android Studio"],
        cli: &["studio", "studio.cmd"],
    },
    EditorDef {
        id: "xcode",
        name: "Xcode",
        mac_apps: &["Xcode"],
        cli: &[],
    },
];

#[cfg(target_os = "macos")]
fn mac_app_installed(name: &str) -> bool {
    let bundle = format!("{name}.app");
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }
    roots.iter().any(|r| r.join(&bundle).exists())
}

#[cfg(not(target_os = "macos"))]
fn mac_app_installed(_name: &str) -> bool {
    false
}

/// Extra locations where launchers live but are not on a GUI app's PATH.
fn cli_candidates(cli: &str) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(path) = which::which(cli) {
        found.push(path.to_string_lossy().to_string());
    }
    #[cfg(target_os = "macos")]
    {
        for dir in ["/opt/homebrew/bin", "/usr/local/bin"] {
            let candidate = format!("{dir}/{cli}");
            if Path::new(&candidate).is_file() {
                found.push(candidate);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let toolbox = home
                .join("Library/Application Support/JetBrains/Toolbox/scripts")
                .join(cli);
            if toolbox.is_file() {
                found.push(toolbox.to_string_lossy().to_string());
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let roots = [
                format!("{local}\\Programs\\Microsoft VS Code\\bin\\{cli}"),
                format!("{local}\\Microsoft VS Code\\bin\\{cli}"),
                format!("{local}\\Programs\\cursor\\resources\\app\\bin\\{cli}"),
                format!("{local}\\JetBrains\\Toolbox\\scripts\\{cli}"),
            ];
            found.extend(roots.into_iter().filter(|p| Path::new(p).is_file()));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            let candidate = format!("{pf}\\Microsoft VS Code\\bin\\{cli}");
            if Path::new(&candidate).is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

fn editor_installed(def: &EditorDef) -> bool {
    def.mac_apps.iter().any(|a| mac_app_installed(a))
        || def.cli.iter().any(|c| !cli_candidates(c).is_empty())
}

pub fn list_editors() -> Vec<LaunchOption> {
    EDITORS
        .iter()
        .filter(|d| editor_installed(d))
        .map(|d| LaunchOption {
            id: d.id.into(),
            name: d.name.into(),
        })
        .collect()
}

/// Opens `folder` in the chosen editor; `None` or an unavailable id falls back to the first installed one.
pub fn open_in_editor(editor: Option<&str>, folder: &Path) -> AppResult<()> {
    if !folder.exists() {
        return Err(AppError::FolderMissing);
    }
    let wanted = editor.map(str::trim).filter(|e| !e.is_empty());
    let def = wanted
        .and_then(|id| {
            EDITORS
                .iter()
                .find(|d| d.id == id)
                .filter(|d| editor_installed(d))
        })
        .or_else(|| EDITORS.iter().find(|d| editor_installed(d)))
        .ok_or(AppError::VsCodeNotFound)?;

    #[cfg(target_os = "macos")]
    {
        // `open -a` works for every app bundle, even when no shell launcher is installed.
        for app in def.mac_apps {
            if mac_app_installed(app) {
                let status = Command::new("open")
                    .args(["-a", app])
                    .arg(folder.as_os_str())
                    .status();
                if matches!(status, Ok(s) if s.success()) {
                    return Ok(());
                }
            }
        }
    }
    for cli in def.cli {
        for bin in cli_candidates(cli) {
            if Command::new(&bin).arg(folder.as_os_str()).spawn().is_ok() {
                return Ok(());
            }
        }
    }
    Err(AppError::VsCodeNotFound)
}

#[cfg(target_os = "macos")]
const MAC_TERMINALS: &[(&str, &str, &str)] = &[
    ("terminal", "Terminal", "Terminal"),
    ("iterm", "iTerm", "iTerm"),
    ("warp", "Warp", "Warp"),
    ("ghostty", "Ghostty", "Ghostty"),
    ("kitty", "kitty", "kitty"),
];

pub fn list_terminals() -> Vec<LaunchOption> {
    #[cfg(target_os = "macos")]
    {
        MAC_TERMINALS
            .iter()
            .filter(|(id, _, app)| *id == "terminal" || mac_app_installed(app))
            .map(|(id, name, _)| LaunchOption {
                id: (*id).into(),
                name: (*name).into(),
            })
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec![LaunchOption {
            id: "default".into(),
            name: if cfg!(windows) {
                "Windows Terminal"
            } else {
                "System terminal"
            }
            .into(),
        }]
    }
}

/// Quotes a value for POSIX shells.
pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Opens the system terminal in `folder`, optionally running `command` and keeping the shell open afterwards.
pub fn open_terminal(preferred: &str, folder: &Path, command: Option<&str>) -> AppResult<()> {
    if !folder.exists() {
        return Err(AppError::FolderMissing);
    }
    let command = command.map(str::trim).filter(|c| !c.is_empty());
    #[cfg(target_os = "windows")]
    {
        let _ = preferred;
        let dir = folder.to_string_lossy().to_string();
        let mut args = vec!["-d".to_string(), dir.clone()];
        if let Some(cmd) = command {
            args.extend(["cmd".into(), "/K".into(), cmd.to_string()]);
        }
        if Command::new("wt").args(&args).spawn().is_ok() {
            return Ok(());
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let wt = format!("{local}\\Microsoft\\WindowsApps\\wt.exe");
            if Command::new(&wt).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }
        let line = match command {
            Some(cmd) => format!("cd /d {dir} && {cmd}"),
            None => format!("cd /d {dir}"),
        };
        Command::new("cmd").args(["/K", &line]).spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let shell_line = match command {
            Some(cmd) => format!("cd {} && {}", sh_quote(&folder.to_string_lossy()), cmd),
            None => format!("cd {}", sh_quote(&folder.to_string_lossy())),
        };
        let wanted = if preferred.is_empty() || preferred == "default" {
            "terminal"
        } else {
            preferred
        };
        let id = MAC_TERMINALS
            .iter()
            .find(|(id, _, app)| *id == wanted && (*id == "terminal" || mac_app_installed(app)))
            .map(|(id, _, _)| *id)
            .unwrap_or("terminal");
        // For terminals that take argv, keep an interactive shell open once the command ends.
        let keep_open = match command {
            Some(cmd) => format!("{cmd}; exec \"${{SHELL:-/bin/zsh}}\" -l"),
            None => "exec \"${SHELL:-/bin/zsh}\" -l".to_string(),
        };
        match id {
            "iterm" => {
                let script = format!(
                    "tell application \"iTerm\"\nactivate\nset w to (create window with default profile)\ntell current session of w to write text \"{}\"\nend tell",
                    applescript_string(&shell_line)
                );
                Command::new("osascript").args(["-e", &script]).spawn()?;
            }
            "warp" => {
                // Warp has no scripting entry point for commands; open it at the folder.
                Command::new("open")
                    .args(["-a", "Warp"])
                    .arg(folder.as_os_str())
                    .spawn()?;
            }
            "ghostty" => {
                let mut cmd = Command::new("open");
                cmd.args(["-na", "Ghostty", "--args"]);
                cmd.arg(format!("--working-directory={}", folder.to_string_lossy()));
                if command.is_some() {
                    cmd.args(["-e", "/bin/sh", "-c", &keep_open]);
                }
                cmd.spawn()?;
            }
            "kitty" => {
                let mut cmd = Command::new("open");
                cmd.args(["-na", "kitty", "--args", "--directory"]);
                cmd.arg(folder.as_os_str());
                if command.is_some() {
                    cmd.args(["/bin/sh", "-c", &keep_open]);
                }
                cmd.spawn()?;
            }
            _ => {
                let script = format!(
                    "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
                    applescript_string(&shell_line)
                );
                Command::new("osascript").args(["-e", &script]).spawn()?;
            }
        }
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = preferred;
        let keep_open = match command {
            Some(cmd) => format!("{cmd}; exec \"${{SHELL:-/bin/sh}}\""),
            None => String::new(),
        };
        let mut x = Command::new("x-terminal-emulator");
        x.current_dir(folder);
        if command.is_some() {
            x.args(["-e", "sh", "-c", &keep_open]);
        }
        if x.spawn().is_ok() {
            return Ok(());
        }
        let mut g = Command::new("gnome-terminal");
        g.current_dir(folder);
        if command.is_some() {
            g.args(["--", "sh", "-c", &keep_open]);
        }
        g.spawn()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quoting_survives_apostrophes() {
        assert_eq!(sh_quote("/Users/me/it's here"), r"'/Users/me/it'\''s here'");
        assert_eq!(sh_quote("plain"), "'plain'");
    }

    #[test]
    fn editor_ids_are_unique() {
        let mut ids: Vec<_> = EDITORS.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), EDITORS.len());
    }

    #[test]
    fn missing_folder_is_reported() {
        let missing = Path::new("/definitely/not/here");
        assert!(matches!(
            open_in_editor(None, missing),
            Err(AppError::FolderMissing)
        ));
        assert!(matches!(
            open_terminal("", missing, None),
            Err(AppError::FolderMissing)
        ));
    }
}

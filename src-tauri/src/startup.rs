use std::io;
use std::path::Path;
#[cfg(target_os = "windows")]
use std::process::Command;

pub const APP_RUN_NAME: &str = "Codex Island";
#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
pub fn is_enabled() -> bool {
    Command::new("reg")
        .args(["query", RUN_KEY, "/v", APP_RUN_NAME])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool, exe_path: &Path) -> io::Result<()> {
    let status = if enabled {
        let value = quoted_path(exe_path);
        Command::new("reg")
            .args(["add", RUN_KEY, "/v", APP_RUN_NAME, "/t", "REG_SZ", "/d"])
            .arg(value)
            .args(["/f"])
            .status()?
    } else {
        Command::new("reg")
            .args(["delete", RUN_KEY, "/v", APP_RUN_NAME, "/f"])
            .status()?
    };

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "failed to update Windows startup registry value",
        ))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_enabled: bool, _exe_path: &Path) -> io::Result<()> {
    Ok(())
}

pub fn quoted_path(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

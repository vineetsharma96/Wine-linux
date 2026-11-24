use std::path::{Path, PathBuf};
use androidpc_runtime_api::ProcessCommand;
use crate::config::WineConfig;

pub struct WineCommandBuilder;

impl WineCommandBuilder {
    /// Builds the `wineboot -u` command to initialize or update a Wine prefix.
    pub fn build_wineboot_command(config: &WineConfig) -> ProcessCommand {
        ProcessCommand {
            program: "box64".to_string(),
            args: vec!["wine".to_string(), "wineboot".to_string(), "-u".to_string()],
            env: config.to_env_vars(),
            cwd: Some(config.prefix_path.display().to_string()),
            uid: None,
            gid: None,
        }
    }

    /// Builds the execution command for a Windows PE binary (`.exe`) using Box64 + Wine.
    pub fn build_launch_command(
        config: &WineConfig,
        exe_path: &str,
        args: &[String],
    ) -> ProcessCommand {
        let mut cmd_args = vec!["wine".to_string(), exe_path.to_string()];
        cmd_args.extend(args.iter().cloned());

        let exe_dir = Path::new(exe_path)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| config.prefix_path.display().to_string());

        ProcessCommand {
            program: "box64".to_string(),
            args: cmd_args,
            env: config.to_env_vars(),
            cwd: Some(exe_dir),
            uid: None,
            gid: None,
        }
    }

    /// Builds a command to cleanly kill the wineserver and all processes running in the prefix.
    pub fn build_wineserver_kill_command(config: &WineConfig) -> ProcessCommand {
        ProcessCommand {
            program: "wineserver".to_string(),
            args: vec!["-k".to_string()],
            env: vec![("WINEPREFIX".to_string(), config.prefix_path.display().to_string())],
            cwd: None,
            uid: None,
            gid: None,
        }
    }

    /// Builds a command to launch `winecfg` configuration GUI.
    pub fn build_winecfg_command(config: &WineConfig) -> ProcessCommand {
        ProcessCommand {
            program: "box64".to_string(),
            args: vec!["wine".to_string(), "winecfg".to_string()],
            env: config.to_env_vars(),
            cwd: Some(config.prefix_path.display().to_string()),
            uid: None,
            gid: None,
        }
    }

    /// Builds a command to launch Windows command prompt (`cmd.exe`).
    pub fn build_cmd_command(config: &WineConfig) -> ProcessCommand {
        ProcessCommand {
            program: "box64".to_string(),
            args: vec!["wine".to_string(), "cmd.exe".to_string()],
            env: config.to_env_vars(),
            cwd: Some(config.prefix_path.display().to_string()),
            uid: None,
            gid: None,
        }
    }

    /// Translates a guest Unix path inside the prefix C: drive to a Windows path (`C:\...`).
    /// Also translates standard Linux rootfs paths to `Z:\...` (Wine default rootfs drive).
    pub fn to_windows_path(unix_path: &Path, prefix_path: &Path) -> Option<String> {
        let drive_c = prefix_path.join("drive_c");
        if let Ok(rel) = unix_path.strip_prefix(&drive_c) {
            let rel_str = rel.to_string_lossy().replace('/', "\\");
            if rel_str.is_empty() {
                Some("C:\\".to_string())
            } else {
                Some(format!("C:\\{}", rel_str))
            }
        } else {
            // Map general Linux path to Z: drive
            let path_str = unix_path.to_string_lossy();
            let rel_str = path_str.trim_start_matches('/').replace('/', "\\");
            if rel_str.is_empty() {
                Some("Z:\\".to_string())
            } else {
                Some(format!("Z:\\{}", rel_str))
            }
        }
    }

    /// Translates a Windows path (`C:\...` or `Z:\...`) to a guest Unix path.
    pub fn to_unix_path(win_path: &str, prefix_path: &Path) -> Option<PathBuf> {
        let clean = win_path.trim();
        if clean.len() < 2 {
            return None;
        }

        let drive_letter = clean.chars().next()?.to_ascii_uppercase();
        if clean.chars().nth(1) != Some(':') {
            return None;
        }

        let remainder = &clean[2..].trim_start_matches('\\').trim_start_matches('/');
        let unix_rel = remainder.replace('\\', "/");

        match drive_letter {
            'C' => Some(prefix_path.join("drive_c").join(unix_rel)),
            'Z' => Some(PathBuf::from("/").join(unix_rel)),
            _ => None,
        }
    }
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use androidpc_runtime_api::ProcessCommand;

use crate::config::WineConfig;
use crate::error::WineError;
use crate::prefix::WinePrefixMetadata;
use crate::registry::WineRegistryManager;

pub const WINE_PINNED_VERSION: &str = "11.0";
pub const WINE_LICENSE: &str = "LGPL-2.1-or-later";
pub const WINE_UPSTREAM_URL: &str = "https://gitlab.winehq.org/wine/wine";
pub const WINE_SELECTED_ARCHITECTURE: &str = "x86_64 Wine 11.0 (pure-64-bit WoW64) via Box64";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineInstallationStatus {
    pub installed: bool,
    pub binary_path: Option<String>,
    pub detected_version: Option<String>,
    pub pinned_version: String,
    pub license: String,
    pub architecture_strategy: String,
    pub wow64_enabled: bool,
    pub box86_required: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowsProcessState {
    Launching,
    Running,
    Exiting,
    Exited,
    Terminated,
    Crashed,
}

/// Structured process record for Milestone 4.8 — tracks ownership across
/// application ID, isolated Wine prefix, Linux environment, Wine/Box64 PIDs,
/// child PIDs, RSS memory (when measurable), and exit reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedWindowsProcess {
    pub app_id: String,
    pub app_name: String,
    pub prefix_id: String,
    pub environment_id: String,
    pub wine_pid: Option<u32>,
    pub box64_pid: Option<u32>,
    pub child_pids: Vec<u32>,
    pub state: WindowsProcessState,
    pub memory_rss_bytes: Option<u64>,
    pub exit_code: Option<i32>,
    pub exit_reason: Option<String>,
    pub started_at_epoch_ms: u64,
    pub ended_at_epoch_ms: Option<u64>,
}

pub struct WineRuntimeManager {
    artifacts_dir: PathBuf,
    tracked_processes: RwLock<HashMap<String, TrackedWindowsProcess>>,
}

impl WineRuntimeManager {
    pub fn new(artifacts_dir: PathBuf) -> Self {
        Self {
            artifacts_dir,
            tracked_processes: RwLock::new(HashMap::new()),
        }
    }

    fn now_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Probes staged artifacts and the selected guest rootfs for the Wine `11.0` loader.
    pub fn probe_installation(&self, guest_rootfs: Option<&Path>) -> WineInstallationStatus {
        let mut diagnostics = Vec::new();
        let mut found_path: Option<String> = None;

        if let Some(rootfs) = guest_rootfs {
            for rel in [
                "opt/wine/bin/wine",
                "usr/local/bin/wine",
                "usr/bin/wine",
            ] {
                if rootfs.join(rel).is_file() {
                    found_path = Some(format!("/{rel}"));
                    diagnostics.push(format!("Detected Wine loader inside guest rootfs: /{rel}"));
                    break;
                }
            }
        }

        if found_path.is_none() {
            let staged = self.artifacts_dir.join("wine").join("bin").join("wine");
            if staged.is_file() {
                found_path = Some(staged.display().to_string());
                diagnostics.push(format!("Detected staged Wine binary: {}", staged.display()));
            }
        }

        let installed = found_path.is_some();
        if !installed {
            diagnostics.push(
                "Wine 11.0 WoW64 x86-64 binary not staged yet. Stage verified Wine tarball under artifacts/wine/ or /opt/wine inside guest rootfs."
                    .to_string(),
            );
        }
        diagnostics.push(
            "Architecture: x86-64 Wine 11.0 WoW64 executed via Box64 (supports PE32+ x64 and PE32 i386 without Box86)."
                .to_string(),
        );

        WineInstallationStatus {
            installed,
            binary_path: found_path,
            detected_version: if installed {
                Some(WINE_PINNED_VERSION.to_string())
            } else {
                None
            },
            pinned_version: WINE_PINNED_VERSION.to_string(),
            license: WINE_LICENSE.to_string(),
            architecture_strategy: WINE_SELECTED_ARCHITECTURE.to_string(),
            wow64_enabled: true,
            box86_required: false,
            diagnostics,
        }
    }

    /// Builds a launch command for an `.exe` or `.msi` inside an isolated Wine prefix,
    /// combining Box64 env, Wine prefix env, graphics backend env, DXVK/VKD3D overrides,
    /// optional Wine virtual desktop resolution, and per-app custom environment variables.
    pub fn build_prefix_launch_command(
        prefix: &WinePrefixMetadata,
        target_path: &str,
        args: &[String],
        working_dir: Option<&str>,
        virtual_desktop_resolution: Option<(u32, u32)>,
        is_msi: bool,
        graphics_env: &[(String, String)],
        extra_dll_overrides: &[(String, String)],
        extra_env: &[(String, String)],
    ) -> ProcessCommand {
        let mut wine_cfg = WineConfig::default();
        wine_cfg.prefix_path = PathBuf::from(&prefix.prefix_path);
        wine_cfg.arch = prefix.arch;
        wine_cfg.windows_version = prefix.windows_version.clone();
        wine_cfg.dxvk_enabled = prefix.dxvk_enabled;

        let mut env = wine_cfg.to_env_vars();
        // Remove default WINEDLLOVERRIDES so we replace it cleanly with merged overrides.
        env.retain(|(k, _)| k != "WINEDLLOVERRIDES");

        let mut all_overrides: Vec<(String, String)> = prefix
            .dll_overrides
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        all_overrides.extend(extra_dll_overrides.iter().cloned());

        let merged_overrides = WineRegistryManager::build_dll_overrides(
            &[("mscoree", ""), ("mshtml", "")],
            &all_overrides,
        );
        env.push(("WINEDLLOVERRIDES".to_string(), merged_overrides));
        env.push(("WINE_WOW64".to_string(), "1".to_string()));
        env.extend(graphics_env.iter().cloned());

        for (k, v) in &prefix.env_vars {
            env.push((k.clone(), v.clone()));
        }
        for (k, v) in extra_env {
            env.push((k.clone(), v.clone()));
        }

        let mut cmd_args = vec!["wine".to_string()];
        if let Some((w, h)) = virtual_desktop_resolution {
            if w >= 640 && h >= 480 {
                cmd_args.push("explorer".to_string());
                cmd_args.push(format!("/desktop={},{}x{}", prefix.id, w, h));
            }
        }
        if is_msi {
            cmd_args.push("msiexec".to_string());
            cmd_args.push("/i".to_string());
            cmd_args.push(target_path.to_string());
        } else {
            cmd_args.push(target_path.to_string());
        }
        cmd_args.extend(args.iter().cloned());

        let cwd = working_dir
            .filter(|d| !d.trim().is_empty())
            .map(|d| d.to_string())
            .or_else(|| {
                Path::new(target_path)
                    .parent()
                    .map(|p| p.display().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| prefix.prefix_path.clone());

        ProcessCommand {
            program: "box64".to_string(),
            args: cmd_args,
            env,
            cwd: Some(cwd),
            uid: None,
            gid: None,
        }
    }

    /// Builds a scoped `wineserver -k` command targeting ONLY the specified prefix
    /// (`WINEPREFIX`) — never broad system-wide process kills.
    pub fn build_scoped_prefix_shutdown(prefix: &WinePrefixMetadata, force: bool) -> ProcessCommand {
        ProcessCommand {
            program: "box64".to_string(),
            args: vec![
                "wineserver".to_string(),
                if force { "-k9".to_string() } else { "-k".to_string() },
            ],
            env: vec![("WINEPREFIX".to_string(), prefix.prefix_path.clone())],
            cwd: None,
            uid: None,
            gid: None,
        }
    }

    pub fn register_tracked_process(
        &self,
        app_id: &str,
        app_name: &str,
        prefix_id: &str,
        environment_id: &str,
        pid: Option<u32>,
    ) -> Result<TrackedWindowsProcess, WineError> {
        let entry = TrackedWindowsProcess {
            app_id: app_id.to_string(),
            app_name: app_name.to_string(),
            prefix_id: prefix_id.to_string(),
            environment_id: environment_id.to_string(),
            wine_pid: pid,
            box64_pid: pid,
            child_pids: Vec::new(),
            state: if pid.is_some() {
                WindowsProcessState::Running
            } else {
                WindowsProcessState::Launching
            },
            memory_rss_bytes: pid.and_then(read_proc_rss_bytes),
            exit_code: None,
            exit_reason: None,
            started_at_epoch_ms: Self::now_epoch_ms(),
            ended_at_epoch_ms: None,
        };
        let mut guard = self
            .tracked_processes
            .write()
            .map_err(|_| WineError::Supervisor("Tracked process lock poisoned".into()))?;
        guard.insert(app_id.to_string(), entry.clone());
        Ok(entry)
    }

    pub fn mark_process_terminated(
        &self,
        app_id: &str,
        reason: &str,
        exit_code: Option<i32>,
    ) -> Result<Option<TrackedWindowsProcess>, WineError> {
        let mut guard = self
            .tracked_processes
            .write()
            .map_err(|_| WineError::Supervisor("Tracked process lock poisoned".into()))?;
        if let Some(entry) = guard.get_mut(app_id) {
            entry.state = WindowsProcessState::Terminated;
            entry.exit_code = exit_code;
            entry.exit_reason = Some(reason.to_string());
            entry.ended_at_epoch_ms = Some(Self::now_epoch_ms());
            return Ok(Some(entry.clone()));
        }
        Ok(None)
    }

    pub fn list_tracked_processes(&self) -> Vec<TrackedWindowsProcess> {
        let Ok(mut guard) = self.tracked_processes.write() else {
            return Vec::new();
        };
        for proc_info in guard.values_mut() {
            if proc_info.state == WindowsProcessState::Running {
                if let Some(pid) = proc_info.wine_pid {
                    proc_info.memory_rss_bytes = read_proc_rss_bytes(pid);
                }
            }
        }
        let mut list: Vec<TrackedWindowsProcess> = guard.values().cloned().collect();
        list.sort_by(|a, b| b.started_at_epoch_ms.cmp(&a.started_at_epoch_ms));
        list
    }
}

fn read_proc_rss_bytes(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

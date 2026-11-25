use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use androidpc_configuration::Box64AppOverride;
use androidpc_graphics_backend::GraphicsBackendKind;

const MAX_LOG_LINES_PER_APP: usize = 200;

#[derive(Debug, Error)]
pub enum WindowsAppError {
    #[error("Windows application not found: {0}")]
    NotFound(String),

    #[error("Invalid application profile: {0}")]
    InvalidProfile(String),

    #[error("Security error: {0}")]
    Security(#[from] androidpc_security::SecurityError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsAppProfile {
    pub app_id: String,
    pub name: String,
    pub executable_path: String,
    pub install_dir: String,
    pub prefix_id: String,
    pub environment_id: String,
    pub wine_version: String,
    pub box64_override: Option<Box64AppOverride>,
    pub graphics_backend: GraphicsBackendKind,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub launch_args: Vec<String>,
    pub env_vars: BTreeMap<String, String>,
    pub compatibility_notes: String,
    pub created_at_epoch_ms: u64,
    pub last_launched_at_epoch_ms: Option<u64>,
    pub shortcut_created: bool,
}

/// Manages the user's Windows application library, per-app profiles,
/// XFCE desktop shortcuts, and bounded per-app launch/runtime logs.
pub struct WindowsAppLibrary {
    apps_dir: PathBuf,
    app_logs: RwLock<HashMap<String, VecDeque<String>>>,
}

impl WindowsAppLibrary {
    pub fn new(base_data_dir: &Path) -> Self {
        Self {
            apps_dir: base_data_dir.join("windows_apps"),
            app_logs: RwLock::new(HashMap::new()),
        }
    }

    fn now_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn validate_app_id(id: &str) -> Result<(), WindowsAppError> {
        let valid = !id.is_empty()
            && id.len() <= 64
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
        if !valid {
            return Err(WindowsAppError::InvalidProfile(format!(
                "Invalid application ID '{id}': use 1..=64 ASCII alphanumeric, '-' or '_' characters"
            )));
        }
        Ok(())
    }

    fn profile_file(&self, app_id: &str) -> PathBuf {
        self.apps_dir.join(format!("{app_id}.json"))
    }

    pub fn register_app(&self, mut profile: WindowsAppProfile) -> Result<WindowsAppProfile, WindowsAppError> {
        Self::validate_app_id(&profile.app_id)?;
        if profile.name.trim().is_empty() {
            return Err(WindowsAppError::InvalidProfile("Application name cannot be empty".into()));
        }
        if profile.executable_path.trim().is_empty() {
            return Err(WindowsAppError::InvalidProfile("Executable path cannot be empty".into()));
        }
        if profile.prefix_id.trim().is_empty() {
            return Err(WindowsAppError::InvalidProfile("Prefix ID cannot be empty".into()));
        }
        if profile.resolution_width < 640 {
            profile.resolution_width = 1280;
        }
        if profile.resolution_height < 480 {
            profile.resolution_height = 720;
        }
        if profile.created_at_epoch_ms == 0 {
            profile.created_at_epoch_ms = Self::now_epoch_ms();
        }
        fs::create_dir_all(&self.apps_dir)?;
        fs::write(
            self.profile_file(&profile.app_id),
            serde_json::to_vec_pretty(&profile)?,
        )?;
        Ok(profile)
    }

    pub fn list_apps(&self) -> Vec<WindowsAppProfile> {
        let Ok(entries) = fs::read_dir(&self.apps_dir) else {
            return Vec::new();
        };
        let mut apps = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(raw) = fs::read_to_string(&p) {
                    if let Ok(app) = serde_json::from_str::<WindowsAppProfile>(&raw) {
                        apps.push(app);
                    }
                }
            }
        }
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        apps
    }

    pub fn get_app(&self, app_id: &str) -> Result<WindowsAppProfile, WindowsAppError> {
        Self::validate_app_id(app_id)?;
        let path = self.profile_file(app_id);
        if !path.is_file() {
            return Err(WindowsAppError::NotFound(app_id.to_string()));
        }
        let raw = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn mark_launched(&self, app_id: &str) -> Result<WindowsAppProfile, WindowsAppError> {
        let mut app = self.get_app(app_id)?;
        app.last_launched_at_epoch_ms = Some(Self::now_epoch_ms());
        self.register_app(app)
    }

    pub fn remove_app(&self, app_id: &str, guest_rootfs: Option<&Path>) -> Result<(), WindowsAppError> {
        Self::validate_app_id(app_id)?;
        let path = self.profile_file(app_id);
        if path.is_file() {
            fs::remove_file(path)?;
        }
        if let Some(rootfs) = guest_rootfs {
            if let Ok(shortcut) = androidpc_security::resolve_safe_path(
                rootfs,
                Path::new(&format!("home/user/Desktop/androidpc-{app_id}.desktop")),
            ) {
                if shortcut.is_file() {
                    let _ = fs::remove_file(shortcut);
                }
            }
        }
        if let Ok(mut logs) = self.app_logs.write() {
            logs.remove(app_id);
        }
        Ok(())
    }

    /// Generates an XDG `.desktop` shortcut inside the guest XFCE desktop directory
    /// using safe path resolution and without hardcoding host Android paths.
    pub fn create_xfce_shortcut(
        &self,
        app_id: &str,
        guest_rootfs: &Path,
        prefix_guest_path: &str,
    ) -> Result<PathBuf, WindowsAppError> {
        let mut app = self.get_app(app_id)?;
        let rel = PathBuf::from(format!("home/user/Desktop/androidpc-{}.desktop", app.app_id));
        let shortcut_path = androidpc_security::resolve_safe_path(guest_rootfs, &rel)?;
        if let Some(parent) = shortcut_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let sanitized_name = app.name.replace(['\r', '\n'], " ");
        let sanitized_exe = app.executable_path.replace(['\r', '\n', '"'], "");
        let sanitized_prefix = prefix_guest_path.replace(['\r', '\n', '"'], "");
        let args_joined = app
            .launch_args
            .iter()
            .map(|a| a.replace(['\r', '\n'], ""))
            .collect::<Vec<_>>()
            .join(" ");

        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={sanitized_name}\n\
             Comment=Android PC Windows Application ({id})\n\
             Exec=env WINEPREFIX=\"{sanitized_prefix}\" box64 wine \"{sanitized_exe}\" {args_joined}\n\
             Terminal=false\n\
             Categories=Application;Wine;\n",
            id = app.app_id
        );
        fs::write(&shortcut_path, content)?;
        app.shortcut_created = true;
        self.register_app(app)?;
        Ok(shortcut_path)
    }

    pub fn append_log(&self, app_id: &str, line: &str) {
        if let Ok(mut guard) = self.app_logs.write() {
            let queue = guard.entry(app_id.to_string()).or_default();
            if queue.len() >= MAX_LOG_LINES_PER_APP {
                queue.pop_front();
            }
            queue.push_back(line.to_string());
        }
    }

    pub fn get_logs(&self, app_id: &str) -> Vec<String> {
        self.app_logs
            .read()
            .ok()
            .and_then(|g| g.get(app_id).map(|q| q.iter().cloned().collect()))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_creates_shortcut_and_removes_app() {
        let tmp = std::env::temp_dir().join("androidpc_winapps_test");
        let _ = fs::remove_dir_all(&tmp);
        let rootfs = tmp.join("rootfs");
        fs::create_dir_all(&rootfs).unwrap();

        let lib = WindowsAppLibrary::new(&tmp);
        let profile = WindowsAppProfile {
            app_id: "notepad-plus".into(),
            name: "Notepad++".into(),
            executable_path: "C:\\Program Files\\Notepad++\\notepad++.exe".into(),
            install_dir: "C:\\Program Files\\Notepad++".into(),
            prefix_id: "pfx-default".into(),
            environment_id: "env-ubuntu".into(),
            wine_version: "11.0".into(),
            box64_override: None,
            graphics_backend: GraphicsBackendKind::Software,
            resolution_width: 1280,
            resolution_height: 720,
            launch_args: vec![],
            env_vars: BTreeMap::new(),
            compatibility_notes: "64-bit Win32 editor".into(),
            created_at_epoch_ms: 0,
            last_launched_at_epoch_ms: None,
            shortcut_created: false,
        };

        let saved = lib.register_app(profile).unwrap();
        assert_eq!(saved.app_id, "notepad-plus");
        assert_eq!(lib.list_apps().len(), 1);

        let shortcut = lib
            .create_xfce_shortcut("notepad-plus", &rootfs, "/home/user/.androidpc-prefixes/pfx-default")
            .unwrap();
        assert!(shortcut.is_file());

        lib.append_log("notepad-plus", "Launched via Box64 + Wine 11.0");
        assert_eq!(lib.get_logs("notepad-plus").len(), 1);

        lib.remove_app("notepad-plus", Some(&rootfs)).unwrap();
        assert!(lib.list_apps().is_empty());
        assert!(!shortcut.exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use tracing::info;

use crate::config::{WineConfig, WineSessionState, WineStatusInfo};
use crate::error::WineError;

#[derive(Debug)]
struct SessionEntry {
    config: WineConfig,
    state: WineSessionState,
    active_pids: Vec<u32>,
}

pub struct WineProcessSupervisor {
    sessions: RwLock<HashMap<String, SessionEntry>>,
}

impl Default for WineProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl WineProcessSupervisor {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_session(&self, env_id: &str, config: WineConfig) -> Result<(), WineError> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_| WineError::Supervisor("Lock poisoned".to_string()))?;

        guard.insert(
            env_id.to_string(),
            SessionEntry {
                config,
                state: WineSessionState::Ready,
                active_pids: Vec::new(),
            },
        );
        info!("Registered Wine session for environment '{}'", env_id);
        Ok(())
    }

    pub fn get_session_config(&self, env_id: &str) -> Result<Option<WineConfig>, WineError> {
        let guard = self
            .sessions
            .read()
            .map_err(|_| WineError::Supervisor("Lock poisoned".to_string()))?;
        Ok(guard.get(env_id).map(|e| e.config.clone()))
    }

    pub fn set_session_state(
        &self,
        env_id: &str,
        state: WineSessionState,
    ) -> Result<(), WineError> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_| WineError::Supervisor("Lock poisoned".to_string()))?;
        if let Some(entry) = guard.get_mut(env_id) {
            entry.state = state;
        }
        Ok(())
    }

    pub fn add_wine_pid(&self, env_id: &str, pid: u32) -> Result<(), WineError> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_| WineError::Supervisor("Lock poisoned".to_string()))?;
        if let Some(entry) = guard.get_mut(env_id) {
            if !entry.active_pids.contains(&pid) {
                entry.active_pids.push(pid);
            }
            entry.state = WineSessionState::Running;
        }
        Ok(())
    }

    pub fn clear_wine_pids(&self, env_id: &str) -> Result<(), WineError> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_| WineError::Supervisor("Lock poisoned".to_string()))?;
        if let Some(entry) = guard.get_mut(env_id) {
            entry.active_pids.clear();
            entry.state = WineSessionState::Terminated;
        }
        Ok(())
    }

    /// Inspects host filesystem to check if the Wine prefix is initialized within the guest rootfs.
    pub fn check_prefix_initialized(rootfs_dir: &Path, prefix_rel_path: &Path) -> bool {
        // Strip leading slash if present
        let clean_rel = prefix_rel_path
            .strip_prefix("/")
            .unwrap_or(prefix_rel_path);

        let prefix_dir = rootfs_dir.join(clean_rel);
        let drive_c = prefix_dir.join("drive_c");
        let system_reg = prefix_dir.join("system.reg");

        drive_c.is_dir() && (system_reg.is_file() || prefix_dir.is_dir())
    }

    pub fn get_status(
        &self,
        env_id: &str,
        rootfs_dir: &Path,
    ) -> Result<WineStatusInfo, WineError> {
        let guard = self
            .sessions
            .read()
            .map_err(|_| WineError::Supervisor("Lock poisoned".to_string()))?;

        if let Some(entry) = guard.get(env_id) {
            let prefix_exists =
                Self::check_prefix_initialized(rootfs_dir, &entry.config.prefix_path);
            let wineserver_running = !entry.active_pids.is_empty();

            Ok(WineStatusInfo {
                prefix_path: entry.config.prefix_path.display().to_string(),
                arch: entry.config.arch,
                state: entry.state.clone(),
                prefix_exists,
                wineserver_running,
                active_wine_pids: entry.active_pids.clone(),
                box64_dynarec: entry.config.box64.dynarec_enabled,
            })
        } else {
            // Default placeholder status for unregistered environment
            let default_config = WineConfig::default();
            let prefix_exists =
                Self::check_prefix_initialized(rootfs_dir, &default_config.prefix_path);

            Ok(WineStatusInfo {
                prefix_path: default_config.prefix_path.display().to_string(),
                arch: default_config.arch,
                state: WineSessionState::Uninitialized,
                prefix_exists,
                wineserver_running: false,
                active_wine_pids: Vec::new(),
                box64_dynarec: default_config.box64.dynarec_enabled,
            })
        }
    }
}

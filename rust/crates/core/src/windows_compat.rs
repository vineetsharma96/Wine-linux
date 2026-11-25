use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use androidpc_box64::Box64InstallationStatus;
use androidpc_dependency_manager::{
    DependencyProfile, InstalledDependencyRecord, PrefixDependencyDiagnosis,
};
use androidpc_graphics_backend::{
    GraphicsBackendConfig, GraphicsBackendKind, GraphicsStackOverview,
};
use androidpc_runtime_api::{ProcessCommand, ProcessId};
use androidpc_windows_apps::WindowsAppProfile;
use androidpc_wine::{
    TrackedWindowsProcess, WineArch, WineInstallationStatus, WinePrefixBackupManifest,
    WinePrefixMetadata, WineRuntimeManager,
};

use crate::{CoreError, EngineCore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsCompatibilityOverview {
    pub environment_id: String,
    pub box64: Box64InstallationStatus,
    pub wine: WineInstallationStatus,
    pub graphics: GraphicsStackOverview,
    pub prefix_count: usize,
    pub installed_apps_count: usize,
    pub active_processes_count: usize,
    pub runtime_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsAppLaunchResult {
    pub app_id: String,
    pub prefix_id: String,
    pub command: ProcessCommand,
    pub spawned_pid: Option<ProcessId>,
    pub effective_graphics_backend: GraphicsBackendKind,
    pub dx_choice: androidpc_graphics_backend::DirectXBackendChoice,
    pub warnings: Vec<String>,
}

impl EngineCore {
    fn resolve_env_rootfs(&self, env_id: &str) -> Option<PathBuf> {
        if env_id.is_empty() {
            return None;
        }
        self.environments
            .read()
            .ok()
            .and_then(|envs| envs.get(env_id).map(|m| m.config().rootfs_directory.clone()))
    }

    pub fn get_windows_compatibility_overview(
        &self,
        env_id: Option<&str>,
    ) -> WindowsCompatibilityOverview {
        let eid = env_id.unwrap_or("").to_string();
        let rootfs = self.resolve_env_rootfs(&eid);
        let box64 = self.box64.probe_installation(rootfs.as_deref());
        let wine = self.wine_runtime.probe_installation(rootfs.as_deref());
        let graphics = self.graphics.probe_all(rootfs.as_deref());
        let prefixes = self.wine_prefixes.list_prefixes();
        let apps = self.windows_apps.list_apps();
        let active_processes_count = self
            .wine_runtime
            .list_tracked_processes()
            .iter()
            .filter(|p| p.state == androidpc_wine::WindowsProcessState::Running)
            .count();

        let mut runtime_errors = Vec::new();
        if !box64.installed {
            runtime_errors.push(
                "Box64 v0.4.4 binary is not staged; x86-64 Wine execution requires staging artifacts/box64/box64."
                    .to_string(),
            );
        }
        if !wine.installed {
            runtime_errors.push(
                "Wine 11.0 WoW64 x86-64 runtime is not staged inside the guest or artifacts/wine/."
                    .to_string(),
            );
        }
        if let Some(ref reason) = graphics.fallback_reason {
            runtime_errors.push(reason.clone());
        }

        WindowsCompatibilityOverview {
            environment_id: eid,
            box64,
            wine,
            graphics,
            prefix_count: prefixes.len(),
            installed_apps_count: apps.len(),
            active_processes_count,
            runtime_errors,
        }
    }

    // -------------------------------------------------------------------------
    // Milestone 4.2 — Isolated Wine Prefix Management
    // -------------------------------------------------------------------------

    pub fn list_wine_prefixes(&self) -> Vec<WinePrefixMetadata> {
        self.wine_prefixes.list_prefixes()
    }

    pub fn create_isolated_wine_prefix(
        &self,
        id: &str,
        display_name: &str,
        env_id: &str,
        arch: WineArch,
        windows_version: &str,
        graphics_backend: GraphicsBackendKind,
    ) -> Result<WinePrefixMetadata, CoreError> {
        let rootfs = self.resolve_env_rootfs(env_id);
        let meta = self.wine_prefixes.create_prefix(
            id,
            display_name,
            env_id,
            rootfs.as_deref(),
            arch,
            windows_version,
            graphics_backend,
        )?;
        Ok(meta)
    }

    pub fn configure_wine_prefix(
        &self,
        meta: WinePrefixMetadata,
    ) -> Result<WinePrefixMetadata, CoreError> {
        self.wine_prefixes.save_prefix(&meta)?;
        Ok(meta)
    }

    pub fn clone_wine_prefix(
        &self,
        source_id: &str,
        new_id: &str,
        new_name: &str,
    ) -> Result<WinePrefixMetadata, CoreError> {
        let source = self.wine_prefixes.get_prefix(source_id)?;
        let rootfs = self.resolve_env_rootfs(&source.environment_id);
        let cloned = self
            .wine_prefixes
            .clone_prefix(source_id, new_id, new_name, rootfs.as_deref())?;
        Ok(cloned)
    }

    pub fn backup_wine_prefix(&self, prefix_id: &str) -> Result<WinePrefixBackupManifest, CoreError> {
        let meta = self.wine_prefixes.get_prefix(prefix_id)?;
        let rootfs = self
            .resolve_env_rootfs(&meta.environment_id)
            .ok_or_else(|| {
                CoreError::SubsystemFailure(format!(
                    "Linux environment '{}' is not provisioned on disk",
                    meta.environment_id
                ))
            })?;
        Ok(self.wine_prefixes.backup_prefix(prefix_id, &rootfs)?)
    }

    pub fn list_wine_prefix_backups(&self) -> Vec<WinePrefixBackupManifest> {
        self.wine_prefixes.list_backups()
    }

    pub fn restore_wine_prefix(
        &self,
        backup_id: &str,
        target_prefix_id: &str,
        env_id: &str,
    ) -> Result<WinePrefixMetadata, CoreError> {
        let rootfs = self.resolve_env_rootfs(env_id).ok_or_else(|| {
            CoreError::SubsystemFailure(format!("Linux environment '{env_id}' is not provisioned"))
        })?;
        Ok(self
            .wine_prefixes
            .restore_prefix(backup_id, target_prefix_id, &rootfs)?)
    }

    pub fn repair_wine_prefix(&self, prefix_id: &str) -> Result<Vec<String>, CoreError> {
        let meta = self.wine_prefixes.get_prefix(prefix_id)?;
        let rootfs = self.resolve_env_rootfs(&meta.environment_id).ok_or_else(|| {
            CoreError::SubsystemFailure(format!(
                "Linux environment '{}' is not provisioned",
                meta.environment_id
            ))
        })?;
        Ok(self
            .wine_prefixes
            .repair_prefix_structure(prefix_id, &rootfs)?)
    }

    pub fn delete_wine_prefix(&self, prefix_id: &str) -> Result<(), CoreError> {
        let meta = self.wine_prefixes.get_prefix(prefix_id)?;
        let rootfs = self.resolve_env_rootfs(&meta.environment_id);
        self.wine_prefixes
            .delete_prefix(prefix_id, rootfs.as_deref())?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Milestones 4.3 & 4.4 — Graphics Backend & DXVK/VKD3D-Proton
    // -------------------------------------------------------------------------

    pub fn probe_graphics_backends(&self, env_id: Option<&str>) -> GraphicsStackOverview {
        let rootfs = env_id.and_then(|id| self.resolve_env_rootfs(id));
        self.graphics.probe_all(rootfs.as_deref())
    }

    pub fn configure_graphics_backend(
        &self,
        config: GraphicsBackendConfig,
        env_id: Option<&str>,
    ) -> GraphicsStackOverview {
        let legacy_mode = config.preferred_backend.to_legacy_gpu_mode();
        self.graphics.set_config(config);
        self.set_gpu_acceleration_mode(legacy_mode);
        self.probe_graphics_backends(env_id)
    }

    // -------------------------------------------------------------------------
    // Milestone 4.5 — Windows Dependency Management
    // -------------------------------------------------------------------------

    pub fn list_windows_dependency_profiles(&self) -> Vec<DependencyProfile> {
        self.dependency_manager.list_profiles().to_vec()
    }

    pub fn diagnose_prefix_dependencies(
        &self,
        prefix_id: &str,
        recent_logs: &[String],
    ) -> PrefixDependencyDiagnosis {
        let host_dir = self.wine_prefixes.get_prefix(prefix_id).ok().and_then(|meta| {
            self.resolve_env_rootfs(&meta.environment_id).and_then(|rootfs| {
                androidpc_wine::WinePrefixManager::guest_prefix_host_dir(&rootfs, &meta.prefix_path).ok()
            })
        });
        self.dependency_manager
            .diagnose_prefix(prefix_id, host_dir.as_deref(), recent_logs)
    }

    pub fn record_prefix_dependency(
        &self,
        prefix_id: &str,
        profile_id: &str,
        verified_sha256: Option<String>,
        source_note: &str,
    ) -> Result<InstalledDependencyRecord, CoreError> {
        let record = self
            .dependency_manager
            .record_installed(prefix_id, profile_id, verified_sha256, source_note)
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;

        if let Ok(mut meta) = self.wine_prefixes.get_prefix(prefix_id) {
            if !meta.dependencies.iter().any(|d| d == profile_id) {
                meta.dependencies.push(profile_id.to_string());
                let _ = self.wine_prefixes.save_prefix(&meta);
            }
        }
        Ok(record)
    }

    // -------------------------------------------------------------------------
    // Milestones 4.6 & 4.8 — Windows Application Manager & Process Supervision
    // -------------------------------------------------------------------------

    pub fn list_windows_apps(&self) -> Vec<WindowsAppProfile> {
        self.windows_apps.list_apps()
    }

    pub fn register_windows_app(
        &self,
        profile: WindowsAppProfile,
    ) -> Result<WindowsAppProfile, CoreError> {
        let saved = self
            .windows_apps
            .register_app(profile)
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;

        if let Ok(mut prefix) = self.wine_prefixes.get_prefix(&saved.prefix_id) {
            if !prefix.installed_apps.iter().any(|id| id == &saved.app_id) {
                prefix.installed_apps.push(saved.app_id.clone());
                let _ = self.wine_prefixes.save_prefix(&prefix);
            }
        }
        Ok(saved)
    }

    pub fn remove_windows_app(&self, app_id: &str) -> Result<(), CoreError> {
        let existing = self.windows_apps.get_app(app_id).ok();
        let rootfs = existing
            .as_ref()
            .and_then(|a| self.resolve_env_rootfs(&a.environment_id));
        self.windows_apps
            .remove_app(app_id, rootfs.as_deref())
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;

        if let Some(app) = existing {
            if let Ok(mut prefix) = self.wine_prefixes.get_prefix(&app.prefix_id) {
                prefix.installed_apps.retain(|id| id != app_id);
                let _ = self.wine_prefixes.save_prefix(&prefix);
            }
        }
        Ok(())
    }

    pub fn create_windows_app_shortcut(&self, app_id: &str) -> Result<String, CoreError> {
        let app = self
            .windows_apps
            .get_app(app_id)
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;
        let prefix = self.wine_prefixes.get_prefix(&app.prefix_id)?;
        let rootfs = self.resolve_env_rootfs(&app.environment_id).ok_or_else(|| {
            CoreError::SubsystemFailure(format!(
                "Linux environment '{}' is not provisioned",
                app.environment_id
            ))
        })?;
        let path = self
            .windows_apps
            .create_xfce_shortcut(app_id, &rootfs, &prefix.prefix_path)
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;
        Ok(path.display().to_string())
    }

    pub fn launch_windows_app(&self, app_id: &str) -> Result<WindowsAppLaunchResult, CoreError> {
        let app = self
            .windows_apps
            .get_app(app_id)
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;
        let prefix = self.wine_prefixes.get_prefix(&app.prefix_id)?;
        let rootfs = self.resolve_env_rootfs(&app.environment_id);

        let (graphics_env, dx_plan, overview) = self
            .graphics
            .build_effective_guest_env(rootfs.as_deref(), Some(app.graphics_backend))
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;

        let is_msi = app.executable_path.to_ascii_lowercase().ends_with(".msi");
        let extra_env: Vec<(String, String)> = app
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let cmd = WineRuntimeManager::build_prefix_launch_command(
            &prefix,
            &app.executable_path,
            &app.launch_args,
            if app.install_dir.trim().is_empty() {
                None
            } else {
                Some(app.install_dir.as_str())
            },
            Some((app.resolution_width, app.resolution_height)),
            is_msi,
            &graphics_env,
            &dx_plan.wine_dll_overrides,
            &extra_env,
        );

        let mut warnings = dx_plan.warnings.clone();
        if let Some(ref reason) = overview.fallback_reason {
            warnings.push(reason.clone());
        }

        let mut spawned_pid = None;
        if let Ok(mut envs) = self.environments.write() {
            if let Some(env_mgr) = envs.get_mut(&app.environment_id) {
                let mut guest_argv = vec![cmd.program.clone()];
                guest_argv.extend(cmd.args.clone());
                match env_mgr.execute_guest_command_with_env(&guest_argv, &cmd.env) {
                    Ok(pid) => {
                        spawned_pid = Some(pid);
                        self.windows_apps.append_log(
                            app_id,
                            &format!(
                                "Launched PID {pid} in prefix '{}' ({:?}, DX={:?})",
                                prefix.id, overview.effective_backend, dx_plan.effective_choice
                            ),
                        );
                    }
                    Err(e) => {
                        let msg = format!("Guest dispatch deferred/failed: {e}");
                        warnings.push(msg.clone());
                        self.windows_apps.append_log(app_id, &msg);
                    }
                }
            } else {
                let msg = format!(
                    "Linux environment '{}' is not running; launch command prepared only.",
                    app.environment_id
                );
                warnings.push(msg.clone());
                self.windows_apps.append_log(app_id, &msg);
            }
        }

        let _ = self.wine_prefixes.touch_last_used(&prefix.id);
        let _ = self.windows_apps.mark_launched(app_id);
        let _ = self.wine_runtime.register_tracked_process(
            &app.app_id,
            &app.name,
            &prefix.id,
            &app.environment_id,
            spawned_pid,
        );

        Ok(WindowsAppLaunchResult {
            app_id: app.app_id,
            prefix_id: prefix.id,
            command: cmd,
            spawned_pid,
            effective_graphics_backend: overview.effective_backend,
            dx_choice: dx_plan.effective_choice,
            warnings,
        })
    }

    /// Performs scoped, bounded termination of a tracked Windows application
    /// (`wineserver -k` on the app's isolated `WINEPREFIX` only — never broad kills).
    pub fn terminate_windows_app(
        &self,
        app_id: &str,
        force: bool,
    ) -> Result<Option<TrackedWindowsProcess>, CoreError> {
        let app = self
            .windows_apps
            .get_app(app_id)
            .map_err(|e| CoreError::SubsystemFailure(e.to_string()))?;
        let prefix = self.wine_prefixes.get_prefix(&app.prefix_id)?;
        let shutdown_cmd = WineRuntimeManager::build_scoped_prefix_shutdown(&prefix, force);

        if let Ok(mut envs) = self.environments.write() {
            if let Some(env_mgr) = envs.get_mut(&app.environment_id) {
                let mut argv = vec![shutdown_cmd.program.clone()];
                argv.extend(shutdown_cmd.args.clone());
                let _ = env_mgr.execute_guest_command_with_env(&argv, &shutdown_cmd.env);
            }
        }

        let reason = if force {
            "Scoped forced termination (wineserver -k9 on isolated prefix)"
        } else {
            "Scoped graceful termination (wineserver -k on isolated prefix)"
        };
        self.windows_apps.append_log(app_id, reason);
        Ok(self
            .wine_runtime
            .mark_process_terminated(app_id, reason, Some(0))?)
    }

    pub fn list_windows_processes(&self) -> Vec<TrackedWindowsProcess> {
        self.wine_runtime.list_tracked_processes()
    }

    pub fn get_windows_app_logs(&self, app_id: &str) -> Vec<String> {
        self.windows_apps.get_logs(app_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use androidpc_configuration::EngineConfig;
    use std::collections::BTreeMap;

    #[test]
    fn end_to_end_phase4_prefix_app_graphics_and_dependency_orchestration() {
        let tmp = std::env::temp_dir().join("androidpc_core_phase4_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let cfg = EngineConfig {
            base_data_dir: tmp.clone(),
            runtime_artifacts_dir: tmp.join("artifacts"),
            temporary_dir: tmp.join("tmp"),
            ..EngineConfig::default()
        };
        let core = EngineCore::new(cfg).unwrap();
        core.provision_environment("env-ubuntu", "ubuntu").unwrap();

        let prefix = core
            .create_isolated_wine_prefix(
                "pfx-apps",
                "Productivity Prefix",
                "env-ubuntu",
                WineArch::WoW64,
                "win10",
                GraphicsBackendKind::Software,
            )
            .unwrap();
        assert_eq!(prefix.id, "pfx-apps");

        let app = core
            .register_windows_app(WindowsAppProfile {
                app_id: "win-calc".into(),
                name: "Calculator".into(),
                executable_path: "C:\\windows\\system32\\calc.exe".into(),
                install_dir: "C:\\windows\\system32".into(),
                prefix_id: "pfx-apps".into(),
                environment_id: "env-ubuntu".into(),
                wine_version: "11.0".into(),
                box64_override: None,
                graphics_backend: GraphicsBackendKind::Software,
                resolution_width: 1280,
                resolution_height: 720,
                launch_args: vec![],
                env_vars: BTreeMap::new(),
                compatibility_notes: "Built-in Wine test utility".into(),
                created_at_epoch_ms: 0,
                last_launched_at_epoch_ms: None,
                shortcut_created: false,
            })
            .unwrap();
        assert_eq!(app.app_id, "win-calc");

        let overview = core.get_windows_compatibility_overview(Some("env-ubuntu"));
        assert_eq!(overview.prefix_count, 1);
        assert_eq!(overview.installed_apps_count, 1);
        assert_eq!(
            overview.graphics.effective_backend,
            GraphicsBackendKind::Software
        );

        let shortcut = core.create_windows_app_shortcut("win-calc").unwrap();
        assert!(std::path::Path::new(&shortcut).is_file());

        let launch = core.launch_windows_app("win-calc").unwrap();
        assert_eq!(launch.command.program, "box64");
        assert!(launch.command.args.contains(&"wine".to_string()));
        assert_eq!(core.list_windows_processes().len(), 1);

        let terminated = core.terminate_windows_app("win-calc", false).unwrap();
        assert!(terminated.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

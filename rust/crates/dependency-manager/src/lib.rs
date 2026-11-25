use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DependencyError {
    #[error("Dependency profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Security check failed: {0}")]
    Security(#[from] androidpc_security::SecurityError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMechanism {
    /// Bundled or open-source redistributable with verified SHA-256 checksum.
    VerifiedOpenSourceArtifact,
    /// Native Wine tool (`wineboot`, `reg`, `winecfg`, Wine Mono/Gecko MSI).
    WineNativeTool,
    /// License-restricted proprietary redistributable (e.g., VC++ Runtime, .NET)
    /// — never bundled; user supplies official installer and verifies checksum.
    UserSuppliedOfficialInstaller,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyProfile {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub category: String,
    pub license: String,
    pub mechanism: InstallMechanism,
    pub official_source_url: String,
    pub expected_dlls: Vec<String>,
    pub conflicts_with: Vec<String>,
    pub recovery_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledDependencyRecord {
    pub profile_id: String,
    pub version: String,
    pub prefix_id: String,
    pub installed_at_epoch_ms: u64,
    pub verified_sha256: Option<String>,
    pub source_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    pub profile_a: String,
    pub profile_b: String,
    pub reason: String,
    pub recovery_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixDependencyDiagnosis {
    pub prefix_id: String,
    pub installed_records: Vec<InstalledDependencyRecord>,
    pub present_dlls: Vec<String>,
    pub missing_dlls: Vec<String>,
    pub recommended_profiles: Vec<String>,
    pub conflicts: Vec<DependencyConflict>,
    pub notes: Vec<String>,
}

/// Manages Windows runtime dependencies, missing DLL diagnostics, installation
/// tracking, and conflict detection across isolated Wine prefixes.
///
/// Security policy:
/// - Never blindly downloads and executes third-party shell scripts (`curl | sh`).
/// - License-restricted Microsoft redistributables are NEVER bundled; users install
///   official installers via Wine with SHA-256 verification.
pub struct DependencyManager {
    state_dir: PathBuf,
    catalog: Vec<DependencyProfile>,
}

impl DependencyManager {
    pub fn new(base_data_dir: &Path) -> Self {
        Self {
            state_dir: base_data_dir.join("windows_dependencies"),
            catalog: Self::default_catalog(),
        }
    }

    fn now_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn default_catalog() -> Vec<DependencyProfile> {
        vec![
            DependencyProfile {
                id: "wine-mono".to_string(),
                display_name: "Wine Mono (.NET Open-Source Runtime)".to_string(),
                version: "9.4.0".to_string(),
                category: "dotnet".to_string(),
                license: "MIT / LGPL / GPL-2.0".to_string(),
                mechanism: InstallMechanism::WineNativeTool,
                official_source_url: "https://dl.winehq.org/wine/wine-mono/".to_string(),
                expected_dlls: vec!["mscoree.dll".to_string()],
                conflicts_with: vec!["dotnet-desktop-8".to_string()],
                recovery_hint: "Install official Wine Mono MSI via `wine msiexec /i` or remove conflicting .NET overrides.".to_string(),
            },
            DependencyProfile {
                id: "dxvk-2.7.1".to_string(),
                display_name: "DXVK (Direct3D 8/9/10/11 over Vulkan)".to_string(),
                version: "2.7.1".to_string(),
                category: "graphics".to_string(),
                license: "zlib".to_string(),
                mechanism: InstallMechanism::VerifiedOpenSourceArtifact,
                official_source_url: "https://github.com/doitsujin/dxvk/releases".to_string(),
                expected_dlls: vec![
                    "d3d9.dll".to_string(),
                    "d3d10core.dll".to_string(),
                    "d3d11.dll".to_string(),
                    "dxgi.dll".to_string(),
                ],
                conflicts_with: vec!["wined3d-builtin".to_string()],
                recovery_hint: "Requires operational Vulkan 1.3+ backend (Mesa Turnip). Switch to WineD3D OpenGL fallback if Vulkan is unavailable.".to_string(),
            },
            DependencyProfile {
                id: "vkd3d-proton-3.1".to_string(),
                display_name: "VKD3D-Proton (Direct3D 12 over Vulkan)".to_string(),
                version: "3.1".to_string(),
                category: "graphics".to_string(),
                license: "LGPL-2.1".to_string(),
                mechanism: InstallMechanism::VerifiedOpenSourceArtifact,
                official_source_url: "https://github.com/HansKristian-Work/vkd3d-proton/releases".to_string(),
                expected_dlls: vec!["d3d12.dll".to_string(), "d3d12core.dll".to_string()],
                conflicts_with: vec![],
                recovery_hint: "Requires Turnip Vulkan 1.3+ with dynamic_rendering, descriptor_indexing, buffer_device_address, timeline_semaphore, and robustness2.".to_string(),
            },
            DependencyProfile {
                id: "vcrun2015-2022".to_string(),
                display_name: "Visual C++ 2015–2022 Redistributable (x64)".to_string(),
                version: "14.42".to_string(),
                category: "cpp_runtime".to_string(),
                license: "Microsoft Software License (User-Supplied)".to_string(),
                mechanism: InstallMechanism::UserSuppliedOfficialInstaller,
                official_source_url: "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist".to_string(),
                expected_dlls: vec![
                    "vcruntime140.dll".to_string(),
                    "vcruntime140_1.dll".to_string(),
                    "msvcp140.dll".to_string(),
                    "ucrtbase.dll".to_string(),
                ],
                conflicts_with: vec![],
                recovery_hint: "Download official vc_redist.x64.exe from Microsoft, verify SHA-256, and install into the isolated prefix via Windows Application Manager.".to_string(),
            },
            DependencyProfile {
                id: "dotnet-desktop-8".to_string(),
                display_name: ".NET 8 Desktop Runtime (x64)".to_string(),
                version: "8.0 LTS".to_string(),
                category: "dotnet".to_string(),
                license: "MIT / Microsoft Installer (User-Supplied)".to_string(),
                mechanism: InstallMechanism::UserSuppliedOfficialInstaller,
                official_source_url: "https://dotnet.microsoft.com/en-us/download/dotnet/8.0".to_string(),
                expected_dlls: vec!["hostfxr.dll".to_string()],
                conflicts_with: vec!["wine-mono".to_string()],
                recovery_hint: "Disable Wine Mono in the target prefix before installing official .NET 8 Desktop Runtime.".to_string(),
            },
            DependencyProfile {
                id: "corefonts".to_string(),
                display_name: "Open-Source Metric-Compatible Fonts (Liberation / Carlito)".to_string(),
                version: "2.1.5".to_string(),
                category: "fonts".to_string(),
                license: "SIL OFL 1.1".to_string(),
                mechanism: InstallMechanism::VerifiedOpenSourceArtifact,
                official_source_url: "https://github.com/liberationfonts/liberation-fonts".to_string(),
                expected_dlls: vec![],
                conflicts_with: vec![],
                recovery_hint: "Install `fonts-liberation` inside the Ubuntu/Debian ARM64 guest via APT to fix missing UI glyphs in Wine apps.".to_string(),
            },
        ]
    }

    pub fn list_profiles(&self) -> &[DependencyProfile] {
        &self.catalog
    }

    fn prefix_records_path(&self, prefix_id: &str) -> PathBuf {
        self.state_dir.join(format!("{prefix_id}.json"))
    }

    pub fn list_installed(&self, prefix_id: &str) -> Vec<InstalledDependencyRecord> {
        let path = self.prefix_records_path(prefix_id);
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn record_installed(
        &self,
        prefix_id: &str,
        profile_id: &str,
        verified_sha256: Option<String>,
        source_note: &str,
    ) -> Result<InstalledDependencyRecord, DependencyError> {
        let profile = self
            .catalog
            .iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| DependencyError::ProfileNotFound(profile_id.to_string()))?;

        fs::create_dir_all(&self.state_dir)?;
        let mut records = self.list_installed(prefix_id);
        records.retain(|r| r.profile_id != profile_id);

        let rec = InstalledDependencyRecord {
            profile_id: profile.id.clone(),
            version: profile.version.clone(),
            prefix_id: prefix_id.to_string(),
            installed_at_epoch_ms: Self::now_epoch_ms(),
            verified_sha256,
            source_note: source_note.to_string(),
        };
        records.push(rec.clone());
        fs::write(
            self.prefix_records_path(prefix_id),
            serde_json::to_vec_pretty(&records)?,
        )?;
        Ok(rec)
    }

    /// Scans a Wine prefix's `system32` / `syswow64` directories plus optional Wine error logs
    /// for missing DLLs, installed profiles, recommendations, and conflicts.
    pub fn diagnose_prefix(
        &self,
        prefix_id: &str,
        prefix_host_dir: Option<&Path>,
        recent_logs: &[String],
    ) -> PrefixDependencyDiagnosis {
        let installed_records = self.list_installed(prefix_id);
        let installed_ids: HashSet<&str> = installed_records
            .iter()
            .map(|r| r.profile_id.as_str())
            .collect();

        let mut present_dlls = HashSet::new();
        if let Some(dir) = prefix_host_dir {
            for sub in ["drive_c/windows/system32", "drive_c/windows/syswow64"] {
                if let Ok(entries) = fs::read_dir(dir.join(sub)) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.to_ascii_lowercase().ends_with(".dll") {
                                present_dlls.insert(name.to_ascii_lowercase());
                            }
                        }
                    }
                }
            }
        }

        // Extract DLL names referenced in Wine `err:module:import_dll Library <x.dll> ... not found` logs
        let mut missing_dlls = HashSet::new();
        for line in recent_logs {
            let lower = line.to_ascii_lowercase();
            if lower.contains("not found") || lower.contains("failed to load") {
                for token in line.split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | ',' | '(' | ')')) {
                    let t = token.trim().to_ascii_lowercase();
                    if t.ends_with(".dll") && !present_dlls.contains(&t) {
                        missing_dlls.insert(t);
                    }
                }
            }
        }

        // Map missing DLLs to recommended profiles
        let mut recommended = Vec::new();
        let dll_to_profile: BTreeMap<&str, &str> = self
            .catalog
            .iter()
            .flat_map(|p| p.expected_dlls.iter().map(move |d| (d.as_str(), p.id.as_str())))
            .collect();

        for dll in &missing_dlls {
            if let Some(prof_id) = dll_to_profile.get(dll.as_str()) {
                if !recommended.iter().any(|r| r == prof_id) {
                    recommended.push((*prof_id).to_string());
                }
            }
        }

        // Detect conflicts among installed records
        let mut conflicts = Vec::new();
        for profile in &self.catalog {
            if !installed_ids.contains(profile.id.as_str()) {
                continue;
            }
            for conflicting in &profile.conflicts_with {
                if installed_ids.contains(conflicting.as_str()) && profile.id < *conflicting {
                    conflicts.push(DependencyConflict {
                        profile_a: profile.id.clone(),
                        profile_b: conflicting.clone(),
                        reason: format!(
                            "'{}' and '{}' override overlapping runtime DLLs inside the same prefix.",
                            profile.display_name, conflicting
                        ),
                        recovery_action: profile.recovery_hint.clone(),
                    });
                }
            }
        }

        let mut notes = Vec::new();
        if prefix_host_dir.is_none() {
            notes.push("Prefix directory is not mounted/initialized on disk yet; showing tracked metadata only.".to_string());
        }
        notes.push(
            "Proprietary redistributables (VC++ Runtime, .NET) are never bundled; supply official installers with SHA-256 verification."
                .to_string(),
        );

        let mut present_sorted: Vec<String> = present_dlls.into_iter().collect();
        present_sorted.sort();
        let mut missing_sorted: Vec<String> = missing_dlls.into_iter().collect();
        missing_sorted.sort();

        PrefixDependencyDiagnosis {
            prefix_id: prefix_id.to_string(),
            installed_records,
            present_dlls: present_sorted,
            missing_dlls: missing_sorted,
            recommended_profiles: recommended,
            conflicts,
            notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnoses_missing_vcruntime_and_detects_dotnet_conflict() {
        let tmp = std::env::temp_dir().join("androidpc_dep_mgr_test");
        let _ = fs::remove_dir_all(&tmp);
        let mgr = DependencyManager::new(&tmp);

        mgr.record_installed("pfx-1", "wine-mono", None, "wine native")
            .unwrap();
        mgr.record_installed("pfx-1", "dotnet-desktop-8", None, "user installer")
            .unwrap();

        let logs = vec![
            "0024:err:module:import_dll Library VCRUNTIME140.dll (which is needed by L\"C:\\\\app.exe\") not found".to_string(),
        ];

        let diag = mgr.diagnose_prefix("pfx-1", None, &logs);
        assert!(diag.missing_dlls.contains(&"vcruntime140.dll".to_string()));
        assert!(diag.recommended_profiles.contains(&"vcrun2015-2022".to_string()));
        assert_eq!(diag.conflicts.len(), 1);

        let _ = fs::remove_dir_all(&tmp);
    }
}

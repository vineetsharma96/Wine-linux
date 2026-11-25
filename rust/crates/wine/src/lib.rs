pub mod config;
pub mod error;
pub mod launcher;
pub mod manager;
pub mod prefix;
pub mod process;
pub mod registry;

pub use config::{Box64Config, WineArch, WineConfig, WineSessionState, WineStatusInfo};
pub use error::WineError;
pub use launcher::WineCommandBuilder;
pub use manager::{
    TrackedWindowsProcess, WineInstallationStatus, WineRuntimeManager, WindowsProcessState,
    WINE_LICENSE, WINE_PINNED_VERSION, WINE_SELECTED_ARCHITECTURE, WINE_UPSTREAM_URL,
};
pub use prefix::{WinePrefixBackupManifest, WinePrefixManager, WinePrefixMetadata};
pub use process::WineProcessSupervisor;
pub use registry::{
    inspect_windows_binary, inspect_windows_binary_bytes, DllOverrideMode,
    WindowsBinaryInspection, WindowsBinaryKind, WineRegistryManager,
};

#[cfg(test)]
mod tests {
    use super::*;
    use androidpc_graphics_backend::GraphicsBackendKind;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_box64_env_generation() {
        let box64 = Box64Config::default();
        let env_vars = box64.to_env_vars();

        let map: std::collections::HashMap<_, _> = env_vars.into_iter().collect();
        assert_eq!(map.get("BOX64_DYNAREC").map(String::as_str), Some("1"));
        assert_eq!(map.get("BOX64_LOG").map(String::as_str), Some("0"));
        assert_eq!(map.get("BOX64_FASTNAN").map(String::as_str), Some("1"));
        assert_eq!(map.get("BOX64_FASTROUND").map(String::as_str), Some("1"));
        assert_eq!(map.get("BOX64_BOGUS_AVX").map(String::as_str), Some("1"));
        assert!(map.get("BOX64_EMULATED_LIBS").unwrap().contains("libpulse.so.0"));
    }

    #[test]
    fn test_wine_env_generation() {
        let config = WineConfig::default();
        let env_vars = config.to_env_vars();

        let map: std::collections::HashMap<_, _> = env_vars.into_iter().collect();
        assert_eq!(map.get("WINEPREFIX").map(String::as_str), Some("/home/user/.wine"));
        assert_eq!(map.get("WINEARCH").map(String::as_str), Some("win64"));
        assert_eq!(map.get("DISPLAY").map(String::as_str), Some(":1"));
        assert_eq!(map.get("PULSE_SERVER").map(String::as_str), Some("127.0.0.1:4713"));
        assert_eq!(map.get("BOX64_DYNAREC").map(String::as_str), Some("1"));
    }

    #[test]
    fn test_wineboot_command_builder() {
        let config = WineConfig::default();
        let cmd = WineCommandBuilder::build_wineboot_command(&config);

        assert_eq!(cmd.program, "box64");
        assert_eq!(cmd.args, vec!["wine", "wineboot", "-u"]);
        assert!(cmd.env.iter().any(|(k, v)| k == "WINEPREFIX" && v == "/home/user/.wine"));
    }

    #[test]
    fn test_launch_command_builder() {
        let config = WineConfig::default();
        let args = vec!["/arg1".to_string(), "--test".to_string()];
        let cmd = WineCommandBuilder::build_launch_command(&config, "C:\\Games\\Doom.exe", &args);

        assert_eq!(cmd.program, "box64");
        assert_eq!(cmd.args, vec!["wine", "C:\\Games\\Doom.exe", "/arg1", "--test"]);
        assert!(cmd.env.iter().any(|(k, v)| k == "BOX64_DYNAREC" && v == "1"));
    }

    #[test]
    fn test_wineserver_kill_command() {
        let config = WineConfig::default();
        let cmd = WineCommandBuilder::build_wineserver_kill_command(&config);

        assert_eq!(cmd.program, "wineserver");
        assert_eq!(cmd.args, vec!["-k"]);
        assert!(cmd.env.iter().any(|(k, v)| k == "WINEPREFIX" && v == "/home/user/.wine"));
    }

    #[test]
    fn test_path_conversions() {
        let prefix = PathBuf::from("/home/user/.wine");
        let unix_path = PathBuf::from("/home/user/.wine/drive_c/windows/system32/notepad.exe");

        let win_path = WineCommandBuilder::to_windows_path(&unix_path, &prefix);
        assert_eq!(win_path, Some("C:\\windows\\system32\\notepad.exe".to_string()));

        let roundtrip = WineCommandBuilder::to_unix_path("C:\\windows\\system32\\notepad.exe", &prefix);
        assert_eq!(roundtrip, Some(unix_path));

        let linux_file = Path::new("/etc/hosts");
        let z_path = WineCommandBuilder::to_windows_path(linux_file, &prefix);
        assert_eq!(z_path, Some("Z:\\etc\\hosts".to_string()));

        let z_roundtrip = WineCommandBuilder::to_unix_path("Z:\\etc\\hosts", &prefix);
        assert_eq!(z_roundtrip, Some(PathBuf::from("/etc/hosts")));
    }

    #[test]
    fn test_wine_supervisor_lifecycle() {
        let supervisor = WineProcessSupervisor::new();
        let config = WineConfig::default();

        supervisor.register_session("env-ubuntu", config).unwrap();
        supervisor.add_wine_pid("env-ubuntu", 1234).unwrap();

        let tmp_rootfs = std::env::temp_dir().join("androidpc_wine_test_rootfs");
        let _ = std::fs::create_dir_all(tmp_rootfs.join("home/user/.wine/drive_c"));

        let status = supervisor.get_status("env-ubuntu", &tmp_rootfs).unwrap();
        assert_eq!(status.state, WineSessionState::Running);
        assert!(status.prefix_exists);
        assert!(status.wineserver_running);
        assert_eq!(status.active_wine_pids, vec![1234]);

        supervisor.clear_wine_pids("env-ubuntu").unwrap();
        let status_after = supervisor.get_status("env-ubuntu", &tmp_rootfs).unwrap();
        assert_eq!(status_after.state, WineSessionState::Terminated);
        assert!(!status_after.wineserver_running);

        let _ = std::fs::remove_dir_all(tmp_rootfs);
    }

    #[test]
    fn test_pe_and_msi_inspection() {
        let mut pe64 = vec![0u8; 0x100];
        pe64[0] = b'M';
        pe64[1] = b'Z';
        pe64[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        pe64[0x80..0x84].copy_from_slice(b"PE\0\0");
        pe64[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        let res = inspect_windows_binary_bytes("app.exe".into(), &pe64);
        assert_eq!(res.kind, WindowsBinaryKind::Pe64X86_64);
        assert!(res.supported_by_wine_wow64);

        let msi = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0, 0];
        let msi_res = inspect_windows_binary_bytes("setup.msi".into(), &msi);
        assert_eq!(msi_res.kind, WindowsBinaryKind::MsiInstaller);
        assert!(msi_res.supported_by_wine_wow64);
    }

    #[test]
    fn test_prefix_lifecycle_clone_backup_restore() {
        let tmp = std::env::temp_dir().join("androidpc_wine_prefix_lifecycle_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let rootfs = tmp.join("rootfs");
        std::fs::create_dir_all(&rootfs).unwrap();

        let mgr = WinePrefixManager::new(&tmp);
        let created = mgr
            .create_prefix(
                "pfx-work",
                "Work Prefix",
                "env-ubuntu",
                Some(&rootfs),
                WineArch::WoW64,
                "win10",
                GraphicsBackendKind::Software,
            )
            .unwrap();
        assert_eq!(created.id, "pfx-work");

        let marker = rootfs.join("home/user/.androidpc-prefixes/pfx-work/drive_c/marker.txt");
        std::fs::write(&marker, b"isolated-prefix").unwrap();

        let cloned = mgr
            .clone_prefix("pfx-work", "pfx-clone", "Work Clone", Some(&rootfs))
            .unwrap();
        assert_eq!(cloned.id, "pfx-clone");
        assert!(rootfs.join("home/user/.androidpc-prefixes/pfx-clone/drive_c/marker.txt").is_file());

        let backup = mgr.backup_prefix("pfx-work", &rootfs).unwrap();
        assert_eq!(backup.sha256.len(), 64);

        let restored = mgr
            .restore_prefix(&backup.backup_id, "pfx-restored", &rootfs)
            .unwrap();
        assert_eq!(restored.id, "pfx-restored");
        assert_eq!(
            std::fs::read_to_string(
                rootfs.join("home/user/.androidpc-prefixes/pfx-restored/drive_c/marker.txt")
            )
            .unwrap(),
            "isolated-prefix"
        );

        mgr.delete_prefix("pfx-work", Some(&rootfs)).unwrap();
        assert!(mgr.get_prefix("pfx-work").is_err());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

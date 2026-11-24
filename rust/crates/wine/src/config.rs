use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WineArch {
    Win32,
    Win64,
    WoW64,
}

impl Default for WineArch {
    fn default() -> Self {
        Self::WoW64
    }
}

impl WineArch {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Win32 => "win32",
            Self::Win64 => "win64",
            Self::WoW64 => "win64",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box64Config {
    pub dynarec_enabled: bool,
    pub log_level: u8,
    pub fast_nan: bool,
    pub fast_round: bool,
    pub bogus_avx: bool,
    pub prefer_wrapped: bool,
    pub safe_flags: bool,
    pub emulated_libs: Vec<String>,
}

impl Default for Box64Config {
    fn default() -> Self {
        Self {
            dynarec_enabled: true,
            log_level: 0,
            fast_nan: true,
            fast_round: true,
            bogus_avx: true,
            prefer_wrapped: true,
            safe_flags: true,
            emulated_libs: vec!["libpulse.so.0".to_string(), "libvulkan.so.1".to_string()],
        }
    }
}

impl Box64Config {
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        vec![
            (
                "BOX64_DYNAREC".to_string(),
                if self.dynarec_enabled { "1".to_string() } else { "0".to_string() },
            ),
            ("BOX64_LOG".to_string(), self.log_level.to_string()),
            (
                "BOX64_FASTNAN".to_string(),
                if self.fast_nan { "1".to_string() } else { "0".to_string() },
            ),
            (
                "BOX64_FASTROUND".to_string(),
                if self.fast_round { "1".to_string() } else { "0".to_string() },
            ),
            (
                "BOX64_BOGUS_AVX".to_string(),
                if self.bogus_avx { "1".to_string() } else { "0".to_string() },
            ),
            (
                "BOX64_PREFER_WRAPPED".to_string(),
                if self.prefer_wrapped { "1".to_string() } else { "0".to_string() },
            ),
            (
                "BOX64_SAFEFLAGS".to_string(),
                if self.safe_flags { "1".to_string() } else { "0".to_string() },
            ),
            ("BOX64_EMULATED_LIBS".to_string(), self.emulated_libs.join(":")),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineConfig {
    pub prefix_path: PathBuf,
    pub arch: WineArch,
    pub windows_version: String,
    pub dxvk_enabled: bool,
    pub esync_enabled: bool,
    pub debug_channels: String,
    pub display: String,
    pub pulse_server: String,
    pub box64: Box64Config,
}

impl Default for WineConfig {
    fn default() -> Self {
        Self {
            prefix_path: PathBuf::from("/home/user/.wine"),
            arch: WineArch::default(),
            windows_version: "win10".to_string(),
            dxvk_enabled: true,
            esync_enabled: false, // Rootless Android kernel might lack eventfd support for esync
            debug_channels: "fixme-all,err-all".to_string(),
            display: ":1".to_string(),
            pulse_server: "127.0.0.1:4713".to_string(),
            box64: Box64Config::default(),
        }
    }
}

impl WineConfig {
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        let mut vars = self.box64.to_env_vars();
        vars.push(("WINEPREFIX".to_string(), self.prefix_path.display().to_string()));
        vars.push(("WINEARCH".to_string(), self.arch.as_str().to_string()));
        vars.push(("WINEDEBUG".to_string(), self.debug_channels.clone()));
        vars.push(("WINEDLLOVERRIDES".to_string(), "mscoree,mshtml=".to_string()));
        vars.push(("DISPLAY".to_string(), self.display.clone()));
        vars.push(("PULSE_SERVER".to_string(), self.pulse_server.clone()));
        if self.esync_enabled {
            vars.push(("WINEESYNC".to_string(), "1".to_string()));
        }
        vars
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WineSessionState {
    Uninitialized,
    InitializingPrefix,
    Ready,
    Running,
    Terminated,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineStatusInfo {
    pub prefix_path: String,
    pub arch: WineArch,
    pub state: WineSessionState,
    pub prefix_exists: bool,
    pub wineserver_running: bool,
    pub active_wine_pids: Vec<u32>,
    pub box64_dynarec: bool,
}

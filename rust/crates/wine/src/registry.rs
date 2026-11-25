use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::WineError;

pub const PE_MACHINE_I386: u16 = 0x014C;
pub const PE_MACHINE_AMD64: u16 = 0x8664;
pub const PE_MACHINE_ARM64: u16 = 0xAA64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowsBinaryKind {
    Pe64X86_64,
    Pe32I386WoW64,
    Pe64Arm64,
    MsiInstaller,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsBinaryInspection {
    pub path: String,
    pub kind: WindowsBinaryKind,
    pub machine_code: Option<u16>,
    pub supported_by_wine_wow64: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DllOverrideMode {
    Native,
    Builtin,
    NativeThenBuiltin,
    BuiltinThenNative,
    Disabled,
}

impl DllOverrideMode {
    pub fn as_wine_token(&self) -> &'static str {
        match self {
            Self::Native => "n",
            Self::Builtin => "b",
            Self::NativeThenBuiltin => "n,b",
            Self::BuiltinThenNative => "b,n",
            Self::Disabled => "",
        }
    }
}

/// Inspects a Windows executable (`.exe`) or installer (`.msi`) header in pure Rust
/// to determine architecture compatibility before invoking Wine + Box64.
pub fn inspect_windows_binary(path: &Path) -> Result<WindowsBinaryInspection, WineError> {
    if !path.is_file() {
        return Err(WineError::BinaryNotFound(path.display().to_string()));
    }
    let mut file = File::open(path)?;
    let mut header = vec![0u8; 4096];
    let n = file.read(&mut header)?;
    header.truncate(n);
    Ok(inspect_windows_binary_bytes(path.display().to_string(), &header))
}

pub fn inspect_windows_binary_bytes(path_label: String, bytes: &[u8]) -> WindowsBinaryInspection {
    // OLE2 Compound File Binary (MSI installer magic: D0 CF 11 E0 A1 B1 1A E1)
    const MSI_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    if bytes.len() >= 8 && bytes[..8] == MSI_MAGIC {
        return WindowsBinaryInspection {
            path: path_label,
            kind: WindowsBinaryKind::MsiInstaller,
            machine_code: None,
            supported_by_wine_wow64: true,
            detail: "Windows MSI installer package (OLE2 CFBF); execute via `wine msiexec /i`.".to_string(),
        };
    }

    // DOS MZ header + PE signature
    if bytes.len() >= 0x40 && &bytes[0..2] == b"MZ" {
        let pe_offset = u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]]) as usize;
        if pe_offset + 6 <= bytes.len() && &bytes[pe_offset..pe_offset + 4] == b"PE\0\0" {
            let machine = u16::from_le_bytes([bytes[pe_offset + 4], bytes[pe_offset + 5]]);
            return match machine {
                PE_MACHINE_AMD64 => WindowsBinaryInspection {
                    path: path_label,
                    kind: WindowsBinaryKind::Pe64X86_64,
                    machine_code: Some(machine),
                    supported_by_wine_wow64: true,
                    detail: "64-bit Windows PE32+ (x86-64); supported via Box64 + Wine 11.0.".to_string(),
                },
                PE_MACHINE_I386 => WindowsBinaryInspection {
                    path: path_label,
                    kind: WindowsBinaryKind::Pe32I386WoW64,
                    machine_code: Some(machine),
                    supported_by_wine_wow64: true,
                    detail: "32-bit Windows PE32 (i386); routed through Wine 11.0 pure-64-bit WoW64 mode (no Box86 required).".to_string(),
                },
                PE_MACHINE_ARM64 => WindowsBinaryInspection {
                    path: path_label,
                    kind: WindowsBinaryKind::Pe64Arm64,
                    machine_code: Some(machine),
                    supported_by_wine_wow64: false,
                    detail: "ARM64 PE executable detected; requires native ARM64 Wine rather than x86-64 Wine+Box64.".to_string(),
                },
                other => WindowsBinaryInspection {
                    path: path_label,
                    kind: WindowsBinaryKind::Unsupported,
                    machine_code: Some(other),
                    supported_by_wine_wow64: false,
                    detail: format!("Unsupported PE machine identifier 0x{other:04X}."),
                },
            };
        }
    }

    WindowsBinaryInspection {
        path: path_label,
        kind: WindowsBinaryKind::Unsupported,
        machine_code: None,
        supported_by_wine_wow64: false,
        detail: "File is neither a valid Windows PE (.exe/.dll) nor an OLE2 MSI package.".to_string(),
    }
}

pub struct WineRegistryManager;

impl WineRegistryManager {
    /// Merges base overrides (e.g. disabling Wine Mono/Gecko popups) with graphics
    /// and user-configured DLL overrides into a deterministic `WINEDLLOVERRIDES` string.
    pub fn build_dll_overrides(
        base_overrides: &[(&str, &str)],
        extra_overrides: &[(String, String)],
    ) -> String {
        let mut merged: BTreeMap<String, String> = BTreeMap::new();
        for (dll, mode) in base_overrides {
            merged.insert((*dll).to_string(), (*mode).to_string());
        }
        for (dll, mode) in extra_overrides {
            let trimmed = dll.trim();
            if !trimmed.is_empty() {
                merged.insert(trimmed.to_string(), mode.trim().to_string());
            }
        }
        merged
            .into_iter()
            .map(|(dll, mode)| format!("{dll}={mode}"))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Produces a safe `wine reg add` argument list to configure the reported Windows version
    /// (`win10`, `win11`, `win7`, `win81`) inside an isolated Wine prefix.
    pub fn windows_version_reg_commands(windows_version: &str) -> Vec<Vec<String>> {
        let (product_name, current_ver, current_build) = match windows_version.trim().to_ascii_lowercase().as_str() {
            "win11" => ("Windows 11", "10.0", "22631"),
            "win7" => ("Windows 7", "6.1", "7601"),
            "win81" => ("Windows 8.1", "6.3", "9600"),
            _ => ("Windows 10", "10.0", "19045"),
        };

        vec![
            vec![
                "reg".into(),
                "add".into(),
                r"HKLM\Software\Microsoft\Windows NT\CurrentVersion".into(),
                "/v".into(),
                "ProductName".into(),
                "/t".into(),
                "REG_SZ".into(),
                "/d".into(),
                product_name.into(),
                "/f".into(),
            ],
            vec![
                "reg".into(),
                "add".into(),
                r"HKLM\Software\Microsoft\Windows NT\CurrentVersion".into(),
                "/v".into(),
                "CurrentVersion".into(),
                "/t".into(),
                "REG_SZ".into(),
                "/d".into(),
                current_ver.into(),
                "/f".into(),
            ],
            vec![
                "reg".into(),
                "add".into(),
                r"HKLM\Software\Microsoft\Windows NT\CurrentVersion".into(),
                "/v".into(),
                "CurrentBuildNumber".into(),
                "/t".into(),
                "REG_SZ".into(),
                "/d".into(),
                current_build.into(),
                "/f".into(),
            ],
        ]
    }
}

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const MAX_ELF_INSPECT_BYTES: usize = 4 * 1024 * 1024;
const MAX_SHEBANG_DEPTH: usize = 4;

const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;

const SHT_STRTAB: u32 = 3;
const SHT_DYNAMIC: u32 = 6;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;

const EM_386: u16 = 3;
const EM_ARM: u16 = 40;
const EM_X86_64: u16 = 62;
const EM_AARCH64: u16 = 183;
const EM_RISCV: u16 = 243;

#[derive(Debug, Error)]
pub enum ElfParseError {
    #[error("Binary not found: {0}")]
    NotFound(String),

    #[error("I/O error inspecting binary '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid ELF header: {0}")]
    InvalidElf(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElfClass {
    Elf32,
    Elf64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElfEndianness {
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElfObjectType {
    Relocatable,
    Executable,
    SharedOrPie,
    Core,
    Unknown(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElfArchitecture {
    Aarch64,
    X86_64,
    X86_32,
    Arm32,
    Riscv64,
    Other(u16),
}

impl ElfArchitecture {
    pub fn from_e_machine(machine: u16) -> Self {
        match machine {
            EM_AARCH64 => Self::Aarch64,
            EM_X86_64 => Self::X86_64,
            EM_386 => Self::X86_32,
            EM_ARM => Self::Arm32,
            EM_RISCV => Self::Riscv64,
            other => Self::Other(other),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64 (ARM64)",
            Self::X86_64 => "x86_64 (AMD64)",
            Self::X86_32 => "i386 (x86 32-bit)",
            Self::Arm32 => "arm (32-bit ARM)",
            Self::Riscv64 => "riscv64 (RISC-V)",
            Self::Other(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElfInspection {
    pub guest_path: String,
    pub class: ElfClass,
    pub endianness: ElfEndianness,
    pub object_type: ElfObjectType,
    pub architecture: ElfArchitecture,
    pub e_machine: u16,
    pub interpreter: Option<String>,
    pub needed_libraries: Vec<String>,
    pub rpaths: Vec<String>,
    pub runpaths: Vec<String>,
    pub is_static: bool,
    pub uses_android_bionic: bool,
    pub shebang_interpreter: Option<String>,
}

impl ElfInspection {
    pub fn is_runnable_elf(&self) -> bool {
        matches!(
            self.object_type,
            ElfObjectType::Executable | ElfObjectType::SharedOrPie
        )
    }
}

/// Inspects a guest executable inside `rootfs_dir` (resolving `/path/in/guest` safely within rootfs).
pub fn inspect_guest_binary(
    rootfs_dir: &Path,
    guest_path: &str,
) -> Result<ElfInspection, ElfParseError> {
    inspect_guest_binary_inner(rootfs_dir, guest_path, 0)
}

fn inspect_guest_binary_inner(
    rootfs_dir: &Path,
    guest_path: &str,
    depth: usize,
) -> Result<ElfInspection, ElfParseError> {
    if depth > MAX_SHEBANG_DEPTH {
        return Err(ElfParseError::InvalidElf(
            "Exceeded maximum shebang interpreter indirection depth".into(),
        ));
    }

    let host_path = resolve_path_in_rootfs(rootfs_dir, guest_path)
        .ok_or_else(|| ElfParseError::NotFound(guest_path.to_string()))?;

    let mut file = File::open(&host_path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            ElfParseError::NotFound(guest_path.to_string())
        } else {
            ElfParseError::Io {
                path: guest_path.to_string(),
                source,
            }
        }
    })?;

    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_ELF_INSPECT_BYTES as u64)
        .read_to_end(&mut buffer)
        .map_err(|source| ElfParseError::Io {
            path: guest_path.to_string(),
            source,
        })?;

    if buffer.starts_with(b"#!") {
        if let Some(interp) = parse_shebang_interpreter(&buffer) {
            let mut inspected = inspect_guest_binary_inner(rootfs_dir, &interp, depth + 1)?;
            inspected.guest_path = guest_path.to_string();
            inspected.shebang_interpreter = Some(interp);
            return Ok(inspected);
        }
        return Err(ElfParseError::InvalidElf(
            "Malformed shebang (#!) header with empty interpreter".into(),
        ));
    }

    inspect_elf_bytes(guest_path, &buffer)
}

pub fn resolve_path_in_rootfs(rootfs_dir: &Path, guest_path: &str) -> Option<PathBuf> {
    let trimmed = guest_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        // Resolve relative command names along standard Linux guest PATH directories.
        for dir in [
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/local/sbin",
            "/usr/sbin",
            "/sbin",
        ] {
            let candidate = safe_join_rootfs(rootfs_dir, &format!("{dir}/{trimmed}"))?;
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        return None;
    };

    let candidate = safe_join_rootfs(rootfs_dir, &normalized)?;
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn safe_join_rootfs(rootfs_dir: &Path, guest_abs: &str) -> Option<PathBuf> {
    let mut resolved = rootfs_dir.to_path_buf();
    for component in Path::new(guest_abs.trim_start_matches('/')).components() {
        match component {
            std::path::Component::Normal(part) => resolved.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if resolved == rootfs_dir {
                    return None;
                }
                resolved.pop();
            }
            _ => return None,
        }
    }
    Some(resolved)
}

fn parse_shebang_interpreter(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == b'\n').unwrap_or(bytes.len().min(256));
    let line = std::str::from_utf8(&bytes[2..end]).ok()?.trim();
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first == "/usr/bin/env" || first == "/bin/env" {
        if let Some(target) = parts.next() {
            if target.starts_with('/') {
                return Some(target.to_string());
            }
            return Some(format!("/usr/bin/{target}"));
        }
    }
    Some(first.to_string())
}

/// Pure-Rust ELF32/ELF64 inspector extracting architecture, PT_INTERP, and DT_NEEDED dependencies.
pub fn inspect_elf_bytes(guest_path: &str, bytes: &[u8]) -> Result<ElfInspection, ElfParseError> {
    if bytes.len() < 52 {
        return Err(ElfParseError::InvalidElf(format!(
            "File too small ({} bytes) to contain a valid ELF header",
            bytes.len()
        )));
    }
    if bytes[0..4] != ELF_MAGIC {
        return Err(ElfParseError::InvalidElf(
            "Missing ELF magic bytes (\\x7fELF)".into(),
        ));
    }

    let class = match bytes[4] {
        1 => ElfClass::Elf32,
        2 => ElfClass::Elf64,
        other => {
            return Err(ElfParseError::InvalidElf(format!(
                "Unknown ELF class byte: {other}"
            )))
        }
    };

    let endianness = match bytes[5] {
        1 => ElfEndianness::Little,
        2 => ElfEndianness::Big,
        other => {
            return Err(ElfParseError::InvalidElf(format!(
                "Unknown ELF endianness byte: {other}"
            )))
        }
    };

    let read_u16 = |off: usize| -> Result<u16, ElfParseError> {
        let slice = bytes
            .get(off..off + 2)
            .ok_or_else(|| ElfParseError::InvalidElf("Truncated ELF header (u16)".into()))?;
        let arr = [slice[0], slice[1]];
        Ok(match endianness {
            ElfEndianness::Little => u16::from_le_bytes(arr),
            ElfEndianness::Big => u16::from_be_bytes(arr),
        })
    };

    let read_u32 = |off: usize| -> Result<u32, ElfParseError> {
        let slice = bytes
            .get(off..off + 4)
            .ok_or_else(|| ElfParseError::InvalidElf("Truncated ELF header (u32)".into()))?;
        let arr = [slice[0], slice[1], slice[2], slice[3]];
        Ok(match endianness {
            ElfEndianness::Little => u32::from_le_bytes(arr),
            ElfEndianness::Big => u32::from_be_bytes(arr),
        })
    };

    let read_u64 = |off: usize| -> Result<u64, ElfParseError> {
        let slice = bytes
            .get(off..off + 8)
            .ok_or_else(|| ElfParseError::InvalidElf("Truncated ELF header (u64)".into()))?;
        let arr = [
            slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
        ];
        Ok(match endianness {
            ElfEndianness::Little => u64::from_le_bytes(arr),
            ElfEndianness::Big => u64::from_be_bytes(arr),
        })
    };

    let e_type_raw = read_u16(16)?;
    let object_type = match e_type_raw {
        1 => ElfObjectType::Relocatable,
        2 => ElfObjectType::Executable,
        3 => ElfObjectType::SharedOrPie,
        4 => ElfObjectType::Core,
        other => ElfObjectType::Unknown(other),
    };

    let e_machine = read_u16(18)?;
    let architecture = ElfArchitecture::from_e_machine(e_machine);

    let (e_phoff, e_shoff, e_phentsize, e_phnum, e_shentsize, e_shnum) = match class {
        ElfClass::Elf64 => {
            if bytes.len() < 64 {
                return Err(ElfParseError::InvalidElf("Truncated ELF64 header".into()));
            }
            (
                read_u64(32)? as usize,
                read_u64(40)? as usize,
                read_u16(54)? as usize,
                read_u16(56)? as usize,
                read_u16(58)? as usize,
                read_u16(60)? as usize,
            )
        }
        ElfClass::Elf32 => (
            read_u32(28)? as usize,
            read_u32(32)? as usize,
            read_u16(42)? as usize,
            read_u16(44)? as usize,
            read_u16(46)? as usize,
            read_u16(48)? as usize,
        ),
    };

    let mut interpreter = None;
    let mut has_dynamic = false;

    if e_phoff > 0 && e_phentsize >= 32 && e_phnum > 0 {
        for i in 0..e_phnum.min(256) {
            let base = match e_phoff.checked_add(i.saturating_mul(e_phentsize)) {
                Some(b) if b + e_phentsize <= bytes.len() => b,
                _ => break,
            };
            let p_type = read_u32(base)?;
            if p_type == PT_DYNAMIC {
                has_dynamic = true;
            } else if p_type == PT_INTERP {
                let (p_offset, p_filesz) = match class {
                    ElfClass::Elf64 => (read_u64(base + 8)? as usize, read_u64(base + 32)? as usize),
                    ElfClass::Elf32 => (read_u32(base + 4)? as usize, read_u32(base + 16)? as usize),
                };
                if let Some(interp_bytes) = bytes.get(p_offset..p_offset.saturating_add(p_filesz)) {
                    let nul_end = interp_bytes
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(interp_bytes.len());
                    if let Ok(s) = std::str::from_utf8(&interp_bytes[..nul_end]) {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() {
                            interpreter = Some(trimmed.to_string());
                        }
                    }
                }
            }
        }
    }

    let mut needed_libraries = Vec::new();
    let mut rpaths = Vec::new();
    let mut runpaths = Vec::new();

    if e_shoff > 0 && e_shentsize >= 40 && e_shnum > 0 {
        let mut sections: Vec<(u32, usize, usize, usize)> = Vec::new();
        for i in 0..e_shnum.min(512) {
            let base = match e_shoff.checked_add(i.saturating_mul(e_shentsize)) {
                Some(b) if b + e_shentsize <= bytes.len() => b,
                _ => break,
            };
            let sh_type = read_u32(base + 4)?;
            let (sh_offset, sh_size, sh_link) = match class {
                ElfClass::Elf64 => (
                    read_u64(base + 24)? as usize,
                    read_u64(base + 32)? as usize,
                    read_u32(base + 40)? as usize,
                ),
                ElfClass::Elf32 => (
                    read_u32(base + 16)? as usize,
                    read_u32(base + 20)? as usize,
                    read_u32(base + 24)? as usize,
                ),
            };
            sections.push((sh_type, sh_offset, sh_size, sh_link));
        }

        for &(sh_type, sh_offset, sh_size, sh_link) in &sections {
            if sh_type != SHT_DYNAMIC {
                continue;
            }
            has_dynamic = true;
            let Some(&(str_type, str_off, str_size, _)) = sections.get(sh_link) else {
                continue;
            };
            if str_type != SHT_STRTAB {
                continue;
            }
            let Some(strtab) = bytes.get(str_off..str_off.saturating_add(str_size)) else {
                continue;
            };
            let entry_size = match class {
                ElfClass::Elf64 => 16,
                ElfClass::Elf32 => 8,
            };
            let count = (sh_size / entry_size).min(1024);
            for idx in 0..count {
                let off = sh_offset + idx * entry_size;
                if off + entry_size > bytes.len() {
                    break;
                }
                let (d_tag, d_val) = match class {
                    ElfClass::Elf64 => (read_u64(off)?, read_u64(off + 8)? as usize),
                    ElfClass::Elf32 => (read_u32(off)? as u64, read_u32(off + 4)? as usize),
                };
                if d_tag == DT_NULL {
                    break;
                }
                if matches!(d_tag, DT_NEEDED | DT_RPATH | DT_RUNPATH) {
                    if let Some(name) = read_c_string(strtab, d_val) {
                        match d_tag {
                            DT_NEEDED => needed_libraries.push(name),
                            DT_RPATH => rpaths.extend(name.split(':').filter(|s| !s.is_empty()).map(str::to_owned)),
                            DT_RUNPATH => runpaths.extend(name.split(':').filter(|s| !s.is_empty()).map(str::to_owned)),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let uses_android_bionic = interpreter.as_deref().is_some_and(|i| {
        i.starts_with("/system/bin/linker") || i.contains("bionic")
    }) || needed_libraries
        .iter()
        .any(|lib| lib == "libc.so" || lib == "libdl.so" || lib == "libandroid.so");

    Ok(ElfInspection {
        guest_path: guest_path.to_string(),
        class,
        endianness,
        object_type,
        architecture,
        e_machine,
        interpreter,
        needed_libraries,
        rpaths,
        runpaths,
        is_static: !has_dynamic,
        uses_android_bionic,
        shebang_interpreter: None,
    })
}

fn read_c_string(table: &[u8], offset: usize) -> Option<String> {
    let tail = table.get(offset..)?;
    let nul = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    let text = std::str::from_utf8(&tail[..nul]).ok()?.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

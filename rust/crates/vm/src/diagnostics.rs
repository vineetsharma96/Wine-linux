use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::config::{EDK2_PINNED_VERSION, LIBSLIRP_PINNED_VERSION, QEMU_PINNED_VERSION};
use crate::network::VmNetworkBuilder;

pub const VM_LOG_MAX_LINES: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmCapabilities {
    pub device_present: bool,
    pub accessible: bool,
    pub accelerator: String,
    pub notes: String,
}

impl KvmCapabilities {
    /// Probes `/dev/kvm` on the host without requesting root privileges.
    pub fn probe() -> Self {
        let kvm_path = Path::new("/dev/kvm");
        let present = kvm_path.exists();
        if !present {
            return Self {
                device_present: false,
                accessible: false,
                accelerator: "tcg".to_string(),
                notes: "/dev/kvm not exposed by kernel; using QEMU multi-threaded TCG software translation".to_string(),
            };
        }

        match OpenOptions::new().read(true).write(true).open(kvm_path) {
            Ok(_) => Self {
                device_present: true,
                accessible: true,
                accelerator: "kvm".to_string(),
                notes: "/dev/kvm accessible; hardware-assisted virtualization available for ARM64 guests".to_string(),
            },
            Err(err) => Self {
                device_present: true,
                accessible: false,
                accelerator: "tcg".to_string(),
                notes: format!("/dev/kvm present but restricted by Android SELinux/permissions ({err}); falling back to TCG"),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvfCapabilities {
    pub ordinary_app_accessible: bool,
    pub permission_required: String,
    pub notes: String,
}

impl AvfCapabilities {
    pub fn report() -> Self {
        Self {
            ordinary_app_accessible: false,
            permission_required: "android.permission.MANAGE_VIRTUAL_MACHINE (@SystemApi)".to_string(),
            notes: "Android Virtualization Framework (AVF / pKVM) is restricted to privileged system apps (e.g. Android 16 system Terminal). Ordinary rootless third-party apps cannot obtain MANAGE_VIRTUAL_MACHINE and must rely on QEMU TCG.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QemuEngineCapabilities {
    pub pinned_qemu_version: String,
    pub pinned_edk2_version: String,
    pub pinned_libslirp_version: String,
    pub qemu_aarch64_installed: bool,
    pub qemu_x86_64_installed: bool,
    pub qemu_img_installed: bool,
    pub aavmf_firmware_installed: bool,
    pub ovmf_firmware_installed: bool,
    pub kvm: KvmCapabilities,
    pub avf: AvfCapabilities,
    pub networking_notes: String,
    pub security_boundary_note: String,
}

impl QemuEngineCapabilities {
    pub fn probe(qemu_bin_dir: &Path, firmware_dir: &Path) -> Self {
        Self {
            pinned_qemu_version: QEMU_PINNED_VERSION.to_string(),
            pinned_edk2_version: EDK2_PINNED_VERSION.to_string(),
            pinned_libslirp_version: LIBSLIRP_PINNED_VERSION.to_string(),
            qemu_aarch64_installed: qemu_bin_dir.join("qemu-system-aarch64").is_file(),
            qemu_x86_64_installed: qemu_bin_dir.join("qemu-system-x86_64").is_file(),
            qemu_img_installed: qemu_bin_dir.join("qemu-img").is_file(),
            aavmf_firmware_installed: firmware_dir.join("AAVMF_CODE.fd").is_file(),
            ovmf_firmware_installed: firmware_dir.join("OVMF_CODE.fd").is_file(),
            kvm: KvmCapabilities::probe(),
            avf: AvfCapabilities::report(),
            networking_notes: VmNetworkBuilder::rootless_limitations_summary().to_string(),
            security_boundary_note: "QEMU runs inside the ordinary Android app UID sandbox; it provides guest emulation isolation rather than an absolute kernel/hardware security boundary.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmProcessTelemetry {
    pub pid: u32,
    pub start_time_ticks: Option<u64>,
    pub memory_rss_bytes: Option<u64>,
    pub cpu_time_ms: Option<u64>,
    pub alive: bool,
}

impl VmProcessTelemetry {
    /// Reads `/proc/<pid>/status` and `/proc/<pid>/stat` honestly; returns `None` when unreadable.
    pub fn inspect(pid: u32, expected_start_ticks: Option<u64>) -> Self {
        let start_ticks = read_proc_start_time_ticks(pid);
        let identity_matches = match (expected_start_ticks, start_ticks) {
            (Some(exp), Some(actual)) => exp == actual,
            (None, Some(_)) => true,
            _ => false,
        };

        if !identity_matches {
            return Self {
                pid,
                start_time_ticks: start_ticks,
                memory_rss_bytes: None,
                cpu_time_ms: None,
                alive: false,
            };
        }

        Self {
            pid,
            start_time_ticks: start_ticks,
            memory_rss_bytes: read_proc_rss_bytes(pid),
            cpu_time_ms: read_proc_cpu_ms(pid),
            alive: true,
        }
    }
}

/// Reads field 22 (`starttime`) from `/proc/<pid>/stat` to prevent PID-reuse termination mistakes.
pub fn read_proc_start_time_ticks(pid: u32) -> Option<u64> {
    let raw = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = raw.rsplit_once(") ")?.1;
    // After ") ", state is index 0 (field 3), so field 22 (starttime) is index 19.
    after_comm.split_whitespace().nth(19)?.parse::<u64>().ok()
}

pub fn read_proc_rss_bytes(pid: u32) -> Option<u64> {
    let raw = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

pub fn read_proc_cpu_ms(pid: u32) -> Option<u64> {
    let raw = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = raw.rsplit_once(") ")?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    // utime = field 14 (index 11), stime = field 15 (index 12) in clock ticks (typically 100 Hz)
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((utime + stime) * 10)
}

#[derive(Debug, Default)]
pub struct VmLogBuffer {
    lines: VecDeque<String>,
}

impl VmLogBuffer {
    pub fn push(&mut self, line: impl Into<String>) {
        if self.lines.len() >= VM_LOG_MAX_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line.into());
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }
}

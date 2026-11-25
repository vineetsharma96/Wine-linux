use std::path::Path;
use serde::{Deserialize, Serialize};
use androidpc_configuration::is_android_bionic_path;
use crate::elf::{inspect_guest_binary, ElfArchitecture, ElfInspection};

/// Core Linux shared libraries wrapped natively by Box64 onto ARM64 guest glibc libraries.
///
/// Box64 wraps these libraries so x86-64 binaries invoke the guest ARM64 `.so` directly
/// without translating libc/libm/pthread/X11/OpenGL/Vulkan internals.
const BOX64_WRAPPED_LIBRARIES: &[&str] = &[
    "ld-linux-x86-64.so.2",
    "libc.so.6",
    "libm.so.6",
    "libpthread.so.0",
    "libdl.so.2",
    "librt.so.1",
    "libutil.so.1",
    "libresolv.so.2",
    "libanl.so.1",
    "libcrypt.so.1",
    "libnsl.so.1",
    "libgcc_s.so.1",
    "libstdc++.so.6",
    "libz.so.1",
    "libbz2.so.1.0",
    "liblzma.so.5",
    "libexpat.so.1",
    "libffi.so.7",
    "libffi.so.8",
    "libncurses.so.6",
    "libncursesw.so.6",
    "libtinfo.so.6",
    "libreadline.so.8",
    "libssl.so.3",
    "libcrypto.so.3",
    "libcurl.so.4",
    "libglib-2.0.so.0",
    "libgobject-2.0.so.0",
    "libgio-2.0.so.0",
    "libX11.so.6",
    "libXext.so.6",
    "libXrender.so.1",
    "libXrandr.so.2",
    "libXcursor.so.1",
    "libXi.so.6",
    "libXfixes.so.3",
    "libXxf86vm.so.1",
    "libxcb.so.1",
    "libGL.so.1",
    "libGLX.so.0",
    "libEGL.so.1",
    "libGLESv2.so.2",
    "libvulkan.so.1",
    "libasound.so.2",
    "libpulse.so.0",
    "libpulse-simple.so.0",
    "libSDL2-2.0.so.0",
    "libfreetype.so.6",
    "libfontconfig.so.1",
    "libpng16.so.16",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLibraryEntry {
    pub soname: String,
    pub guest_path: String,
    pub architecture: ElfArchitecture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LibraryResolutionReport {
    pub wrapped_native_libs: Vec<String>,
    pub resolved_guest_libs: Vec<ResolvedLibraryEntry>,
    pub missing_libs: Vec<String>,
    pub abi_warnings: Vec<String>,
}

impl LibraryResolutionReport {
    pub fn has_missing_dependencies(&self) -> bool {
        !self.missing_libs.is_empty()
    }
}

pub fn is_box64_wrapped_library(soname: &str) -> bool {
    BOX64_WRAPPED_LIBRARIES
        .iter()
        .any(|wrapped| wrapped.eq_ignore_ascii_case(soname))
}

/// Checks `DT_NEEDED` dependencies for an inspected guest binary against guest rootfs paths
/// and Box64's native library wrapper table while guarding against Android Bionic contamination.
pub fn resolve_guest_libraries(
    rootfs_dir: &Path,
    inspection: &ElfInspection,
    configured_search_paths: &[String],
) -> LibraryResolutionReport {
    let mut report = LibraryResolutionReport::default();

    let mut search_paths: Vec<String> = Vec::new();
    if let Some(parent) = Path::new(&inspection.guest_path).parent() {
        let parent_str = parent.to_string_lossy().to_string();
        if parent_str.starts_with('/') {
            search_paths.push(parent_str);
        }
    }
    for p in inspection
        .runpaths
        .iter()
        .chain(inspection.rpaths.iter())
        .chain(configured_search_paths.iter())
    {
        if is_android_bionic_path(p) {
            report.abi_warnings.push(format!(
                "Ignored Android/Bionic host path '{p}' while resolving guest libraries"
            ));
            continue;
        }
        if p.starts_with('/') && !p.contains("..") && !search_paths.contains(p) {
            search_paths.push(p.clone());
        }
    }

    if inspection.architecture == ElfArchitecture::Aarch64 {
        for default_arm64_dir in ["/usr/lib/aarch64-linux-gnu", "/lib/aarch64-linux-gnu", "/usr/lib", "/lib"] {
            let owned = default_arm64_dir.to_string();
            if !search_paths.contains(&owned) {
                search_paths.push(owned);
            }
        }
    }

    for soname in &inspection.needed_libraries {
        if inspection.architecture == ElfArchitecture::X86_64 && is_box64_wrapped_library(soname) {
            report.wrapped_native_libs.push(soname.clone());
            continue;
        }

        let mut matched = false;
        for dir in &search_paths {
            let candidate_guest = format!("{}/{}", dir.trim_end_matches('/'), soname);
            match inspect_guest_binary(rootfs_dir, &candidate_guest) {
                Ok(lib_elf) => {
                    if lib_elf.uses_android_bionic {
                        report.abi_warnings.push(format!(
                            "Skipped '{candidate_guest}': linked against Android Bionic instead of Linux glibc"
                        ));
                        continue;
                    }
                    if lib_elf.architecture != inspection.architecture {
                        report.abi_warnings.push(format!(
                            "Skipped '{candidate_guest}': architecture {:?} does not match executable {:?}",
                            lib_elf.architecture, inspection.architecture
                        ));
                        continue;
                    }
                    report.resolved_guest_libs.push(ResolvedLibraryEntry {
                        soname: soname.clone(),
                        guest_path: candidate_guest,
                        architecture: lib_elf.architecture,
                    });
                    matched = true;
                    break;
                }
                Err(_) => continue,
            }
        }

        if !matched {
            report.missing_libs.push(soname.clone());
        }
    }

    report
}

# Windows Dependency Management (Phase 4.5)

*Last updated: 2026-11-25*
*Rust crate: `rust/crates/dependency-manager` (`androidpc-dependency-manager`) · UI: `WineAndGraphicsTabs.kt` (Deps & Diag tab)*

## 1. Design Principles (No Blind `curl | sh`, No Unlicensed Proprietary Bundles)

Android PC Environment explicitly **rejects**:
- Bundling proprietary Microsoft redistributables (`vc_redist`, `.NET`, DirectX End-User Runtime, Microsoft Core Fonts) inside the APK or repository.
- Executing unverified remote shell scripts (`curl ... | sh` or unpinned upstream Winetricks scripts).

Instead, `DependencyManager` implements a transparent, auditable two-mechanism catalog:

| Profile ID | Component | Version | License | Installation Mechanism |
|---|---|---|---|---|
| `wine-mono` | Wine Mono (.NET open-source runtime) | `9.4.0` | MIT / GPL-2 / LGPL-2 | `verified_open_source_artifact` (SHA-256 verified `.msi` via WineHQ) |
| `dxvk-2.7.1` | DXVK (D3D8/9/10/11 → Vulkan) | `2.7.1` | zlib | `verified_open_source_artifact` (requires `Operational` Vulkan 1.3+ backend) |
| `vkd3d-proton-3.1` | VKD3D-Proton (D3D12 → Vulkan) | `3.1` | LGPL-2.1 | `verified_open_source_artifact` (requires `Operational` Vulkan 1.3+ + 6 extensions) |
| `vcrun2015-2022` | Visual C++ 2015–2022 Redistributable | `14.x` | Proprietary (Microsoft) | `user_supplied_official_installer` (user downloads official `vc_redist.x64.exe` / `vc_redist.x86.exe` from Microsoft and runs inside isolated prefix) |
| `dotnet-desktop-8` | .NET 8 Desktop Runtime | `8.0.x` | MIT / Microsoft | `user_supplied_official_installer` (official Microsoft installer run inside isolated prefix) |
| `corefonts` | Liberation / DejaVu Open-Source Font Substitution | System | SIL OFL / Apache-2 | `verified_open_source_artifact` (guest APT open-source metric-compatible fonts) |

## 2. Missing DLL Scanner & Conflict Detection

`DependencyManager::diagnose_prefix`:
1. **Prefix DLL Inventory**: Scans `<prefix>/drive_c/windows/system32/` and `syswow64/` to record present DLLs.
2. **Log Pattern Scanner**: Parses Wine stderr (`err:module:import_dll Library <name>.dll ... not found`, `err:module:load_so_dll`) to identify missing runtime DLLs (`vcruntime140.dll`, `msvcp140.dll`, `ucrtbase.dll`, `mscoree.dll`, `d3d11.dll`, `d3d12.dll`, etc.).
3. **Recommendation Mapping**: Maps missing DLLs back to catalog profiles (e.g., `vcruntime140.dll` → `vcrun2015-2022`, `mscoree.dll` → `wine-mono` or `dotnet-desktop-8`).
4. **Conflict Detection**: Flags mutually conflicting installations inside the same prefix (e.g., `wine-mono` + `dotnet-desktop-8` both active simultaneously in the same prefix) and supplies concrete recovery actions (clone prefix or remove conflicting override).

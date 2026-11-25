# Wine Compatibility Research (Phase 4.2)

*Last updated: 2026-11-25*
*Target architecture: ARM64 Android host → Rootless PRoot Ubuntu/Debian ARM64 guest → Box64 `v0.4.4` → Wine `11.0` (x86-64 pure-64-bit WoW64)*

## 1. Executive Summary & Architecture Decision

For running x86 (32-bit PE32) and x86-64 (64-bit PE32+) Windows applications on ARM64 Linux guests without Android root access, **Android PC Environment pins Wine `11.0` (LGPL-2.1-or-later, official x86-64 pure-64-bit WoW64 build) executed through Box64 `v0.4.4` (MIT)**.

### Why x86-64 Wine 11.0 WoW64 via Box64 (and NOT Box86 or Native ARM64 Wine-only)
1. **Pure 64-bit WoW64 parity in Wine 11.0**:
   - Released January 2026 by WineHQ (`https://www.winehq.org/`), Wine 11.0 delivers mature new-WoW64 support where 32-bit Windows PE32 (`i386`) applications run inside a 64-bit Unix process without requiring 32-bit x86 Linux multilib libraries (`i386` glibc, `i386` X11, `i386` Vulkan) or Box86.
   - Single `wine` binary routes both PE32 (`0x014c`) and PE32+ (`0x8664`) executables via Box64 translation.
   - Eliminating Box86 removes 32-bit ARM (`armhf`) ABI dependencies that fail on modern 64-bit-only ARMv9 SoCs (including Snapdragon 7+ Gen 3 / Adreno 732 on Xiaomi Pad 7).
2. **Comparison against Alternative Strategies**:
   - **Native ARM64 Wine + Hangover/FEX/Box64 DLL thunks**: Hangover (`AndreRH/hangover`) runs Wine's Unix side natively on ARM64 and translates Windows PE code via FEX-Emu or Box64 WoW64 DLLs. While promising for reduced syscall overhead, native ARM64 Wine requires custom upstream/forked Wine builds with FEX/Box64 WoW64 glue and currently exhibits higher fragility across arbitrary user installers compared to standard upstream x86-64 Wine under Box64. Our `WineRuntimeManager` architecture keeps `WineVersion` and `WineArchitecture` pluggable so a future Hangover/ARM64-Wine strategy can be evaluated without breaking prefix isolation.
   - **Box86 + 32-bit Wine**: Rejected because modern ARM64 Android devices drop 32-bit kernel/userspace execution paths and 32-bit multilib PRoot rootfses bloat storage and complicate library resolution.

## 2. Confirmed Facts (Official Sources & Code Inspection)

- **Official Project**: WineHQ (`https://www.winehq.org/`, `https://gitlab.winehq.org/wine/wine`)
- **Pinned Version**: Wine `11.0` (stable release, January 2026)
- **License**: `LGPL-2.1-or-later`
- **WoW64 Behavior**: A single 64-bit `WINEARCH=win64` prefix supports both 64-bit PE32+ (`C:\windows\system32`) and 32-bit PE32 (`C:\windows\syswow64`) binaries when built with `--enable-archs=i386,x86_64` (new WoW64 mode).
- **Binary Inspection (Implemented in `rust/crates/wine/src/registry.rs`)**:
  - `inspect_windows_binary_bytes` parses DOS `MZ` magic (`0x4D 0x5A`), `e_lfanew` offset, `PE\0\0` signature, COFF Machine (`0x014C` i386, `0x8664` x86-64, `0xAA64` ARM64), Optional Header Magic (`0x10B` PE32, `0x20B` PE32+), and OLE2 Compound File magic (`D0 CF 11 E0 A1 B1 1A E1`) for `.msi` packages.
- **Prefix Isolation & Persistence (Implemented in `rust/crates/wine/src/prefix.rs`)**:
  - Every prefix lives under its own isolated directory (`<data_dir>/wine_prefixes/<prefix_id>`) with `prefix.json`, `drive_c/windows/{system32,syswow64}`, `dosdevices/c:`, and independent `wineserver` sockets.
  - Terminating `wineserver -k` / `wineserver -k9` with a scoped `WINEPREFIX` affects only that prefix and never issues global `killall` across other prefixes or Ubuntu/Debian guest daemons.
- **Registry & DLL Overrides**:
  - Controlled via `WINEDLLOVERRIDES` environment variables and `reg add` commands (`HKCU\Software\Wine\DllOverrides`, `HKCU\Software\Wine\Explorer\Desktops`).
  - Supported override modes: `NativeThenBuiltin`, `BuiltinThenNative`, `NativeOnly`, `BuiltinOnly`, `Disabled`.

## 3. Public Reference Project Observations (Winlator / Hangover)

- **Winlator** (public reference analysis documented in `WINDOWS_COMPATIBILITY_RESEARCH.md`): Demonstrates that bundling Wine inside an isolated rootfs/prefix with Box64 Dynarec and explicit `WINEDLLOVERRIDES` (`d3d9,d3d10core,d3d11,dxgi=n,b` for DXVK; `d3d12,d3d12core=n,b` for VKD3D) enables predictable switching between WineD3D and Vulkan-based DirectX translation.
- **Key Architectural Difference in Android PC Environment**: Unlike Winlator's single-purpose container scripts, Android PC Environment keeps Ubuntu/Debian ARM64 Linux environments intact, runs all orchestration in Rust (`androidpc-wine` + `androidpc-core`), and separates prefix management, dependency management, graphics probing, and app lifecycle into auditable, unit-tested Rust crates.

## 4. Hypotheses & Unverified Runtime Areas (Pending Xiaomi Pad 7 Device Testing)

- **PRoot + `wineserver` Socket Paths**: Unix domain socket paths inside `/tmp/.wine-<uid>/server-*` must remain under Linux's 108-byte `sun_path` limit. Our existing Stage 3.1 short `<base_data_dir>/dtmp:/tmp` bind addresses this, but live `wineserver` startup inside PRoot under Box64 `v0.4.4` is **`NOT RUN`** on Xiaomi Pad 7.
- **ESync / FSync Availability**: Unprivileged Android kernels may restrict `eventfd` limits or lack `futex_waitv` (FSync) for PRoot guests. Therefore, `WINEESYNC` and `WINEFSYNC` default to **disabled** (`0`) until tested on target hardware.
- **MSIEXEC Complex Installers**: `wine msiexec /i <path>` is wired in `WineRuntimeManager`, but InstallShield / WiX custom actions involving Windows services or `.NET` bootstrapper chains require live device verification.
- **Audio Routing**: Guest Wine ALSA/PulseAudio output to our existing loopback `AudioTcpSink` (`127.0.0.1:4713`) is architected via `PULSE_SERVER=tcp:127.0.0.1:4713`, pending end-to-end audio validation on device.

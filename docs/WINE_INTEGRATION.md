# Wine Integration Guide (Phase 4.2)

*Last updated: 2026-11-25*
*Pinned release: Wine `11.0` (LGPL-2.1-or-later, x86-64 pure-64-bit WoW64) — `https://www.winehq.org/`*

## 1. Crate Structure (`rust/crates/wine`)

`androidpc-wine` preserves all legacy `WineArch` / `WineConfig` / `WineCommandBuilder` / `WineProcessSupervisor` APIs while adding the full Phase 4.2 production engine:

- `src/manager.rs`: `WineRuntimeManager` (`WINE_PINNED_VERSION = "11.0"`), runtime binary detection (`wine`, `wineserver`, `wineboot`), EXE/MSI launch specification builder (optional virtual desktop `explorer /desktop=App,<W>x<H>`), scoped `wineserver -k`/`-k9` command generation, and `TrackedWindowsProcess` with `/proc/<pid>/status` `VmRSS` lookup.
- `src/prefix.rs`: `WinePrefixManager` managing isolated prefixes (`create`, `list`, `load`, `configure`, `clone`, `backup` with `.tar.gz` + `.sha256`, `restore` with SHA-256 & Zip-Slip defense, `repair`, `delete`).
- `src/registry.rs`: Pure-Rust Windows binary header inspector (`inspect_windows_binary` / `inspect_windows_binary_bytes` supporting PE32 i386, PE32+ x86-64, PE32+ ARM64 rejection, and OLE2 `.msi` packages) plus `WineRegistryManager` (`WINEDLLOVERRIDES` formatter and `reg add` builders).
- `src/error.rs`: Structured `WineError` enum covering binary inspection failures, missing prefixes, invalid IDs, checksum mismatches, and I/O errors.

## 2. Binary Architecture Support Matrix

| Binary Format | COFF Machine / Magic | Supported via Wine 11.0 WoW64 + Box64 | Action |
|---|---|---|---|
| Windows x86-64 `.exe` | `PE32+` (`0x8664`) | Yes (pure-64-bit) | Launch via `box64 wine <exe>` |
| Windows x86 32-bit `.exe` | `PE32` (`0x014C`) | Yes (new WoW64, no Box86 required) | Launch via `box64 wine <exe>` |
| Windows Installer `.msi` | OLE2 (`D0 CF 11 E0 ...`) | Yes | Launch via `box64 wine msiexec /i <msi>` |
| Windows ARM64 `.exe` | `PE32+` (`0xAA64`) | Unsupported on x86-64 Wine | Return clear diagnostic error |
| Non-PE / Corrupted file | Unknown magic | Rejected | Return `WineError::UnsupportedBinary` |

## 3. Runtime Manifest & Artifact Staging

Runtime metadata is tracked in `/runtime/wine/manifest.json` and pinned in `rust/crates/downloader/src/artifact.rs` (`wine-11.0-wow64-amd64`).
Expected staged layout inside the app artifact directory or guest rootfs:
- `<artifacts>/wine/11.0/bin/wine`
- `<artifacts>/wine/11.0/bin/wineserver`
- `<artifacts>/wine/11.0/lib/wine/{x86_64-unix, x86_64-windows, i386-windows}`

## 4. Environment Variables Applied Per Launch

- `WINEPREFIX=<isolated_prefix_dir>`
- `WINEARCH=win64`
- `WINEDEBUG=-all,err+module,fixme-all` (overridable per app for diagnostics)
- `WINEESYNC=0`, `WINEFSYNC=0` (safe default for unprivileged Android PRoot kernels)
- `WINEDLLOVERRIDES=<computed by WineRegistryManager + DxTranslationManager>`
- `DISPLAY=:1` (connected to existing Stage 3.2 `Xvnc` / `ANativeWindow` display)
- `PULSE_SERVER=tcp:127.0.0.1:4713` (existing `AudioTcpSink`)

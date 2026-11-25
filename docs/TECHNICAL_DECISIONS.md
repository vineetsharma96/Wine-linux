# Technical decisions

[Project decision records](research/TECHNICAL_DECISIONS.md)

## Phase 4.1

- **Pure-Rust ELF inspection & explicit Box64 routing.** `androidpc-box64` (`rust/crates/box64`) parses ELF headers, `PT_INTERP`, and `SHT_DYNAMIC`/`DT_NEEDED` directly in Rust before executing binaries inside PRoot. ARM64 (`EM_AARCH64`) binaries execute natively without Box64, `EM_X86_64` ELF64 binaries route through Box64 with per-environment configuration, and unsupported architectures or Bionic-linked binaries fail with structured diagnostics rather than blind wrapping or root-only `binfmt_misc`. [Compatibility architecture](research/ARM64_X86_COMPATIBILITY.md)
- **Strict glibc vs. Bionic ABI separation.** Box64 `v0.4.4` (MIT) is cross-compiled for `aarch64-linux-gnu` (guest Linux glibc) via `scripts/build_box64_arm64.sh` (`-DARM64=1 -DARM_DYNAREC=ON -DBAD_SIGNAL=ON -DBOX32=OFF -DCMAKE_BUILD_TYPE=RelWithDebInfo`), never linked against Android NDK Bionic, and staged inside `<guest_rootfs>/usr/local/bin/box64`. `Box64EnvironmentConfig` rejects Android/Bionic library paths and strips host `LD_PRELOAD`. [Box64 Android build pipeline](research/BOX64_ANDROID_BUILD.md)
- **Conservative per-environment defaults.** Box64 settings live per environment (`EnvironmentConfig::box64`) with `BOX64_DYNAREC=1`, `SAFEFLAGS=1`, `STRONGMEM=1`, `BIGBLOCK=1`, `BOX64_LOG=1` (Errors), and `BOX64_SHOWSEGV=1` / `BOX64_SHOWBT=1`. Experimental Dynarec flags are never enabled automatically, and Interpreter fallback (`BOX64_DYNAREC=0`) is exposed directly in the Compatibility UI. [Box64 research](research/BOX64_RESEARCH.md)
- **Star Plus container-centric Android UI.** Redesigned the Android Compose shell around a dark obsidian/starlight theme, bundled Space Grotesk / Inter / JetBrains Mono typography, `ModalNavigationDrawer` with `BackHandler` support, container-first dashboard cards, and a dedicated `CompatibilityScreen` exposing Box64 status, simple enable/mode controls, collapsed advanced settings, ELF runner, and Categories A–D test suite.

## Stage 3.2

- **Built-in viewport.** RFB decodes into one CPU framebuffer. The Rust presenter copies a dirty generation directly into a checked ANativeWindow buffer. Canvas provides compatibility on the same transport. Android manages surface creation, destruction and producer coordination. [Architecture decision](research/DISPLAY_ARCHITECTURE_DECISION.md#stage-32-decision)

## Stage 3.1

- **APT source migration.** The guest package architecture and release gate a rewrite of recognized Ubuntu archive URLs. Ubuntu suites, components, signing options, and third-party mirrors remain under guest control. The first original is recoverable beside each changed file. [Guest APT research](research/ANDROID_LINUX_RESEARCH.md#stage-31-guest-package-sources)
- **Desktop lifecycle.** Rust scopes shutdown to the desktop launch that owns the PRoot child. Android activity changes may detach the viewport while the desktop remains available. [Session ownership](research/DISPLAY_ARCHITECTURE_DECISION.md#stage-31-session-ownership)

# Box64 Integration Guide (Phase 4.1)

*Last updated: 2026-11-25*
*Pinned release: Box64 `v0.4.4` (MIT) — `https://github.com/ptitSeb/box64`*

## 1. Architecture & Responsibility

Box64 translates x86-64 Linux ELF binaries—including the x86-64 Wine `11.0` loader and its x86-64 Unix helper libraries—to native ARM64 instructions inside our rootless PRoot Ubuntu/Debian guests.

All Box64 logic is owned by **`rust/crates/box64` (`androidpc-box64`)**:
- `elf.rs`: Pure-Rust ELF32/ELF64 parser detecting `EM_AARCH64`, `EM_X86_64`, `PT_INTERP`, `DT_NEEDED`, and rejecting Android Bionic executables/libraries from crossing into the glibc guest runtime.
- `resolver.rs`: Guest library path resolver checking `<rootfs>/usr/lib/x86_64-linux-gnu` and Box64 wrapped libraries before launch.
- `env.rs`: Per-environment `Box64EnvironmentConfig` and per-app override merger. Strips host `LD_PRELOAD` and blocks `/system/lib64` or `/vendor/lib64` Bionic paths.
- `router.rs`: Routes ARM64 ELF binaries directly to native execution, routes x86-64 ELF64 binaries through Box64, and returns structured errors for unsupported architectures. Maintains a bounded execution history ring buffer.
- `suite.rs`: Built-in Categories A–D verification suite (`CompatibilityScreen.kt` + `scripts/build_box64_test_binaries.sh`).

## 2. Default Conservative Configuration

Every Linux environment initializes `Box64EnvironmentConfig` with safe defaults:
- `BOX64_DYNAREC=1` (with explicit UI switch to Interpreter mode `BOX64_DYNAREC=0`)
- `BOX64_DYNAREC_SAFEFLAGS=1`
- `BOX64_DYNAREC_STRONGMEM=1`
- `BOX64_DYNAREC_BIGBLOCK=1`
- `BOX64_LOG=1` (Errors)
- `BOX64_SHOWSEGV=1`, `BOX64_SHOWBT=1`
- Default rcfile packaged at `app/src/main/assets/box64/default.box64rc`.

## 3. Reproducible Build & ABI Separation

Box64 MUST be compiled for **`aarch64-linux-gnu` (glibc)**—never Android NDK Bionic—so it executes inside the PRoot Ubuntu/Debian rootfs and wraps guest glibc libraries cleanly.

Run `scripts/build_box64_arm64.sh` (or `scripts/build_phase4_stack.sh`) on a Linux x86-64/ARM64 host with `aarch64-linux-gnu-gcc` and `cmake`:
```bash
cmake -B build \
  -DARM64=1 \
  -DARM_DYNAREC=ON \
  -DBAD_SIGNAL=ON \
  -DBOX32=OFF \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo
```
The build script automatically validates `readelf -l` to verify `PT_INTERP` is `/lib/ld-linux-aarch64.so.1` and fails if Bionic `/system/bin/linker64` is detected.

# Box64 Technical Investigation (Phase 4.1)

**Status:** Architecture & Rust integration implemented; on-device Xiaomi Pad 7 execution pending.  
**Pinned Upstream Version:** `v0.4.4` (MIT License)  
**Upstream Repository:** <https://github.com/ptitSeb/box64>  
**Official Documentation:** <https://box64.org/>  

---

## 1. Executive Summary

Box64 is a Linux userspace x86-64 emulator designed to run x86-64 Linux ELF binaries on non-x86-64 Linux hosts (primarily 64-bit ARM64/AArch64). Rather than emulating a full machine (like QEMU system mode) or translating every shared library from scratch, Box64 combines:

1. An **x86-64 ELF loader** and userspace dynamic linker shim.
2. An **ARM64 Dynamic Recompiler (Dynarec)** with a full **Interpreter fallback**.
3. **Native ARM64 shared library wrapping**, allowing x86-64 binaries to call host ARM64 glibc, libm, libpthread, libX11, OpenGL, Vulkan, SDL2, and similar libraries directly at native speed.

For **Android PC Environment**, Box64 runs **inside the ARM64 Ubuntu/Debian Linux guest rootfs via PRoot**, keeping Android host/Bionic binaries strictly isolated from the Linux glibc userspace.

---

## 2. Architecture Deep Dive

### 2.1 ARM64 Host Support & Page Size Requirements
- Box64 natively targets `aarch64-linux-gnu` (`-DARM64=1`).
- Standard ARM64 Dynarec assumes a **4 KiB host kernel page size**. Android 15+ devices configured with 16 KiB pages require compatible memory protection handling and must be explicitly verified on target hardware before claiming Dynarec stability.
- Target validation hardware: **Xiaomi Pad 7** (ARM64 Snapdragon SoC, Android host, PRoot Ubuntu/Debian ARM64 guest).

### 2.2 x86-64 Instruction Translation: Dynarec vs. Interpreter
- **Dynamic Recompiler (`BOX64_DYNAREC=1`)**: Translates x86-64 basic blocks into native AArch64 machine code JIT buffers (`mmap` with RWX / W^X-safe transitions depending on kernel policy). Provides the vast majority of Box64's performance advantage over pure interpretation.
- **Conservative Dynarec Controls**:
  - `BOX64_DYNAREC_SAFEFLAGS=1`: Preserves x86 EFLAGS behavior across block boundaries without enabling aggressive unsafe flag elision (`0`).
  - `BOX64_DYNAREC_STRONGMEM=1`: Inserts ARM64 memory barriers for multi-threaded x86 TSO (Total Store Order) memory-ordering correctness on weakly ordered ARM64 cores.
  - `BOX64_DYNAREC_BIGBLOCK=1`: Standard block chaining without experimental superblock expansion (`2`/`3`).
  - `BOX64_DYNAREC_FASTNAN=0` and `BOX64_DYNAREC_FASTROUND=0`: Preserves strict IEEE-754 NaN/rounding semantics by default.
- **Interpreter Fallback (`BOX64_DYNAREC=0`)**: Pure C interpreter loop. Slower than Dynarec, but immune to JIT code-cache protection issues (`W^X` / SELinux `execmem` restrictions) and serves as the primary diagnostic baseline when isolating translation bugs.

### 2.3 ELF Loader & Dynamic Library Resolution
- Box64 inspects `PT_INTERP`, `PT_LOAD`, `PT_TLS`, and `SHT_DYNAMIC` (`DT_NEEDED`, `DT_RPATH`, `DT_RUNPATH`) of target `EM_X86_64` ELF64 executables.
- **Wrapped Native Libraries**: For well-known system libraries (`libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libdl.so.2`, `librt.so.1`, `libz.so.1`, `libX11.so.6`, `libGL.so.1`, `libvulkan.so.1`, etc.), Box64 intercepts `dlopen`/symbol resolution and bridges the System V AMD64 ABI to the ARM64 AAPCS64 ABI, invoking the guest's native ARM64 `.so`.
- **Emulated x86-64 Libraries**: For unwrapped or app-bundled x86-64 `.so` files (e.g., `libstdc++.so.6` x86-64 builds, custom plugins), Box64 loads them from `BOX64_LD_LIBRARY_PATH` (configured per environment, defaulting to `/usr/lib/box64-x86_64-linux-gnu:/usr/local/lib/box64-x86_64-linux-gnu:/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu`).
- **32-bit x86 (Box32)**: Box64 `v0.4.x` includes experimental `BOX32` support, which is **disabled by default** in Phase 4.1 (`-DBOX32=OFF`) to avoid unverified 32-bit compatibility regressions.

### 2.4 Guest glibc Compatibility & ABI Isolation
- Box64 executes inside the PRoot Ubuntu/Debian ARM64 rootfs and links dynamically against **guest ARM64 glibc** (`ld-linux-aarch64.so.1`).
- **Strict Separation Rule**: Guest x86-64 libraries and the Box64 binary MUST NEVER resolve against Android host Bionic libraries (`/system/lib64`, `/apex/...`, `/vendor/lib64`, `/system/bin/linker64`).
- In `androidpc-box64`:
  - `elf::inspect_elf_bytes` flags any binary whose `PT_INTERP` references Android `linker`/`linker64`.
  - `Box64EnvironmentConfig::validate` rejects any `BOX64_LD_LIBRARY_PATH` or `BOX64_PATH` entry referencing Android/Bionic paths.
  - `env::build_box64_env_vars` strips host `LD_PRELOAD` and enforces guest-only library search paths.

### 2.5 Signal Handling, Threading & Memory Management
- **Signal Handling (`-DBAD_SIGNAL=ON`)**: Box64 installs its own `SIGSEGV`/`SIGBUS`/`SIGILL` handlers to intercept Dynarec self-modifying code (SMC) writes and guest faults, translating native `ucontext_t` state back to x86-64 register context. Enabling `-DBAD_SIGNAL=ON` is essential on Android kernels/PRoot where signal frame layouts or `sigaltstack` behavior can differ from desktop Linux.
- **Threading**: Each x86-64 `pthread` maps to a native guest Linux thread (`clone`). Weak ARM64 memory ordering requires `BOX64_DYNAREC_STRONGMEM=1` (conservative default) to avoid lock-free/atomic data races in multi-threaded x86-64 code.
- **Memory Management**: Box64 reserves userspace address mappings (`mmap`) for the x86-64 47-bit virtual address space and JIT code cache. Android devices with 48-bit VA or strict `RLIMIT_AS` / LMK (Low Memory Killer) policies can fail large allocations; our diagnostics surface `SIGSEGV`, `SIGKILL` (OOM), and `SIGILL` distinctly.

### 2.6 Vulkan / OpenGL Integration Requirements
- Box64 wraps `libGL.so.1`, `libEGL.so.1`, and `libvulkan.so.1` to the guest ARM64 graphics stack.
- In Phase 4.1, **hardware GPU acceleration is NOT assumed**. Inside PRoot without Turnip/Zink/VirGL ICDs configured, OpenGL/Vulkan applications must either run headless/CLI or fall back to Mesa software rendering (`llvmpipe` / `lavapipe`) inside the guest rootfs.
- Full GPU acceleration (Turnip/Zink/VirGL) remains out of scope for Phase 4.1 and is tracked separately.

### 2.7 PRoot Compatibility
- PRoot intercepts syscalls via `ptrace` to virtualize rootfs paths (`/usr`, `/lib`, `/proc/self/exe`).
- **Key PRoot Considerations**:
  - `PRoot` overhead applies on syscalls (not inside pure Dynarec CPU loops), so CPU-bound x86-64 workloads retain most Dynarec speedup while syscall-heavy workloads incur PRoot `ptrace` latency.
  - `/proc/self/exe` and `readlink` translation under PRoot must resolve cleanly so Box64 can locate its binary path and relative libraries; staging Box64 into `/usr/local/bin/box64` inside the guest rootfs avoids host path leakage.
  - `SIGSEGV` self-modifying code traps under `ptrace` add overhead compared to native kernels; `-DBAD_SIGNAL=ON` and `BOX64_DYNAREC_SAFEFLAGS=1` minimize signal-handling edge cases.
  - No root permissions or `binfmt_misc` kernel registration are required; the Rust `Box64CompatibilityLayer` handles ELF inspection and explicit invocation.

---

## 3. Research Audit: Confirmed Facts vs. Hypotheses vs. Unknowns

### Confirmed Facts (Verified from Official Upstream & Codebase Inspection)
- Official Box64 upstream is `ptitSeb/box64`, licensed under **MIT License** (`LICENSE` file must accompany packaged binaries).
- Latest stable release tag verified as **`v0.4.4`** (upstream GitHub release).
- Box64 builds cleanly for ARM64 Linux glibc via CMake (`-DARM64=1 -DARM_DYNAREC=ON -DBAD_SIGNAL=ON`).
- Android PC Environment runs ARM64 Ubuntu/Debian guests via PRoot without root, using Rust (`androidpc-core`, `androidpc-linux-runtime`) bridged via JNI (`androidpc-android-bridge`) to Kotlin/Compose UI.

### Working Hypotheses (Implemented in Rust; Awaiting On-Device Verification)
- Staging an `aarch64-linux-gnu` glibc-linked Box64 `v0.4.4` binary into `<guest_rootfs>/usr/local/bin/box64` with `BOX64_DYNAREC=1`, `SAFEFLAGS=1`, `STRONGMEM=1`, `BIGBLOCK=1` will execute static and glibc-linked x86-64 CLI binaries inside PRoot Ubuntu/Debian on Xiaomi Pad 7.
- Per-environment `BOX64_LD_LIBRARY_PATH` isolation will prevent accidental Android Bionic linkage.

### Unknowns / Open Risks (Require Physical Xiaomi Pad 7 Execution)
- Exact Dynarec JIT `mmap(PROT_READ|PROT_WRITE|PROT_EXEC)` behavior under Xiaomi Pad 7's Android SELinux policy inside PRoot (if `execmem` is restricted, Interpreter mode `BOX64_DYNAREC=0` serves as fallback).
- Host kernel page size and PRoot `ptrace` interaction with Box64 `SIGSEGV` SMC trapping under heavy multi-threaded x86-64 loads.
- Actual glibc version compatibility between the cross-compilation sysroot (Ubuntu/Debian `aarch64-linux-gnu`) and the downloaded guest rootfs glibc version (building on an older LTS glibc baseline such as Ubuntu 22.04 / Debian 12 minimizes `GLIBC_2.38+` symbol mismatches).

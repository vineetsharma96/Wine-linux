# ARM64 & x86-64 Compatibility Layer Architecture (Phase 4.1)

**Crate:** `rust/crates/box64` (`androidpc-box64`)  
**Integration Points:** `androidpc-configuration`, `androidpc-linux-runtime`, `androidpc-core`, `androidpc-android-bridge`  

---

## 1. Design Principles

1. **Rust-Owned Execution Routing**: Binary architecture detection, ELF header parsing, dynamic library diagnostics, and Box64 environment variable assembly live exclusively in Rust (`androidpc-box64`). Kotlin never parses ELFs or constructs Box64 command lines.
2. **Preserve Existing Process Supervision**: Box64 integrates through `ProotEnvironmentManager` and `EngineCore` without replacing the existing Linux runtime, process manager, or XFCE/Xvnc display pipeline.
3. **No Blind Wrapping**: ARM64 binaries execute natively inside PRoot without Box64 overhead. Only `EM_X86_64` ELF64 binaries route through Box64.
4. **No Root or `binfmt_misc` Dependency**: Routing is performed explicitly at execution time after inspecting the target file inside the guest rootfs.

---

## 2. Binary Execution Flow

```text
User / App requests execution of <guest_path> [args...]
        │
        ▼
[1] Path Resolution inside <rootfs_path>
    (resolves relative/PATH commands and #! script interpreters up to depth 4)
        │
        ▼
[2] Pure-Rust ELF Inspection (rust/crates/box64/src/elf.rs)
    Parses ELF header (e_ident, e_type, e_machine), PT_INTERP, SHT_DYNAMIC (DT_NEEDED, DT_RPATH, DT_RUNPATH)
        │
        ├──► Non-ELF / Script with ARM64 interpreter / EM_AARCH64 (183)
        │       └──► Native ARM64 execution via PRoot (zero Box64 involvement)
        │
        ├──► EM_X86_64 (62) + ELF64
        │       ├── Check: uses Android Bionic linker? ──► Reject (ABI isolation error)
        │       ├── Check: Box64 enabled in env config? ──► Reject if disabled
        │       ├── Check: Box64 binary staged & executable? ──► Stage from assets/artifacts or return actionable install error
        │       ├── Resolve DT_NEEDED against Box64 wrapped native libs + guest x86-64 library paths
        │       ├── Build per-env Box64 env vars (+ per-app overrides, strip LD_PRELOAD, block Bionic paths)
        │       └──► Route: PRoot -> /usr/local/bin/box64 <target> [args...]
        │
        └──► EM_386 (3), EM_ARM (40), EM_RISCV (243), EM_PPC64, corrupt ELF, etc.
                └──► Return structured CompatibilityError (never silently invoke Box64)
```

---

## 3. Core Rust Modules (`rust/crates/box64/src/`)

| Module | Responsibility |
| :--- | :--- |
| `elf.rs` | Pure-Rust ELF32/ELF64 parser (`inspect_elf_bytes`, `inspect_guest_binary`). Detects magic, class, endianness, `e_machine`, `PT_INTERP`, `DT_NEEDED`, `DT_RPATH`, `DT_RUNPATH`, Android Bionic linkage, and `#!` shebang interpreters. |
| `env.rs` | Builds isolated per-environment `BOX64_*` environment variables (`build_box64_env_vars`) with application-specific overrides (`Box64AppOverride`), blocks Android Bionic paths, strips `LD_PRELOAD`, and renders optional `.box64rc` (`render_box64_rcfile`). |
| `resolver.rs` | Resolves `DT_NEEDED` shared libraries (`resolve_guest_libraries`) against Box64's built-in wrapped native ARM64 libraries (e.g., `libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libX11.so.6`, `libvulkan.so.1`) and configured guest x86-64 search paths, reporting missing libraries and ABI mismatches. |
| `router.rs` | `Box64CompatibilityLayer`: probes host/guest Box64 installation, stages `box64` into `<rootfs>/usr/local/bin/box64`, plans execution (`plan_execution`), executes with captured stdout/stderr and duration (`execute_and_capture`), classifies signals/exit codes (`SIGSEGV`, `SIGILL`, `SIGABRT`, `SIGKILL`, `127`, etc.), and retains bounded execution history. |
| `suite.rs` | Structured compatibility test suite covering Categories A, B, C, and D (`run_compatibility_suite`), returning per-case pass/fail/skipped diagnostics and summaries. |

---

## 4. Per-Environment Configuration (`Box64EnvironmentConfig`)

Stored inside each environment's `EnvironmentConfig` (`rust/crates/configuration/src/lib.rs`) with `#[serde(default)]` for backwards compatibility with existing saved environments:

| Field | Conservative Default | Notes |
| :--- | :--- | :--- |
| `enabled` | `true` | Toggle Box64 routing per environment without affecting other environments. |
| `binary_version` | `"0.4.4"` | Pinned Box64 release identifier. |
| `dynarec_enabled` | `true` | Switches `BOX64_DYNAREC` (`1` = Dynarec, `0` = Interpreter). |
| `dynarec_safeflags` | `1` | Conservative flag preservation (`0..=2`). |
| `dynarec_strongmem` | `1` | ARM64 memory-ordering barriers for multi-threading (`0..=3`). |
| `dynarec_bigblock` | `1` | Standard block size (`0..=3`); no experimental superblocks by default. |
| `log_level` | `Errors` (`BOX64_LOG=1`) | Surfaces loader/symbol errors without flooding logs. |
| `show_segv` / `show_bt` | `true` / `true` | Captures guest fault addresses and backtraces on crashes. |
| `guest_library_paths` | `/usr/lib/box64-x86_64-linux-gnu`, `/usr/local/lib/box64-x86_64-linux-gnu`, `/lib/x86_64-linux-gnu`, `/usr/lib/x86_64-linux-gnu` | Validated to forbid `/system`, `/vendor`, `/apex`, `/odm`, `bionic`. |
| `app_overrides` | `[]` | Per-binary `dynarec_enabled`, `log_level`, `extra_env` overrides matched by executable name. |

---

## 5. Compatibility Test Suite Design (Categories A–D)

Implemented in `rust/crates/box64/src/suite.rs` and exposed through JNI (`nativeRunBox64Suite`) to the Android **Compatibility** screen:

- **Category A — Native ARM64 Execution**:
  - `A1`: ARM64 ELF header detection (`/bin/sh` or `/usr/bin/env`).
  - `A2`: Native ARM64 shell execution (`echo arm64-native-ok`) verifying zero Box64 interception.
  - `A3`: Standard Linux utility execution (`uname -m`).
- **Category B — x86-64 Execution via Box64**:
  - Uses self-authored C probe compiled by `scripts/build_box64_test_binaries.sh` into `<rootfs>/opt/androidpc-tests/compat_probe_x86_64` (and `compat_probe_arm64` for baseline comparison).
  - Tests ELF64 loading, integer/FP arithmetic, stdio, temp file read/write, 8 MiB heap allocation, 4-thread `pthread` concurrency, and `SIGUSR1` signal delivery.
  - Curated open-source x86-64 Linux CLI candidates for extended on-device validation: `jq`, `busybox` (x86-64), and `sqlite3`.
- **Category C — Failure & Edge-Case Handling**:
  - `C1`: Missing shared library diagnostic (`DT_NEEDED` referencing nonexistent `.so`).
  - `C2`: Unsupported architecture rejection (e.g., `EM_RISCV` / `EM_386` ELF).
  - `C3`: Corrupt/truncated ELF detection.
  - `C4`: Android Bionic ABI isolation rejection (`PT_INTERP = /system/bin/linker64`).
  - `C5`: Signal/crash & non-zero exit capture (`compat_probe_x86_64 exit_42` / `crash_segv` when binary is installed).
- **Category D — Stability**:
  - Repeated sequential x86-64 invocations (`compat_probe_x86_64`) measuring exit-code consistency and RSS/latency stability without process leaks.

### Verification Status (Strict Honesty)
- **Implemented**: Rust ELF parser, library resolver, env builder, router, unit tests (synthetic ELF64 ARM64/x86-64/RISC-V/Bionic/corrupt fixtures), JNI bridge, and Android UI.
- **Not Yet Executed on Device**: `cargo test --workspace` (requires Rust toolchain host) and physical Xiaomi Pad 7 Categories A–D execution (requires cross-compiled Box64 `v0.4.4`, rebuilt `libandroidpc_engine.so`, and physical ARM64 Android hardware). No x86-64 application compatibility is claimed until executed tests pass on device.

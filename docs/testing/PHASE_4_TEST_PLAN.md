# Phase 4 Independent Testing & Validation Handoff Plan

*Last updated: 2026-11-25*
*Target Device: **Xiaomi Pad 7** (`2410CRP4CI`, Snapdragon 7+ Gen 3, Adreno 732 GPU, Android 16, ARM64)*
*Purpose: Complete build, test execution, benchmark, and compatibility verification handoff for the dedicated Phase 4 testing agent.*

> [!IMPORTANT]
> **Execution Honesty Notice**: This implementation turn verified **Android APK compilation (`compile_applet` — PASS)**. All Rust workspace tests (`cargo test --workspace`), ARM64 NDK JNI builds (`cargo-ndk`), cross-compiled guest runtime builds (`scripts/build_phase4_stack.sh`), and **ALL physical Xiaomi Pad 7 device tests (Sections A through L)** are **`NOT RUN`** and MUST be executed and recorded with real evidence by the independent testing agent.

---

## 1. Host Build & ARM64 Packaging Instructions

Run the following on the Linux/Windows Rust + Android NDK host before installing on Xiaomi Pad 7:

```bash
# 1. Run full Rust workspace unit & integration tests (all 20 crates)
cd rust && cargo test --workspace

# 2. Rebuild ARM64 JNI engine library (includes Phase 4.1–4.9 JNI exports)
cargo ndk -t arm64-v8a -o ../app/src/main/jniLibs build --release
cd ..

# 3. Cross-compile Box64 v0.4.4 (aarch64-linux-gnu glibc) & self-authored test probes
./scripts/build_phase4_stack.sh --skip-ndk

# 4. Stage verified runtime artifacts into device/container artifact directory:
#    - Box64 v0.4.4 glibc binary -> <rootfs>/usr/local/bin/box64
#    - Wine 11.0 x86-64 pure-64-bit WoW64 -> <artifacts>/wine/11.0/
#    - Mesa Turnip 26.3.1 (libvulkan_freedreno.so + freedreno_icd.aarch64.json) -> <artifacts>/graphics/turnip/
#    - virglrenderer 1.3.0 (virgl_test_server_android) -> <artifacts>/graphics/virgl/
#    - DXVK v2.7.1 (x64/*.dll, x32/*.dll) -> <artifacts>/dxvk/2.7.1/
#    - VKD3D-Proton v3.1 (x64/*.dll, x86/*.dll) -> <artifacts>/vkd3d/3.1/

# 5. Build and install debug APK on Xiaomi Pad 7
gradle :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

---

## 2. Required Test Campaigns (Sections A through L)

Every test case below starts in status **`NOT RUN`**. The testing agent must replace `NOT RUN` with `PASS`, `FAIL`, or `BLOCKED` accompanied by raw logs/screenshots/exit codes.

### Section A — Box64 Execution & ELF Routing (`rust/crates/box64`)
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| A1 | Execute native ARM64 `compat_probe` inside PRoot Ubuntu 26.04.1 & Debian 13.8 | Routed directly (no Box64 wrapper), exit code `0` | **NOT RUN** |
| A2 | Execute x86-64 `compat_probe` with Box64 Dynarec (`BOX64_DYNAREC=1`) | Routed via Box64 `v0.4.4`, exit code `0`, structured telemetry recorded | **NOT RUN** |
| A3 | Execute x86-64 `compat_probe` with Box64 Interpreter (`BOX64_DYNAREC=0`) | Runs without Dynarec crash, exit code `0` | **NOT RUN** |
| A4 | Attempt to execute Android Bionic ELF or unsupported arch (e.g. RISC-V) | Rejected pre-launch with structured diagnostic error | **NOT RUN** |
| A5 | Missing guest x86-64 `.so` library test | `resolver.rs` reports exact missing `DT_NEEDED` library | **NOT RUN** |

### Section B — Wine 11.0 WoW64 Initialization & Prefix Management (`rust/crates/wine`)
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| B1 | Verify `wine --version` via Box64 inside PRoot | Reports `wine-11.0`, zero Bionic ABI errors | **NOT RUN** |
| B2 | Create isolated Prefix A (`win10`, WoW64) & Prefix B (`win11`, WoW64) | Independent `drive_c`, `prefix.json`, and `dosdevices/c:` created | **NOT RUN** |
| B3 | Clone Prefix A → Prefix A-Copy | Distinct file inodes; edits in copy do not mutate Prefix A | **NOT RUN** |
| B4 | Backup Prefix A (`.tar.gz` + `.sha256`) and Restore into Prefix C | SHA-256 verified; restored prefix boots cleanly | **NOT RUN** |
| B5 | Tamper 1 byte in backup archive and attempt Restore | Rejected with `ChecksumMismatch` before extraction | **NOT RUN** |
| B6 | Repair damaged prefix (delete `dosdevices/c:` symlink & run Repair) | Symlink & missing dirs recreated without losing `drive_c/users` | **NOT RUN** |

### Section C — Windows EXE (PE32 & PE32+) and MSI Installation & Launch
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| C1 | Launch built-in `notepad.exe` / `winecfg` on XFCE `DISPLAY=:1` | Window renders inside Stage 3.2 `ANativeWindow` viewport | **NOT RUN** |
| C2 | Install & launch a 64-bit Windows PE32+ utility (e.g., 7-Zip x64 `.exe`) | PE32+ detected, installs into prefix `drive_c`, launches cleanly | **NOT RUN** |
| C3 | Install & launch a 32-bit Windows PE32 utility (WoW64 without Box86) | PE32 detected, runs under Wine 11.0 pure-64-bit WoW64 | **NOT RUN** |
| C4 | Install a standard `.msi` package via `msiexec /i` | OLE2 magic detected, installs cleanly into isolated prefix | **NOT RUN** |
| C5 | Attempt to launch a Windows ARM64 PE32+ (`0xAA64`) or corrupt file | Rejected pre-launch with clear error message | **NOT RUN** |

### Section D — Scoped Windows Process Cleanup & Resource Tracking
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| D1 | Run App 1 in Prefix A and App 2 in Prefix B simultaneously | Both appear in `TrackedWindowsProcess` with real `VmRSS` (or `Unavailable`) | **NOT RUN** |
| D2 | Graceful Stop (`wineserver -k`) on App 1 | Prefix A stops cleanly; Prefix B, XFCE, and Ubuntu PTY remain alive | **NOT RUN** |
| D3 | Force Stop (`wineserver -k9`) on hung process in Prefix A | Prefix A terminates within bounded timeout; zero collateral kills | **NOT RUN** |
| D4 | Attempt to delete Prefix B while App 2 is running | Refused with active-process guard error | **NOT RUN** |

### Section E — Graphics Backend Detection & Readiness Stage Integrity
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| E1 | Probe with no Turnip/VirGL binaries staged | Both report `not_installed`; effective backend = `software` (`llvmpipe`) | **NOT RUN** |
| E2 | Place `libvulkan_freedreno.so` without live Vulkan verification | Stage stops at `installed`/`detected` (NEVER `operational`); fallback to `software` | **NOT RUN** |
| E3 | Verify Software (`llvmpipe`) OpenGL rendering with `glxgears` / WineD3D | Renders cleanly with honest `software` badge | **NOT RUN** |

### Section F — Mesa Turnip `26.3.1` on Xiaomi Pad 7 (Adreno 732)
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| F1 | Verify unprivileged access to `/dev/kgsl-3d0` inside PRoot | `TurnipBackend` reaches `detected` without root | **NOT RUN** |
| F2 | Run `vulkaninfo --summary` with Mesa Turnip `26.3.1` ICD | Reports `Turnip Adreno 732`, Vulkan 1.3+, stage reaches `operational` | **NOT RUN** |
| F3 | Run `vkcube` / Zink OpenGL test inside `DISPLAY=:1` | Hardware-rendered frames visible in `ANativeWindow` viewport | **NOT RUN** |
| F4 | Confirm fallback behavior if `/dev/kgsl-3d0` is unavailable/denied | Falls back cleanly to `software` with diagnostic reason | **NOT RUN** |

### Section G — VirGL `1.3.0` OpenGL-Only Backend
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| G1 | Start `virgl_test_server_android` on short `/tmp/.virgl_test` socket | Socket path `< 108` bytes; `VirglBackend` reaches `operational` | **NOT RUN** |
| G2 | Run OpenGL app / WineD3D with `GALLIUM_DRIVER=virpipe` | Renders via host Android GLES; `supports_vulkan == false` | **NOT RUN** |
| G3 | Attempt to enable DXVK or VKD3D while VirGL is active | Strictly rejected by `DxTranslationManager`; stays on `WineD3D` OpenGL | **NOT RUN** |

### Section H — DXVK `v2.7.1` & VKD3D-Proton `v3.1` Gating & Execution
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| H1 | Enable DXVK on Turnip `Operational` backend | DLLs staged to `system32`/`syswow64`; D3D9/D3D11 test renders via Vulkan | **NOT RUN** |
| H2 | Enable DXVK on `Software` or `VirGL` backend | Automatic fallback to `WineD3D` OpenGL with clear diagnostic reason | **NOT RUN** |
| H3 | Inspect VKD3D-Proton 6 mandatory Vulkan extensions on Adreno 732 | Exact extension pass/missing list reported (never fabricated) | **NOT RUN** |
| H4 | If all 6 extensions pass on Adreno 732, run minimal D3D12 triangle sample | Document pass/fail/GPU hang honestly; otherwise confirm VKD3D stays gated | **NOT RUN** |

### Section I — Clipboard, Input & XFCE Integration
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| I1 | Copy text on Android → Paste into Windows Notepad (`Ctrl+V`) | Text pastes cleanly via existing RFB `ClientCutText` | **NOT RUN** |
| I2 | Physical keyboard + touch/trackpad + IME typing into Windows app | Keystrokes and pointer clicks delivered accurately | **NOT RUN** |
| I3 | Generate XFCE `.desktop` shortcut for a registered Windows app | Shortcut appears in XFCE and launches app in its bound prefix | **NOT RUN** |

### Section J — Android Lifecycle, Backgrounding & Process Death
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| J1 | Background app (`Home`) while Windows app is open, then resume | Session survives; BLAST `SurfaceView` reconnects cleanly | **NOT RUN** |
| J2 | Rotate Xiaomi Pad 7 portrait ↔ landscape during Windows session | No crash; viewport resizes or preserves framebuffer cleanly | **NOT RUN** |
| J3 | `adb shell am force-stop com.androidpc.environment.debug` and reopen | Persisted prefixes/apps reload; stale PIDs cleared cleanly | **NOT RUN** |

### Section K — Storage, Dependency Diagnostics & Security
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| K1 | Run Dependency Manager scan on prefix with missing `vcruntime140.dll` | Detects missing DLL and recommends user-supplied official `vcrun2015-2022` | **NOT RUN** |
| K2 | Enable `wine-mono` + `dotnet-desktop-8` conflict check | Conflict flagged with explicit recovery hint | **NOT RUN** |
| K3 | Verify zero credentials leaked into `windows_apps/logs/*.log` | Env vars containing `TOKEN`, `SECRET`, `PASSWORD`, `KEY` redacted | **NOT RUN** |

### Section L — Existing Ubuntu 26.04.1 & Debian 13.8 Regression Suite
| ID | Scenario | Expected Result | Status |
|---|---|---|---|
| L1 | Start existing Ubuntu 26.04.1 & Debian 13.8 containers + PTY terminal | `uname -m`, `apt-get update`, and persistent files work unchanged | **NOT RUN** |
| L2 | Launch XFCE 4.20 desktop (`ANativeWindow` + Canvas fallback) | Frame rate and Exit Desktop cleanup match Stage 3.2 baseline | **NOT RUN** |
| L3 | Create/Restore existing Linux container backup | Functions identically to Phase 2/3 verification | **NOT RUN** |

---

## 3. Xiaomi Pad 7 Benchmark & Telemetry Script

Use `/scripts/benchmark_phase4_device.sh` to collect reproducible host metrics (App CPU %, PSS/RSS via `dumpsys meminfo`, `gfxinfo` jank/Vsync, thermal zone temperatures, and active Wine/Box64 processes):

```bash
chmod +x scripts/benchmark_phase4_device.sh
./scripts/benchmark_phase4_device.sh --serial <device_serial> --label phase4_turnip_dxvk --duration 60
```

Outputs are written to `benchmarks/phase4_<label>_<timestamp>/` for direct inclusion in the testing agent's final report.

---

## 4. Compatibility Matrix Template (To Be Filled by Testing Agent)

| Application | Arch | Graphics Backend | DX Layer | Launch | Render | Input/Clipboard | Clean Exit | Notes |
|---|---|---|---|---|---|---|---|---|
| `notepad.exe` | PE32+ | Software (`llvmpipe`) | WineD3D | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Baseline |
| `7zFM.exe` (x64) | PE32+ | Software / Turnip | WineD3D | NOT RUN | NOT RUN | NOT RUN | NOT RUN | PE32+ utility |
| 32-bit PE32 utility | PE32 | Software / Turnip | WineD3D | NOT RUN | NOT RUN | NOT RUN | NOT RUN | WoW64 validation |
| D3D9 / D3D11 sample | PE32+ | Turnip `26.3.1` | DXVK `2.7.1` | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Adreno 732 Vulkan |
| D3D11 sample (VirGL) | PE32+ | VirGL `1.3.0` | WineD3D | NOT RUN | NOT RUN | NOT RUN | NOT RUN | OpenGL fallback check |
| D3D12 minimal sample | PE32+ | Turnip `26.3.1` | VKD3D `3.1` | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Gated by 6 Vulkan ext. |

---

## 5. Known Implementation Caveats & Pre-Test Blockers

1. **Prebuilt `libandroidpc_engine.so` Needs Rebuild**: The checked-in `app/src/main/jniLibs/arm64-v8a/libandroidpc_engine.so` predates Phase 4.1–4.9 JNI additions. Until `cargo ndk -t arm64-v8a -o ../app/src/main/jniLibs build --release` is run, `WindowsCompatibilityBridge.kt` operates in its graceful `UnsatisfiedLinkError` inspection fallback mode.
2. **External Runtime Artifacts Must Be Staged**: Box64 `v0.4.4`, Wine `11.0`, Mesa Turnip `26.3.1`, `virglrenderer 1.3.0`, DXVK `v2.7.1`, and VKD3D-Proton `v3.1` binaries are not committed as multi-hundred-megabyte blobs into Git; stage them using `scripts/build_phase4_stack.sh` and official release tarballs matching `runtime/*/manifest.json`.
3. **Android 16 SELinux / KGSL & `wineserver` Behavior**: Must be validated empirically on Xiaomi Pad 7; no claim is made prior to hardware execution.

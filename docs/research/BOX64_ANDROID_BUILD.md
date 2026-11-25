# Box64 Android Build & Packaging Pipeline (Phase 4.1)

**Build Script:** `scripts/build_box64_arm64.sh`  
**Test Binary Script:** `scripts/build_box64_test_binaries.sh`  
**Target Asset Output:** `app/src/main/assets/box64/`  
**Pinned Version:** Box64 `v0.4.4` (MIT License)  

---

## 1. Critical ABI Decision: Linux Guest glibc vs. Android NDK Bionic

A frequent mistake when packaging Box64 for Android is compiling Box64 with the Android NDK (`aarch64-linux-android-clang`), which links against Android Bionic (`/system/bin/linker64`, `libc.so`).

**In Android PC Environment, Box64 MUST be compiled for ARM64 Linux glibc (`aarch64-linux-gnu-gcc`), NOT Android Bionic:**

| Component | Target Triple | C Library / Dynamic Linker | Execution Context |
| :--- | :--- | :--- | :--- |
| `libandroidpc_engine.so` (Rust JNI) | `aarch64-linux-android` | Android NDK / Bionic | Android app process |
| `proot` launcher binary | `aarch64-linux-android` | Android NDK / Bionic | Android host process spawning guest |
| **`box64` (`v0.4.4`)** | **`aarch64-linux-gnu`** | **Guest Linux glibc (`ld-linux-aarch64.so.1`)** | **Inside PRoot ARM64 Ubuntu/Debian rootfs** |
| Guest x86-64 apps & `.so` libs | `x86_64-linux-gnu` | Wrapped guest ARM64 glibc + x86-64 guest libs | Inside PRoot via Box64 |

**Why this separation is mandatory:**
- Box64 wraps x86-64 glibc/X11/SDL/Vulkan/OpenGL calls directly onto the **guest ARM64 rootfs's glibc libraries**.
- Linking Box64 against Android Bionic prevents it from cleanly wrapping Debian/Ubuntu ARM64 `glibc` libraries and causes symbol/ABI corruption between Bionic and glibc `pthread`/`errno`/`FILE*` structures.
- `scripts/build_box64_arm64.sh` explicitly runs `readelf -l` on the built binary and **aborts the build** if `/system/bin/linker` or `android` appears in the ELF interpreter.

---

## 2. Reproducible Cross-Compilation Pipeline (`scripts/build_box64_arm64.sh`)

Never compile Box64 on the user's Android device. Cross-compile reproducibly on an x86-64 Linux build host (Ubuntu 22.04 LTS / Debian 12 recommended so the resulting binary requires an older, broadly compatible `GLIBC_2.34–2.35` baseline).

### Host Prerequisites
```bash
sudo apt-get update
sudo apt-get install -y \
  git cmake ninja-build python3 \
  gcc-aarch64-linux-gnu g++-aarch64-linux-gnu \
  binutils-aarch64-linux-gnu curl ca-certificates
```

### Pinned CMake Configuration
```cmake
-DCMAKE_SYSTEM_NAME=Linux
-DCMAKE_SYSTEM_PROCESSOR=aarch64
-DCMAKE_C_COMPILER=aarch64-linux-gnu-gcc
-DCMAKE_CXX_COMPILER=aarch64-linux-gnu-g++
-DCMAKE_BUILD_TYPE=RelWithDebInfo
-DARM64=1
-DARM_DYNAREC=ON
-DBAD_SIGNAL=ON
-DBOX32=OFF
-DBOX32_BINFMT=OFF
-DNOGIT=ON
-DSAVE_MEM=OFF
```

### Rationale for Key Flags
- `-DARM64=1 -DARM_DYNAREC=ON`: Enables the AArch64 Dynamic Recompiler while retaining runtime fallback to Interpreter via `BOX64_DYNAREC=0`.
- `-DBAD_SIGNAL=ON`: Required for reliable signal frame handling across Android kernels and PRoot.
- `-DBOX32=OFF`: Keeps Phase 4.1 strictly scoped to 64-bit x86-64 stability without experimental 32-bit Box32 surfaces.
- `-DCMAKE_BUILD_TYPE=RelWithDebInfo`: Full compiler optimization while retaining symbols for crash diagnostics prior to optional stripping.

### Running the Build
```bash
./scripts/build_box64_arm64.sh
```

### Packaged Outputs (`app/src/main/assets/box64/`)
1. `box64`: Cross-compiled `aarch64-linux-gnu` ELF64 executable.
2. `default.box64rc`: Bundled conservative default rcfile (already committed).
3. `LICENSE`: Upstream Box64 MIT License text (mandatory license obligation).
4. `manifest.json`: Version (`0.4.4`), target triple, CMake flags, and SHA-256 checksum verified at install time.

At app startup, `AssetInstaller.kt` extracts `assets/box64/*` into `filesDir/artifacts/box64/` with executable permissions, and `Box64CompatibilityLayer::ensure_guest_box64_staged` copies it into `<guest_rootfs>/usr/local/bin/box64` (`0755`) for PRoot execution.

---

## 3. Rebuilding the Rust Native Engine (`libandroidpc_engine.so`)

Because Phase 4.1 adds the `androidpc-box64` crate and 6 new JNI exports (`nativeGetBox64Status`, `nativeGetBox64Config`, `nativeUpdateBox64Config`, `nativeExecuteCompatibleBinary`, `nativeRunBox64Suite`, `nativeGetBox64History`), rebuild the Android JNI library on the Rust/NDK host before testing on Xiaomi Pad 7:

```bash
# 1. Verify Rust workspace unit tests (includes synthetic ELF/routing tests)
cd rust
cargo test --workspace

# 2. Rebuild ARM64 Android JNI shared library
cargo ndk -t arm64-v8a -o ../app/src/main/jniLibs build --release -p androidpc-android-bridge
```

> **Note:** Until `libandroidpc_engine.so` is rebuilt with `cargo-ndk`, `NativeBridge.kt` catches `UnsatisfiedLinkError` on the new Phase 4.1 JNI symbols and falls back safely to local artifact inspection so the Android UI never crashes.

---

## 4. Building Self-Authored Compatibility Test Binaries

```bash
sudo apt-get install -y gcc-x86-64-linux-gnu gcc-aarch64-linux-gnu
./scripts/build_box64_test_binaries.sh
```

Outputs `build/box64-tests/compat_probe_arm64` and `build/box64-tests/compat_probe_x86_64`, ready to push into `<guest_rootfs>/opt/androidpc-tests/` for Categories A–D verification and native-vs-Dynarec-vs-Interpreter benchmarking.

---

## 5. License Obligations

- **Box64 (`v0.4.4`)**: MIT License (Copyright ptitSeb & contributors). `scripts/build_box64_arm64.sh` automatically copies upstream `LICENSE` into `app/src/main/assets/box64/LICENSE`, and `AssetInstaller.kt` installs it alongside the binary.
- **Self-authored test probes (`compat_probe.c`)**: Authored inside this repository under the project's license; zero proprietary or un redistributable binaries are bundled.

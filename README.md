# Android PC Environment

> **A serious, rootless Linux desktop computing runtime for Android, engineered in Rust and Kotlin.**

Android PC Environment runs Ubuntu and Debian ARM64 userspaces and interactive Linux shells inside an ordinary Android app. The Rust engine uses PRoot to run rootless guests. The repository also contains work on Linux desktop sessions, Wine/Box64 and QEMU. See [development status](DEVELOPMENT_STATUS.md) for the scope of device verification.

---

## 🏛 System Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Android Layer (Kotlin)                   │
│   • Jetpack Compose Modern Dashboard                        │
│   • Subsystem Health & Diagnostics Monitoring               │
│   • App Lifecycle & Foreground Execution Services           │
│   • Custom Integrated Display Viewport                      │
└──────────────────────────────┬──────────────────────────────┘
                               │ JNI (Java Native Interface)
                               │ [Zero panics / Full catch_unwind]
┌──────────────────────────────▼──────────────────────────────┐
│                    Rust Core Engine                         │
│   • androidpc-core: Runtime Orchestration & Lifecycle       │
│   • androidpc-runtime-api: Distribution & Process Traits    │
│   • androidpc-configuration: Strongly-typed JSON Schemas    │
│   • androidpc-logging: In-Memory Ring Buffer & Tracing      │
│   • androidpc-android-bridge: Safe FFI Boundary             │
└──────────────────────────────┬──────────────────────────────┘
                               │ Rootless Sandboxed Execution
┌──────────────────────────────▼──────────────────────────────┐
│                   Linux Guest Userspace                     │
│   • PRoot Rootless PTrace Virtualization                     │
│   • Ubuntu 26.04 LTS / Debian 13 (ARM64 / aarch64)           │
│   • Linux PTY, with optional desktop integration code       │
└─────────────────────────────────────────────────────────────┘
```

---

## 🚀 Key Features (Phase 0 Foundation)

- **Pure Rust Engine Core:** Memory safety, strict error taxonomy, and deterministic performance without JVM garbage collection pauses.
- **Rootless Execution:** Operates entirely in Android application sandbox storage (`/data/data/com.androidpc.environment/files`) without requiring device rooting or bootloader unlocking.
- **Panic-Safe JNI Bridge:** All entry points guarded with `std::panic::catch_unwind` to prevent fatal process aborts across the native ABI boundary.
- **Hardware-Verified ARM64 Implementation:** Tested and validated on physical Android 16 / Linux 6.1 GKI hardware (`aarch64`).
- **Comprehensive Trait Abstraction:** Clean separation between runtime providers (`PRoot`), distributions (`Ubuntu`, `Debian`), supervisors, display servers (`Xvnc`, `Termux:X11`), and artifact verifiers.

---

## 📦 Monorepo Structure

```
android-pc-environment/
├── android/                   # Android App (Kotlin, Compose, Gradle)
│   ├── app/
│   │   ├── src/main/java/     # UI, ViewModels, and JNI Bridge
│   │   ├── src/main/jniLibs/  # Built Rust native libraries (.so)
│   │   └── build.gradle.kts
│   └── settings.gradle.kts
├── rust/                      # Rust Systems Engine Monorepo
│   ├── Cargo.toml             # Cargo Workspace
│   └── crates/
│       ├── core/              # Engine lifecycle, state machine, health check
│       ├── runtime-api/       # Domain models, traits, error taxonomy
│       ├── configuration/     # Engine & environment configuration
│       ├── logging/           # Structured logging & memory ring buffer
│       └── android-bridge/    # JNI C-ABI exports & device test runner
├── runtime/                   # Runtime artifact definitions & rootfs scripts
│   ├── proot/
│   ├── ubuntu/
│   ├── debian/
│   ├── xfce/
│   └── xvnc/
├── distribution-builder/      # Tooling to package prebuilt rootfs artifacts
├── native/                    # Additional native build glue
├── protocols/                 # Display & IPC protocol definitions
├── docs/                      # Architectural & developer specifications
└── .github/workflows/         # CI pipelines
```

---

## 🛠 Building & Testing

### 1. Host Rust Compilation & Unit Tests
```bash
cd rust
cargo test --workspace --verbose
```

### 2. Cross-Compiling for Android ARM64
```bash
cd rust
cargo ndk --target aarch64-linux-android --platform 26 -- build -p androidpc-android-bridge --release
```

### 3. Running Hardware Validation on Connected Device
```bash
cargo ndk --target aarch64-linux-android --platform 26 -- build -p androidpc-android-bridge --bin device_test_runner --release
adb push target/aarch64-linux-android/release/device_test_runner /data/local/tmp/
adb shell chmod 755 /data/local/tmp/device_test_runner
adb shell /data/local/tmp/device_test_runner
```

---

## 📄 License
Licensed under Apache License, Version 2.0 or MIT License. See [LICENSE](LICENSE) for details.

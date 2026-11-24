# Development Guide

## Prerequisites & Tooling
To develop and build Android PC Environment:

1. **Rust Toolchain**:
   - Rust 1.80+ (Tested with Rust 1.98.1)
   - Host target: `x86_64-pc-windows-gnu` or `x86_64-unknown-linux-gnu`
   - Android target: `aarch64-linux-android`
   ```bash
   rustup target add aarch64-linux-android
   ```

2. **cargo-ndk**:
   ```bash
   cargo install cargo-ndk
   ```

3. **Android NDK**:
   - Android NDK r25b or r26b+ (Tested with r26b)
   - Set environment variable: `ANDROID_NDK_HOME=/path/to/ndk`

4. **Android SDK & JDK**:
   - Java 17 or Java 21+ (Tested with Java 26)
   - Android SDK Build-Tools & Platforms (API 35+)

---

## Build Commands

### Test Rust Workspace Locally
```bash
cd rust
cargo test --workspace --verbose
```

### Cross-Compile Native Library for Android ARM64
```bash
cd rust
cargo ndk --target aarch64-linux-android --platform 26 -- build -p androidpc-android-bridge --release
```
The output `.so` will be created at:
`rust/target/aarch64-linux-android/release/libandroidpc_engine.so`

### Deploy Library to Android Project
```bash
mkdir -p android/app/src/main/jniLibs/arm64-v8a
cp rust/target/aarch64-linux-android/release/libandroidpc_engine.so android/app/src/main/jniLibs/arm64-v8a/
```

### Build & Run Hardware Test Runner on Real Device
```bash
cd rust
cargo ndk --target aarch64-linux-android --platform 26 -- build -p androidpc-android-bridge --bin device_test_runner --release
adb push target/aarch64-linux-android/release/device_test_runner /data/local/tmp/
adb shell chmod 755 /data/local/tmp/device_test_runner
adb shell /data/local/tmp/device_test_runner
```

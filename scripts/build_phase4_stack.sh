#!/usr/bin/env bash
# Reproducible Phase 4 Host Build & Packaging Script for Android PC Environment.
# Intended to be executed on a Linux x86-64 build host with Rust, cargo-ndk,
# Android NDK, CMake/Ninja, and aarch64-linux-gnu cross-compiler.
#
# Never compiles Box64, Wine, or Mesa on the end-user Android device.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGING_DIR="${ROOT_DIR}/build/phase4-artifacts"
mkdir -p "${STAGING_DIR}"

echo "=== [1/4] Rust Workspace Unit & Integration Tests ==="
(
  cd "${ROOT_DIR}/rust"
  cargo test --workspace
)

echo "=== [2/4] Cross-Compile Box64 v0.4.4 (aarch64-linux-gnu glibc) ==="
if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
  "${ROOT_DIR}/scripts/build_box64_arm64.sh"
else
  echo "SKIP: aarch64-linux-gnu-gcc not installed on this host (install gcc-aarch64-linux-gnu)."
fi

echo "=== [3/4] Build Self-Authored ARM64 & x86-64 Compatibility Probes ==="
if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1 && command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
  "${ROOT_DIR}/scripts/build_box64_test_binaries.sh"
else
  echo "SKIP: cross compilers missing for compat_probe binaries."
fi

echo "=== [4/4] Rebuild Android ARM64 JNI Library (libandroidpc_engine.so) ==="
if command -v cargo-ndk >/dev/null 2>&1 && [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
  (
    cd "${ROOT_DIR}/rust"
    cargo ndk -t arm64-v8a -o "${ROOT_DIR}/app/src/main/jniLibs" build --release -p androidpc-android-bridge
  )
else
  echo "SKIP: cargo-ndk or ANDROID_NDK_HOME not configured on this host."
fi

echo "=== Phase 4 Host Build Pipeline Complete ==="
echo "Next: stage verified Wine 11.0 WoW64, Mesa Turnip 26.3.1, DXVK 2.7.1, and VKD3D-Proton 3.1"
echo "tarballs per docs/testing/PHASE_4_TEST_PLAN.md and install the debug APK on Xiaomi Pad 7."

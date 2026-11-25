#!/usr/bin/env bash
# Phase 4 On-Device Verification & Benchmark Harness (Xiaomi Pad 7 / ARM64 Android).
# To be executed by the independent testing agent with ADB connected to Xiaomi Pad 7.
#
# Records real hardware observations without inventing or interpolating metrics.

set -euo pipefail

OUT_DIR="${1:-./phase4-device-results}"
mkdir -p "${OUT_DIR}"

if ! command -v adb >/dev/null 2>&1; then
  echo "ERROR: adb not found in PATH. Run this script on a workstation connected to Xiaomi Pad 7."
  exit 1
fi

echo "=== Collecting Target Device Hardware & Kernel Info ==="
{
  echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "model=$(adb shell getprop ro.product.model | tr -d '\r')"
  echo "soc=$(adb shell getprop ro.board.platform | tr -d '\r')"
  echo "abi=$(adb shell getprop ro.product.cpu.abi | tr -d '\r')"
  echo "android_release=$(adb shell getprop ro.build.version.release | tr -d '\r')"
  echo "kernel=$(adb shell uname -a | tr -d '\r')"
  echo "page_size=$(adb shell getconf PAGESIZE | tr -d '\r')"
  echo "kgsl_node=$(adb shell ls -l /dev/kgsl-3d0 2>&1 | tr -d '\r')"
  echo "gpu_model=$(adb shell cat /sys/class/kgsl/kgsl-3d0/gpu_model 2>&1 | tr -d '\r')"
} | tee "${OUT_DIR}/device_profile.txt"

echo "=== Checking Installed App & Native Library ==="
adb shell dumpsys package com.androidpc.environment | grep -E "versionName|versionCode|primaryCpuAbi" \
  | tee "${OUT_DIR}/app_package_info.txt" || true

echo "=== Capturing Thermal & Memory Baseline ==="
adb shell dumpsys battery | grep -E "temperature|level" | tee "${OUT_DIR}/thermal_baseline.txt" || true
adb shell dumpsys meminfo com.androidpc.environment | head -n 35 | tee "${OUT_DIR}/meminfo_baseline.txt" || true

echo "=== Done collecting baseline device profile ==="
echo "Follow docs/testing/PHASE_4_TEST_PLAN.md Sections A through L to execute interactive & guest tests."

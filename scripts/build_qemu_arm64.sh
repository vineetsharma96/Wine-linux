#!/usr/bin/env bash
set -euo pipefail

# Reproducible build recipe for QEMU 11.1.3 + libslirp 4.9.7 + EDK2 edk2-stable202608
# Target host: Android ARM64 (aarch64-linux-android, rootless userspace)
# Targets built: aarch64-softmmu, x86_64-softmmu, qemu-img
# Note: Run on an ARM64 Linux / Android NDK build host with meson, ninja, glib2, pixman, zlib, zstd, libslirp.

QEMU_VERSION="${QEMU_VERSION:-11.1.3}"
LIBSLIRP_VERSION="${LIBSLIRP_VERSION:-4.9.7}"
EDK2_TAG="${EDK2_TAG:-edk2-stable202608}"
BUILD_ROOT="${BUILD_ROOT:-$(pwd)/build/qemu-arm64}"
STAGE_DIR="${STAGE_DIR:-$(pwd)/app/src/main/assets/runtime/qemu}"
FIRMWARE_STAGE="${FIRMWARE_STAGE:-$(pwd)/app/src/main/assets/runtime/qemu-firmware}"

mkdir -p "${BUILD_ROOT}" "${STAGE_DIR}" "${FIRMWARE_STAGE}"

echo "[Phase 5] Downloading official QEMU ${QEMU_VERSION} source tarball..."
curl -fL "https://download.qemu.org/qemu-${QEMU_VERSION}.tar.xz" -o "${BUILD_ROOT}/qemu-${QEMU_VERSION}.tar.xz"
tar -xf "${BUILD_ROOT}/qemu-${QEMU_VERSION}.tar.xz" -C "${BUILD_ROOT}"

echo "[Phase 5] Downloading libslirp v${LIBSLIRP_VERSION}..."
curl -fL "https://gitlab.freedesktop.org/slirp/libslirp/-/archive/v${LIBSLIRP_VERSION}/libslirp-v${LIBSLIRP_VERSION}.tar.gz" \
  -o "${BUILD_ROOT}/libslirp-${LIBSLIRP_VERSION}.tar.gz"
tar -xzf "${BUILD_ROOT}/libslirp-${LIBSLIRP_VERSION}.tar.gz" -C "${BUILD_ROOT}"

echo "[Phase 5] Configuring QEMU ${QEMU_VERSION} (aarch64-softmmu + x86_64-softmmu, TCG multi-thread + VNC + SLIRP)..."
mkdir -p "${BUILD_ROOT}/qemu-build"
cd "${BUILD_ROOT}/qemu-build"
"../qemu-${QEMU_VERSION}/configure" \
  --target-list="aarch64-softmmu,x86_64-softmmu" \
  --enable-tcg \
  --enable-slirp \
  --enable-vnc \
  --enable-virtfs=no \
  --disable-gtk \
  --disable-sdl \
  --disable-spice \
  --disable-xen \
  --disable-curses \
  --disable-docs \
  --audio-drv-list="pa,wav" \
  --prefix="${BUILD_ROOT}/install"

ninja -j"$(nproc)"
ninja install

cp -f "${BUILD_ROOT}/install/bin/qemu-system-aarch64" "${STAGE_DIR}/qemu-system-aarch64"
cp -f "${BUILD_ROOT}/install/bin/qemu-system-x86_64" "${STAGE_DIR}/qemu-system-x86_64"
cp -f "${BUILD_ROOT}/install/bin/qemu-img" "${STAGE_DIR}/qemu-img"

echo "[Phase 5] Staged QEMU ${QEMU_VERSION} binaries into ${STAGE_DIR}"
echo "[Phase 5] Stage EDK2 (${EDK2_TAG}) AAVMF_CODE.fd / OVMF_CODE.fd into ${FIRMWARE_STAGE} (verify SHA-256)."

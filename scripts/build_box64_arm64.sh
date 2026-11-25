#!/usr/bin/env bash
# Reproducible Box64 ARM64 Linux-Guest Build & Packaging Pipeline
#
# IMPORTANT ABI SEPARATION:
#   Box64 runs *inside* the PRoot Ubuntu/Debian ARM64 Linux guest (glibc),
#   NOT as an Android Bionic binary. Therefore this script cross-compiles
#   Box64 with `aarch64-linux-gnu-gcc` targeting ARM64 Linux glibc, keeping
#   guest Linux libraries strictly isolated from Android Bionic libraries.
#
# Upstream: https://github.com/ptitSeb/box64
# License:  MIT
# Version:  v0.4.4

set -euo pipefail

BOX64_VERSION="${BOX64_VERSION:-v0.4.4}"
BOX64_REPO="${BOX64_REPO:-https://github.com/ptitSeb/box64.git}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORK_DIR="${REPO_ROOT}/.build/box64-${BOX64_VERSION}"
OUT_DIR="${REPO_ROOT}/app/src/main/assets/box64"

echo "==> Building Box64 ${BOX64_VERSION} for ARM64 Linux guest (aarch64-linux-gnu)"

for tool in git cmake ninja aarch64-linux-gnu-gcc python3 sha256sum; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "ERROR: Missing required host build dependency: ${tool}" >&2
        echo "Install on Ubuntu/Debian: sudo apt-get install -y git cmake ninja-build gcc-aarch64-linux-gnu python3" >&2
        exit 1
    fi
done

mkdir -p "${WORK_DIR}" "${OUT_DIR}"

if [ ! -d "${WORK_DIR}/src/.git" ]; then
    git clone --depth 1 --branch "${BOX64_VERSION}" "${BOX64_REPO}" "${WORK_DIR}/src"
else
    git -C "${WORK_DIR}/src" fetch --depth 1 origin tag "${BOX64_VERSION}"
    git -C "${WORK_DIR}/src" checkout -f "${BOX64_VERSION}"
fi

SOURCE_COMMIT="$(git -C "${WORK_DIR}/src" rev-parse HEAD)"

cmake -S "${WORK_DIR}/src" -B "${WORK_DIR}/build" -G Ninja \
    -DCMAKE_SYSTEM_NAME=Linux \
    -DCMAKE_SYSTEM_PROCESSOR=aarch64 \
    -DCMAKE_C_COMPILER=aarch64-linux-gnu-gcc \
    -DCMAKE_BUILD_TYPE=RelWithDebInfo \
    -DARM64=1 \
    -DARM_DYNAREC=ON \
    -DBAD_SIGNAL=ON \
    -DNOGIT=0 \
    -DSAVE_MEM=0 \
    -DCMAKE_C_FLAGS="-O2 -pipe"

cmake --build "${WORK_DIR}/build" --parallel

# Verify output binary is strictly aarch64 Linux ELF (non-Bionic)
if command -v readelf >/dev/null 2>&1; then
    INTERP="$(readelf -l "${WORK_DIR}/build/box64" | grep 'Requesting program interpreter' || true)"
    echo "==> ELF Interpreter: ${INTERP}"
    if echo "${INTERP}" | grep -qi "linker64"; then
        echo "ERROR: Box64 binary was accidentally linked against Android Bionic!" >&2
        exit 1
    fi
fi

install -m 0755 "${WORK_DIR}/build/box64" "${OUT_DIR}/box64"
install -m 0644 "${WORK_DIR}/src/LICENSE" "${OUT_DIR}/LICENSE"

BOX64_SHA256="$(sha256sum "${OUT_DIR}/box64" | awk '{print $1}')"
BOX64_SIZE="$(stat -c%s "${OUT_DIR}/box64" 2>/dev/null || wc -c < "${OUT_DIR}/box64")"

cat > "${OUT_DIR}/manifest.json" <<EOF
{
  "component": "box64",
  "version": "${BOX64_VERSION}",
  "source_repo": "${BOX64_REPO}",
  "source_commit": "${SOURCE_COMMIT}",
  "license": "MIT",
  "target_abi": "aarch64-linux-gnu",
  "cmake_flags": [
    "-DARM64=1",
    "-DARM_DYNAREC=ON",
    "-DBAD_SIGNAL=ON",
    "-DCMAKE_BUILD_TYPE=RelWithDebInfo"
  ],
  "binary": "box64",
  "sha256": "${BOX64_SHA256}",
  "size_bytes": ${BOX64_SIZE}
}
EOF

echo "==> Packaged Box64 ${BOX64_VERSION} (${BOX64_SIZE} bytes, sha256=${BOX64_SHA256})"
echo "==> Output staged in ${OUT_DIR}"

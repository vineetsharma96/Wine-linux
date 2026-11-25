#!/usr/bin/env bash
# Compiles legally clean, self-authored compatibility test binaries for both:
#   - ARM64 Linux guest (aarch64-linux-gnu) — Category A native execution baseline
#   - x86-64 Linux guest (x86_64-linux-gnu) — Category B Box64 translation probes & Category D failures
#
# Run on a Linux/WSL2 dev workstation, then push output dir into guest `/opt/androidpc-tests`
# on the target Xiaomi Pad 7 device for live benchmark and acceptance verification.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${REPO_ROOT}/.build/compat-test-binaries"
SRC_DIR="${OUT_DIR}/src"

mkdir -p "${SRC_DIR}" "${OUT_DIR}/aarch64" "${OUT_DIR}/x86_64"

cat > "${SRC_DIR}/compat_probe.c" <<'EOF'
#include <errno.h>
#include <math.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static volatile sig_atomic_t got_sigusr1 = 0;

static void on_sigusr1(int sig) {
    (void)sig;
    got_sigusr1 = 1;
}

typedef struct {
    int id;
    uint64_t iterations;
    uint64_t checksum;
} worker_arg_t;

static void *worker_fn(void *arg) {
    worker_arg_t *w = (worker_arg_t *)arg;
    uint64_t acc = (uint64_t)(w->id + 1) * 2654435761ULL;
    for (uint64_t i = 0; i < w->iterations; i++) {
        acc = (acc * 6364136223846793005ULL + 1442695040888963407ULL) ^ (i + w->id);
    }
    w->checksum = acc;
    return NULL;
}

int main(int argc, char **argv) {
    const char *mode = (argc > 1) ? argv[1] : "all";

    if (strcmp(mode, "crash_segv") == 0) {
        volatile int *p = (volatile int *)0;
        return *p;
    }
    if (strcmp(mode, "exit_42") == 0) {
        return 42;
    }

    // 1. Arithmetic + floating point
    double fp = 0.0;
    for (int i = 1; i <= 100000; i++) {
        fp += sin((double)i * 0.001) * cos((double)i * 0.001);
    }

    // 2. File I/O
    const char *tmp_path = "/tmp/androidpc_box64_probe.txt";
    FILE *f = fopen(tmp_path, "wb");
    if (!f) {
        fprintf(stderr, "file_open_failed: %s\n", strerror(errno));
        return 2;
    }
    const char *payload = "androidpc-box64-file-io-ok\n";
    fwrite(payload, 1, strlen(payload), f);
    fclose(f);

    char readback[64] = {0};
    f = fopen(tmp_path, "rb");
    if (!f || !fgets(readback, sizeof(readback), f)) {
        fprintf(stderr, "file_read_failed\n");
        return 3;
    }
    fclose(f);
    unlink(tmp_path);
    if (strcmp(readback, payload) != 0) {
        fprintf(stderr, "file_mismatch\n");
        return 4;
    }

    // 3. Heap allocation
    size_t alloc_bytes = 8 * 1024 * 1024;
    uint8_t *buf = (uint8_t *)malloc(alloc_bytes);
    if (!buf) {
        fprintf(stderr, "malloc_failed\n");
        return 5;
    }
    memset(buf, 0x5A, alloc_bytes);
    uint64_t mem_sum = 0;
    for (size_t i = 0; i < alloc_bytes; i += 4096) mem_sum += buf[i];
    free(buf);

    // 4. Threads
    pthread_t threads[4];
    worker_arg_t args[4];
    for (int i = 0; i < 4; i++) {
        args[i].id = i;
        args[i].iterations = 250000;
        args[i].checksum = 0;
        if (pthread_create(&threads[i], NULL, worker_fn, &args[i]) != 0) {
            fprintf(stderr, "pthread_create_failed\n");
            return 6;
        }
    }
    uint64_t thread_xor = 0;
    for (int i = 0; i < 4; i++) {
        pthread_join(threads[i], NULL);
        thread_xor ^= args[i].checksum;
    }

    // 5. Signals
    signal(SIGUSR1, on_sigusr1);
    raise(SIGUSR1);
    if (!got_sigusr1) {
        fprintf(stderr, "signal_delivery_failed\n");
        return 7;
    }

    printf("OK fp=%.6f mem=%llu threads=%016llx sig=1\n",
           fp, (unsigned long long)mem_sum, (unsigned long long)thread_xor);
    return 0;
}
EOF

if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    aarch64-linux-gnu-gcc -O2 -Wall -Wextra "${SRC_DIR}/compat_probe.c" -o "${OUT_DIR}/aarch64/compat_probe_arm64" -lm -lpthread
    echo "==> Built ARM64 native probe: ${OUT_DIR}/aarch64/compat_probe_arm64"
else
    echo "WARN: aarch64-linux-gnu-gcc not found; skipping ARM64 binary build"
fi

if command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
    x86_64-linux-gnu-gcc -O2 -Wall -Wextra "${SRC_DIR}/compat_probe.c" -o "${OUT_DIR}/x86_64/compat_probe_x86_64" -lm -lpthread
    echo "==> Built x86-64 probe: ${OUT_DIR}/x86_64/compat_probe_x86_64"
else
    echo "WARN: x86_64-linux-gnu-gcc not found; skipping x86-64 binary build"
fi

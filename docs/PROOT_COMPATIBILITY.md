# PRoot Compatibility Investigation — Android PC Environment

*Verified on physical hardware: 2026-11-25*
*Target Device: Xiaomi Redmi (`uke`, `2410CRP4CI`), Android 16 (API 36), Linux 6.1 GKI, `aarch64`*

---

## 1. Critical Gate Result: PASSED

PRoot (`5.1.107.102`, Termux Android build with `process_vm = yes, seccomp_filter = yes`) executes **without root** inside the standalone `com.androidpc.environment.debug` application sandbox (`uid=10768(u0_a768)`, `context=u:r:runas_app:s0:c0,c259,c512,c768`).

### Key Technical Constraints & Solutions Discovered

1. **SELinux `execute` / W^X Policy (`targetSdk = 35`)**:
   - Android blocks `execve()` on binaries placed in `context.filesDir` (`app_data_file`).
   - **Solution**: Package `libproot.so`, `libproot_loader.so`, `libproot_loader32.so`, and `libandroid-shmem.so` in `jniLibs/arm64-v8a/` with `android.packaging.jniLibs.useLegacyPackaging = true`, so Android's package installer extracts them into `context.applicationInfo.nativeLibraryDir` (`apk_data_file` / executable).
2. **`libtalloc.so.2` Versioned SONAME Loading**:
   - Android's `jniLibs` installer only extracts files matching `lib*.so` (ignoring `libtalloc.so.2`).
   - **Solution**: `AssetInstaller` unpacks `libtalloc.so.2` into `files/artifacts/lib/libtalloc.so.2`, and `ProotEnvironmentManager::build_guest_env()` sets `LD_LIBRARY_PATH` to include **both** `files/artifacts/lib` and `nativeLibraryDir`.
3. **Seccomp Filter Behavior**:
   - Tested both default PRoot seccomp mode (`EXIT_1=0`) and `PROOT_NO_SECCOMP=1` (`EXIT_2=0`) on Linux 6.1 GKI (`aarch64`). Both succeeded cleanly.
4. **Environment Variable Isolation**:
   - Inherited Android parent variables (`DEX2OATBOOTCLASSPATH`, `FIRST_STAGE_SNAPUSERD_PID`, `EXTERNAL_STORAGE`) would leak into the guest unless cleared. `DefaultTerminalSession::spawn_shell()` calls `libc::clearenv()` on Android/Linux before applying the sanitized guest environment.

---

## 2. Verified Hardware Proof-of-Concept Output

Executed inside `run-as com.androidpc.environment.debug` against an extracted ARM64 Alpine Linux minirootfs (`files/test_rootfs`):

```text
=== TEST 1: DEFAULT SECCOMP ===
aarch64
NAME="Alpine Linux"
ID=alpine
uid=0(root) gid=0(root) groups=1004,1007,1011,1015,1028,1078,1079,3001,3002,3003,3006,3009,3011,3012,50768
/
bin dev etc home lib media mnt opt proc root run sbin srv sys tmp usr var
EXIT_1=0

=== TEST 2: PROOT_NO_SECCOMP=1 ===
aarch64
NAME="Alpine Linux"
ID=alpine
EXIT_2=0
```

---

## 3. Gate Checklist

| Check | Status |
|---|---|
| Execute Android-compatible PRoot binary from standalone app | **PASSED** (`libproot.so 5.1.107.102`) |
| Access app-private root filesystem | **PASSED** (`files/test_rootfs`) |
| Execute ARM64 Linux guest binaries (`uname`, `id`, `ls`, `/usr/bin/env`) | **PASSED** (`aarch64`, exit code `0`) |
| Launch `/bin/sh` | **PASSED** |
| Operate without root privileges | **PASSED** (`u0_a768` sandbox) |
| Interactive PTY allocation (`posix_openpt`, `grantpt`, `unlockpt`, `TIOCSWINSZ`) | **PASSED** (`/dev/pts/0` verified) |

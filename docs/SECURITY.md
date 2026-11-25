# Security Architecture & Sandbox Model

## 1. Rootless Execution Model
- Android PC Environment never requests root (`su`) or elevated SELinux capabilities.
- Guest files are installed in app-private storage. PRoot and its executable loaders are in Android's APK native library directory.
- No modifications are made to system partitions (`/system`, `/vendor`, `/apex`).

## 2. Android Process & PTrace Constraints
- Android applies strict seccomp filters and SELinux policies to third-party applications.
- **PTrace Verification**: PRoot ran unprivileged on the tested Android 16 device with a 6.1 GKI kernel. See [device compatibility](PROOT_COMPATIBILITY.md) for the tested scope.
- PRoot hooks `execve`, `mount`, `stat`, and filesystem syscalls using ptrace, translating paths into the unprivileged rootfs directory without kernel-level mount privileges.

## 3. Cryptographic Verification
- Release builds verify the detached Ed25519 signature of the manifest with a public key embedded in the APK. Rust validates fields after signature verification.
- The installer checks size and SHA-256 before extraction. Archive paths are checked against the staging root. Installation commits only after the extracted guest runs the identity and package manager checks.
- Guest processes have a controlled set of environment variables. PRoot path translation supports guest compatibility and runs inside the same Android app sandbox.
- Running guests cannot be deleted, and package operations and snapshots take a per-environment lock. Restore verifies both the archive and the booted guest before publishing a new directory. The backup format is app-private and uses a checksum to detect corruption.

## 4. Clean-Checkout Packaging & Production Signing Inputs
- **Rootfs Catalog Signing**: Run `distribution-builder/build_rootfs_catalog.py` with `distribution-builder/arm64-inputs.json`, verified upstream rootfs archives, and a project-owned Ed25519 private key to output `android/app/src/release/assets/rootfs/catalog.json` and the 64-byte detached `catalog.sig`.
- **Public Key Injection**: Export `ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX` (the 64-character hex Ed25519 public key printed by `build_rootfs_catalog.py`) before building. `android/app/build.gradle.kts` (`verifyReleaseRootfsInputs`, wired to `preReleaseBuild`) enforces both the hex key and staged `catalog.json`/`catalog.sig`, embedding the key in `BuildConfig.ROOTFS_PUBLIC_KEY_HEX`.
- **APK Release Signing**: Current debug builds on device use a test-signed rootfs catalog. Production releases require configuring an Android release keystore (`signingConfig`) for `buildTypes.release` alongside the production `ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX`.
- **Clean-Checkout Build Sequence**:
  1. `cd rust && cargo test --workspace`
  2. `cargo ndk -t arm64-v8a -o ../android/app/src/main/jniLibs build --release`
  3. `cd ../android && gradle assembleDebug` (or `assembleRelease` with release signing inputs set).

## 5. Phase 4 Windows Compatibility Security & Trust Boundaries
- **PRoot Boundary Honesty (CRITICAL)**: PRoot (`ptrace` syscall path translation) is a **compatibility and filesystem-namespace tool, NOT a hardware VM or kernel security sandbox**. All PRoot guest processes—including Box64, Wine `11.0`, and Windows executables—execute inside the same Android application UID (`u0_aXXX`) and share the Android app sandbox boundary. Users must treat untrusted Windows executables accordingly and never assume VM-grade kernel isolation.
- **Isolated Wine Prefixes**: Every Wine prefix resides in `<base_data_dir>/wine_prefixes/<prefix_id>` with strict ID validation (`^[a-zA-Z0-9_-]{1,64}$`) to block directory traversal. Prefixes isolate `drive_c`, registry hives, DLL overrides, and `wineserver` sockets from one another and from the host Ubuntu/Debian rootfs.
- **Backup & Restore Integrity**: Wine prefix backups generate companion `.sha256` digests. Restore verifies the SHA-256 digest before extraction, validates every archive entry against Zip-Slip traversal via `androidpc-security`, and unpacks into a staging directory before atomic promotion.
- **No Unverified Scripts or Proprietary Bundles**: The Dependency Manager (`rust/crates/dependency-manager`) prohibits blind `curl | sh` downloads and does not bundle unlicensed proprietary Microsoft redistributables (`vc_redist`, `.NET`, DirectX). Open-source artifacts (`Box64`, `Wine`, `Mesa Turnip`, `virglrenderer`, `DXVK`, `VKD3D-Proton`, `Wine Mono`) require SHA-256 verification; proprietary Windows runtimes use a user-supplied official installer workflow inside an isolated prefix.
- **Scoped Process Termination & Log Redaction**: Stopping a Windows app runs `wineserver -k`/`-k9` strictly against its isolated `WINEPREFIX` (never global `killall`). Per-app diagnostic log buffers (`rust/crates/windows-apps`) are bounded in line count and redact environment values whose keys match credential patterns (`PASSWORD`, `TOKEN`, `SECRET`, `KEY`, `AUTH`).


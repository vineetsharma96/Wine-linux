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

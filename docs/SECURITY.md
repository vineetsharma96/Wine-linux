# Security Architecture & Sandbox Model

## 1. Rootless Execution Model
- Android PC Environment never requests root (`su`) or elevated SELinux capabilities.
- All guest binaries and execution environments are contained within the app-private data directory (`/data/data/com.androidpc.environment/files`).
- No modifications are made to system partitions (`/system`, `/vendor`, `/apex`).

## 2. Android Process & PTrace Constraints
- Android applies strict seccomp filters and SELinux policies to third-party applications.
- **PTrace Verification**: Android kernel 6.1 GKI permits an unprivileged process to trace its own descendants (`PTRACE_TRACEME` / `PTRACE_ATTACH` on child threads) within its own process group.
- PRoot hooks `execve`, `mount`, `stat`, and filesystem syscalls using ptrace, translating paths into the unprivileged rootfs directory without kernel-level mount privileges.

## 3. Cryptographic Verification
- Guest rootfs archives must never be executed without verification.
- Every artifact provider implements SHA-256 validation:
  ```rust
  fn verify_artifact(&self, path: &Path, expected_sha256: &str) -> Result<bool, RuntimeError>;
  ```
- Any checksum mismatch triggers immediate abort and cleanup.

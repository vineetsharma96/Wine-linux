# Linux environment management

Create Ubuntu or Debian from an available catalog entry and optionally name the environment. A generated UUID identifies each new guest independently of its display name. Multiple guests can share a name or distribution. Existing guests retain their original identifiers.

The dashboard gets installed guests, their allocated storage, lifecycle states and last-started timestamps from Rust. The newest registry records store a timestamp for creation and real launches, the verified guest release, and the discovered packaged PRoot version when available. Fields introduced after Phase 1 are optional when loading old records.

Start and restart use the existing PRoot process supervisor. A terminal uses the same registered root filesystem through a real PTY. Stop closes the guest's terminal sessions and then stops its supervised guest processes. Rust rejects deletion while either type of process is live. A repeated start cannot launch a second startup process while the environment is running.

Open Details to view installed packages and measured resources. The same screen leads to backups and cloning. See [packages](PACKAGE_MANAGEMENT.md), [backups](BACKUP_AND_RESTORE.md), and [development status](../DEVELOPMENT_STATUS.md) for verification and remaining work.

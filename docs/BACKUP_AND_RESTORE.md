# Backups, restoration and cloning

Use **Back up or clone** from a stopped environment's Details screen. A backup is an uncompressed tar and a versioned manifest under private `files/backups/<uuid>/`. The manifest records the source, creation time, archive size and SHA-256. A backup appears in the list only after the archive and manifest are written. **All backups** remains accessible even after the source environment is removed.

Restore checks the manifest, size and checksum before extracting privately. The extracted guest must boot and report the saved distribution release. Restore always takes a fresh UUID and a new directory. Clone uses a temporary backup and the same restore path, so the two roots and their package databases are independent. Interrupted clone operations can leave a complete backup for recovery.

Backup cancellation is checked during archive writing. Restore checks cancellation between integrity verification, extraction, guest boot and registration. Capacity checks are estimates. A filesystem error such as running out of space aborts the operation and cleans staging. A backup has regular file contents, mode bits, timestamps and symlink entries supported by the tar implementation. Device state, running processes, Android ownership, host mount permissions and extended attributes are outside the portable snapshot.

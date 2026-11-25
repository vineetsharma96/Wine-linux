# Environment storage

Android supplies the app files directory and separate cache directory. New Linux guests use `files/environments/<distribution>/<uuid>/` with a `rootfs/` and an atomic `config.json`. Existing Phase 1 directories under `files/environments/<id>/` remain registered in place. The registry validates the recorded ID, distribution, configuration and actual directory before opening a guest.

The installer verifies the archive and checks the real guest in a private sibling staging directory before renaming the installation into the registry. Deleted guests are moved to a private tombstone before removal. Startup cleans abandoned install and delete directories under both layouts. Downloads remain in app-private cache and runtime binaries continue to use Android's native library directory and managed artifacts directory.

Android storage totals use allocated blocks. The scan counts hard-linked file data once and does not follow symlinks into other directories. The Windows host test fallback counts regular file lengths because its filesystem API differs.

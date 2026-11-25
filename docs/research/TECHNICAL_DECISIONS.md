# Technical Decisions (ADRs)

- **ADR-009 — Stage 3.2 Surface presentation**. Select the direct Rust ANativeWindow presenter and a Canvas fallback on RFB. [Decision and evidence](../TECHNICAL_DECISIONS.md#stage-32)

- **ADR-008 — Guest APT migration and scoped desktop shutdown (Stage 3.1)**. [Decision and research links](../TECHNICAL_DECISIONS.md#stage-31)

- **ADR-001 — PRoot in `jniLibs`**: Package `libproot.so` and `libproot_loader*.so` in `jniLibs/arm64-v8a` (`useLegacyPackaging = true`) so Android 16 SELinux permits execution without root.
- **ADR-002 — Scoped UUID Registry**: Store new guests in `files/environments/<distro>/<uuid>/` with atomic `config.json` while retaining Phase 1 `files/environments/<id>/` entries.
- **ADR-003 — Guest-Hosted `Xvnc` + Short `/tmp` Bind**: Run `tigervnc-standalone-server` inside PRoot (instead of standalone Termux `Xvnc` missing `libcrypto.so.3`) with `<base_data_dir>/dtmp:/tmp` to satisfy Linux's 108-byte `sun_path` limit.
- **ADR-004 — `glycin` `bwrap` Shim**: Forward `/usr/libexec/glycin-loaders/*` inside PRoot's existing sandbox on Ubuntu 26.04 where `CLONE_NEWUSER` is unavailable.
- **ADR-005 — `DirectByteBuffer` + 32bpp Fast Path**: Eliminate per-frame JNI `vec![]` allocations and scalar pixel divisions in `RfbClient` -> `SurfaceView`.
- **ADR-006 — XERJ Mandatory Knowledge Layer**: Index `rust/`, `android/`, `docs/`, and `distribution-builder/` with `xerj autoindex` (`androidpc` prefix) and keep XERJ strictly out of the Android APK.
- **ADR-007 — Rust `ANativeWindow` Native Surface Renderer (Stage 3)**: Replace per-frame Kotlin `Bitmap`/`DirectByteBuffer` JNI copies with a dedicated Rust RFB reader/presenter thread writing directly to `ANativeWindow` (`ANativeWindow_setBuffersGeometry` + `ANativeWindow_lock` + `ANativeWindow_unlockAndPost`), retaining `Termux:X11` as an optional external backend and `Xvnc` fallback.

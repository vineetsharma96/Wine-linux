# Research Knowledge Index

This directory is the permanent reverse-engineering and technical research knowledge base for Android PC Environment, indexed by XERJ (`androidpc-docs`).

## Documents
- [DISPLAY_RENDERER_EVALUATION.md](DISPLAY_RENDERER_EVALUATION.md) — Stage 3.2 candidate comparison and selection.
- [ANDROID_NATIVE_RENDERING.md](ANDROID_NATIVE_RENDERING.md) — ANativeWindow, compatibility presenter, and SurfaceView lifecycle.
- [DISPLAY_BENCHMARKS.md](DISPLAY_BENCHMARKS.md) — device evidence and workload protocol.
- [REVERSE_ENGINEERING_POLICY.md](REVERSE_ENGINEERING_POLICY.md) — Lawful public-source methodology, evidence hierarchy, and security boundaries.
- [REFERENCE_PROJECTS.md](REFERENCE_PROJECTS.md) — Catalog of Linux-on-Android, Windows-on-Android, and graphics reference projects, licenses, and adoption decisions.
- [ANDROID_LINUX_RESEARCH.md](ANDROID_LINUX_RESEARCH.md) — PRoot, SELinux, `AF_UNIX` `sun_path`, `glycin`/`bwrap`, and Termux/Termux:X11 Android integration findings.
- [GRAPHICS_RESEARCH.md](GRAPHICS_RESEARCH.md) — `Xvnc` vs `libXlorie` (`SurfaceView`/`BLAST`), `libandroid-shmem`, Turnip, VirGL, and Zink.
- [WINDOWS_COMPATIBILITY_RESEARCH.md](WINDOWS_COMPATIBILITY_RESEARCH.md) — Winlator, Wine, Box64, DXVK, and VKD3D-Proton public architecture notes.
- [BOX64_RESEARCH.md](BOX64_RESEARCH.md) — Phase 4.1 Box64 `v0.4.4` (MIT) technical investigation (ARM64 Dynarec, Interpreter, ELF loader, wrapped libs, PRoot, glibc isolation).
- [ARM64_X86_COMPATIBILITY.md](ARM64_X86_COMPATIBILITY.md) — Phase 4.1 Rust compatibility layer (`androidpc-box64`), ELF router, library resolver, and Categories A–D test suite design.
- [BOX64_ANDROID_BUILD.md](BOX64_ANDROID_BUILD.md) — Reproducible `aarch64-linux-gnu` glibc cross-compilation pipeline, packaging, and ABI separation rules.
- [BOX64_PERFORMANCE.md](BOX64_PERFORMANCE.md) — Native ARM64 vs. Box64 Dynarec vs. Interpreter benchmark methodology and Xiaomi Pad 7 results table (pending on-device run).
- [PERFORMANCE_FINDINGS.md](PERFORMANCE_FINDINGS.md) — Xiaomi Pad 7 (`2410CRP4CI`) 11 FPS root-cause analysis and Stage 2 benchmarks.
- [TECHNICAL_DECISIONS.md](TECHNICAL_DECISIONS.md) — Engineering Decision Records (ADRs) for major architectural choices.


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
- [WINE_RESEARCH.md](WINE_RESEARCH.md) — Phase 4.2 Wine `11.0` (LGPL-2.1-or-later) x86-64 pure-64-bit WoW64 strategy, PE32/PE32+/MSI inspection, and prefix isolation.
- [TURNIP_RESEARCH.md](TURNIP_RESEARCH.md) — Phase 4.3 Mesa Turnip `26.3.1` (MIT) Vulkan on Xiaomi Pad 7 (Snapdragon 7+ Gen 3 / Adreno 732, `/dev/kgsl-3d0`) and 6-stage readiness model.
- [VIRGL_RESEARCH.md](VIRGL_RESEARCH.md) — Phase 4.3 `virglrenderer` `1.3.0` (MIT) OpenGL-only IPC (`virpipe`/`vtest`) architecture and Vulkan/DXVK/VKD3D exclusion boundaries.
- [DXVK_RESEARCH.md](DXVK_RESEARCH.md) — Phase 4.4 DXVK `v2.7.1` (zlib) Direct3D 8/9/10/11 → Vulkan 1.3+ capability gating and WineD3D fallback.
- [VKD3D_RESEARCH.md](VKD3D_RESEARCH.md) — Phase 4.4 VKD3D-Proton `v3.1` (LGPL-2.1) Direct3D 12 → Vulkan 1.3+ strict 6-extension gate.
- [WINDOWS_COMPATIBILITY_ARCHITECTURE.md](WINDOWS_COMPATIBILITY_ARCHITECTURE.md) — End-to-end Phase 4 architecture connecting PRoot, Box64, Wine, Graphics Backends, Dependencies, and Android UI.
- [QEMU_RESEARCH.md](QEMU_RESEARCH.md) — Phase 5 QEMU `11.1.3` (GPL-2.0-only), `libslirp 4.9.7` (BSD-3-Clause), and EDK2 `edk2-stable202608` (AAVMF/OVMF) architecture, machine models, TCG, QMP, storage, and display/audio analysis.
- [ANDROID_VIRTUALIZATION_RESEARCH.md](ANDROID_VIRTUALIZATION_RESEARCH.md) — Phase 5 investigation of Android Virtualization Framework (AVF / pKVM `@SystemApi` restriction), `/dev/kvm` SELinux access on stock ARM64 devices, and rootless execution constraints.
- [PERFORMANCE_FINDINGS.md](PERFORMANCE_FINDINGS.md) — Xiaomi Pad 7 (`2410CRP4CI`) 11 FPS root-cause analysis and Stage 2 benchmarks.
- [TECHNICAL_DECISIONS.md](TECHNICAL_DECISIONS.md) — Engineering Decision Records (ADRs) for major architectural choices.


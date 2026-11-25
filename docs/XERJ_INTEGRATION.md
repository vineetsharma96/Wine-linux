# XERJ Integration & Agent Code Intelligence

## Overview
XERJ (`xerj.exe`, local node `http://127.0.0.1:9200`, data dir `C:\Users\abc\xerj-data`, embeddings `all-MiniLM-L6-v2`) is the permanent code-intelligence, semantic search, and project-memory layer for Android PC Environment. XERJ runs strictly in the development environment and is **not** embedded in the Android APK.

## Configuration
- **MCP Config**: [.mcp.json](file:///c:/GitHub%20Projects/Working/LINUX+WINE/.mcp.json) registers `C:\Users\abc\AppData\Local\Programs\xerj\xerj.exe mcp` with `XERJ_URL=http://127.0.0.1:9200`.
- **Ignore Rules**: [.xerjignore](file:///c:/GitHub%20Projects/Working/LINUX+WINE/.xerjignore) excludes `target/`, `rust/target/`, `android/build/`, `android/app/build/`, `.gradle/`, `.cxx/`, `runtime/proot/`, `runtime/xvnc/`, `assets/`, `android/app/src/main/jniLibs/`, `android/app/src/main/assets/`, `android/app/src/release/assets/`, `local.properties`, and all keystores/keys (`*.keystore`, `*.jks`, `*.pem`, `*.key`, `*.sig`, `*.p12`).
- **Index Prefix**: `androidpc` (`androidpc-code`, `androidpc-docs`, `.xerj-memory-androidpc`, `.xerj-memory-linux-wine-edges`).

## Standard Commands
```powershell
$XERJ = "C:\Users\abc\AppData\Local\Programs\xerj\xerj.exe"
& $XERJ autoindex . --prefix androidpc --code-analyzer code --workers 2
& $XERJ def "start_desktop_session" --prefix androidpc
& $XERJ search "DirectByteBuffer framebuffer" --prefix androidpc
```

## Phase 4.1 Refresh (Pending Local Windows XERJ Run)
Phase 4.1 added `rust/crates/box64/`, `scripts/build_box64_arm64.sh`, `scripts/build_box64_test_binaries.sh`, `app/src/main/java/com/androidpc/environment/ui/screens/CompatibilityScreen.kt`, and `docs/research/BOX64_*.md` / `ARM64_X86_COMPATIBILITY.md` (and migrated the Android Gradle project to root `app/`). Because the cloud build environment cannot reach the local Windows XERJ node (`http://127.0.0.1:9200`), run the following refresh on the local development workstation after pulling changes:

```powershell
$XERJ = "C:\Users\abc\AppData\Local\Programs\xerj\xerj.exe"
& $XERJ autoindex . --prefix androidpc --code-analyzer code --workers 2
& $XERJ def "Box64CompatibilityLayer" --prefix androidpc
& $XERJ search "inspect_elf_bytes android bionic" --prefix androidpc
```

## Phase 4 (4.2–4.10) Refresh (Pending Local Windows XERJ Run)
Phase 4 added `rust/crates/graphics-backend/`, `rust/crates/dependency-manager/`, `rust/crates/windows-apps/`, extended `rust/crates/wine/`, `rust/crates/core/src/windows_compat.rs`, `rust/crates/android-bridge/src/windows_jni.rs`, `runtime/*/manifest.json`, `scripts/build_phase4_stack.sh`, `scripts/benchmark_phase4_device.sh`, Android Compose Windows Compatibility screens (`WindowsCompatibilityScreen.kt`, `WindowsAppsTab.kt`, `WineAndGraphicsTabs.kt`, `WindowsCompatibilityBridge.kt`), Phase 4 research & operational docs, and `docs/testing/PHASE_4_TEST_PLAN.md`. On the local Windows workstation with XERJ running, refresh the index and store Phase 4 architectural memory:

```powershell
$XERJ = "C:\Users\abc\AppData\Local\Programs\xerj\xerj.exe"
& $XERJ autoindex . --prefix androidpc --code-analyzer code --workers 2
& $XERJ def "GraphicsManager" --prefix androidpc
& $XERJ def "WinePrefixManager" --prefix androidpc
& $XERJ search "VKD3D_REQUIRED_VULKAN_EXTENSIONS" --prefix androidpc
```

## Phase 5 Refresh (Pending Local Windows XERJ Run)
Phase 5 refactored `rust/crates/vm/` into 10 modular Rust files (`config.rs`, `storage.rs`, `network.rs`, `display_audio.rs`, `monitor.rs`, `snapshot.rs`, `images.rs`, `diagnostics.rs`, `qemu_engine.rs`, `error.rs`), added `rust/crates/core/src/vm_engine.rs`, `rust/crates/android-bridge/src/vm_jni.rs`, `runtime/qemu/manifest.json`, `scripts/build_qemu_arm64.sh`, `VmEngineBridge.kt`, `VirtualMachinesScreen.kt`, `VmDialogsAndTabs.kt`, Phase 5 research & operational docs, and `docs/testing/PHASE_5_TEST_PLAN.md`. On the local Windows workstation with XERJ running, refresh the index and verify symbol lookup:

```powershell
$XERJ = "C:\Users\abc\AppData\Local\Programs\xerj\xerj.exe"
& $XERJ autoindex . --prefix androidpc --code-analyzer code --workers 2
& $XERJ def "QemuSupervisor" --prefix androidpc
& $XERJ def "VmStorageManager" --prefix androidpc
& $XERJ search "write_minimal_qcow2_v3" --prefix androidpc
```




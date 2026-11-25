# DXVK Research — Direct3D 8/9/10/11 to Vulkan (Phase 4.4)

*Last updated: 2026-11-25*
*Pinned component: DXVK `v2.7.1` (zlib License)*

## 1. Overview & Official Provenance

- **Official Repository**: `https://github.com/doitsujin/dxvk`
- **Pinned Version**: **`v2.7.1`**
- **License**: `zlib` (permissive open-source license, redistributable with license attribution)
- **Scope**: Translates Windows Direct3D 8 (`d3d8.dll`), Direct3D 9 (`d3d9.dll`), Direct3D 10 (`d3d10core.dll`), Direct3D 11 (`d3d11.dll`), and DXGI (`dxgi.dll`) API calls into **Vulkan 1.3+**.

## 2. Hard Requirements & Gating Policy

DXVK **cannot** run on OpenGL-only backends (`VirGL`, `llvmpipe`). Enabling DXVK DLL overrides without a live Vulkan 1.3+ driver causes immediate Direct3D device creation crashes (`E_FAIL` / `DXGI_ERROR_UNSUPPORTED`).

Therefore, `DxTranslationManager` (`rust/crates/graphics-backend/src/dx_translation.rs`) enforces strict gating:

1. **Staged Artifact Check**: `x64/` and `x32/` DLL directories (`d3d8.dll`, `d3d9.dll`, `d3d10core.dll`, `d3d11.dll`, `dxgi.dll`) must be present and verified by SHA-256.
2. **Backend Eligibility**: The active `GraphicsBackend` must report:
   - `supports_vulkan == true`
   - `dxvk_compatible == true`
   - `stage >= BackendReadinessStage::Operational` (live Vulkan 1.3+ probe passed, e.g., Mesa Turnip `26.3.1` on Adreno 732).
3. **Automatic WineD3D Fallback**: If any gate fails (Turnip unavailable, VirGL selected, Software selected, or DXVK DLLs not staged), `DxTranslationManager` automatically selects `DxTranslationChoice::WineD3dOpenGl` and omits `d3d9/d3d10core/d3d11/dxgi=n,b` overrides so the application falls back to Wine's built-in OpenGL WineD3D renderer rather than crashing.

## 3. Wine Prefix Staging & Environment Variables

When DXVK is eligible and enabled for a prefix:
- **64-bit DLLs** (`x64/*.dll`) are staged into `<prefix>/drive_c/windows/system32/`.
- **32-bit DLLs** (`x32/*.dll`) are staged into `<prefix>/drive_c/windows/syswow64/` (for Wine 11.0 WoW64 PE32 apps).
- **Wine DLL Override**: `d3d8=n,b;d3d9=n,b;d3d10core=n,b;d3d11=n,b;dxgi=n,b`.
- **State Cache**: `DXVK_STATE_CACHE_PATH` is scoped to the isolated prefix/cache directory (`shader_cache_dir`) so shader compilation stalls diminish across launches without leaking across prefixes.
- **Diagnostics HUD**: Optional `DXVK_HUD=fps,devinfo,api` toggled via Graphics Settings (`enable_hud`).

## 4. Known Constraints & Pending Hardware Verification (`NOT RUN`)

- **Shader Compilation Stutter**: On mobile ARM64 CPUs translating x86-64 via Box64, initial DXVK SPIR-V → Adreno `ir3` pipeline compilation can cause multi-second stalls until `VK_EXT_graphics_pipeline_library` (GPL) support is confirmed on Turnip `26.3.1` + Adreno 732.
- **Memory Pressure**: DXVK maintains host-visible staging buffers and shader caches; behavior on 8–12 GB RAM tablets under Android's `lmkd` (Low Memory Killer Daemon) requires physical Xiaomi Pad 7 testing (`NOT RUN`).
- **DXVK 2.x Vulkan 1.3 Baseline**: DXVK `v2.7.1` requires Vulkan 1.3 core features; devices or drivers limited to Vulkan 1.1/1.2 must remain on `WineD3D`.

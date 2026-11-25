# VKD3D-Proton Research — Direct3D 12 to Vulkan (Phase 4.4)

*Last updated: 2026-11-25*
*Pinned component: VKD3D-Proton `v3.1` (LGPL-2.1 License)*

## 1. Overview & Official Provenance

- **Official Repository**: `https://github.com/HansKristian-Work/vkd3d-proton`
- **Pinned Version**: **`v3.1`** (released October 2026)
- **License**: `LGPL-2.1`
- **Scope**: Translates Windows Direct3D 12 (`d3d12.dll`, `d3d12core.dll`) API calls into **Vulkan 1.3+**.

## 2. Strict Capability Gate (No Unverified DX12 Claims)

Direct3D 12 translation has significantly stricter GPU driver requirements than Direct3D 9–11 (DXVK). Mobile Vulkan drivers frequently lack descriptor indexing tiers, robust buffer access, or dynamic rendering semantics required by VKD3D-Proton `v3.1`.

**Android PC Environment NEVER claims DirectX 12 compatibility unless a live runtime Vulkan probe verifies ALL of the following:**

1. Active backend is `Operational` (`stage >= BackendReadinessStage::Operational`), supports Vulkan 1.3+, and reports `vkd3d_compatible == true`.
2. VKD3D-Proton `v3.1` DLLs (`d3d12.dll`, `d3d12core.dll` in `x64/` and `x86/`) are staged and verified.
3. Live Vulkan physical-device extension query confirms **all 6 mandatory extensions** (`VKD3D_REQUIRED_VULKAN_EXTENSIONS` in `rust/crates/graphics-backend/src/dx_translation.rs`):
   - `VK_KHR_dynamic_rendering`
   - `VK_EXT_descriptor_indexing`
   - `VK_KHR_buffer_device_address`
   - `VK_KHR_timeline_semaphore`
   - `VK_EXT_robustness2`
   - `VK_EXT_extended_dynamic_state`

If **any** extension is missing or unverified, `DxTranslationManager::evaluate` explicitly records the missing extensions in `vkd3d_missing_extensions`, sets `vkd3d_eligible = false`, records a clear `vkd3d_fallback_reason`, and refuses to activate `d3d12=n,b` overrides.

## 3. Wine Prefix Staging & Diagnostics

When all capability gates pass and the user enables VKD3D-Proton:
- **64-bit DLLs**: `<prefix>/drive_c/windows/system32/{d3d12.dll, d3d12core.dll}`
- **32-bit DLLs**: `<prefix>/drive_c/windows/syswow64/{d3d12.dll, d3d12core.dll}`
- **Wine DLL Override**: `d3d12=n,b;d3d12core=n,b`
- **Shader Cache**: `VKD3D_SHADER_CACHE_PATH` scoped to `<shader_cache_dir>/vkd3d`.
- **Debug Logging**: `VKD3D_DEBUG=warn` (elevated to `info` only in diagnostic runs).

## 4. Known Risks & Unverified Hardware Items (`NOT RUN`)

- **Adreno 732 (Xiaomi Pad 7) DX12 Execution**: Live VKD3D-Proton `v3.1` execution on Mesa Turnip `26.3.1` + Adreno 732 is **`NOT RUN`**.
- **Shader Model 6.x & Wave Ops**: Some DX12 workloads require subgroup/wave operations and sparse residency features that may fail or render incorrectly on mobile `a7xx` GPUs even when the 6 baseline extensions are advertised.
- **VRAM / Unified Memory Limits**: DX12 applications manage explicit heap allocations; running under PRoot + Box64 + Turnip without dedicated VRAM may trigger Android `lmkd` termination under high memory pressure.
- **No VirGL or Software DX12 Support**: VirGL is OpenGL-only and `llvmpipe` is OpenGL-only; VKD3D-Proton is strictly disallowed on both.

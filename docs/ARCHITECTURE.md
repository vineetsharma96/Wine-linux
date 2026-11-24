# System Architecture — Android PC Environment

## 1. Architectural Philosophy
Android PC Environment decouples high-level Android presentation and lifecycle orchestration from systems logic, process supervision, and rootless runtime execution. 

### Core Tenet
- **The Engine is Rust**: All process supervision, filesystem operations, archive verification, PTY orchestration, display protocol transport, and configuration are implemented in Rust.
- **The UI is Kotlin**: Kotlin + Jetpack Compose is strictly responsible for presentation, device sensors, window insets, permissions, notifications, and foreground services.
- **Zero Business Logic Duplication**: Kotlin does not re-implement status checks, configuration validation, or path resolution; it delegates directly to Rust via JNI.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        ANDROID APPLICATION LAYER                       │
│  [MainActivity] -> [Compose UI: Dashboard, Health, Diagnostics, Env]   │
│                                   │                                    │
│                 [NativeBridge (Kotlin Object)]                         │
│                                   │ Dispatchers.IO                     │
└───────────────────────────────────┼────────────────────────────────────┘
                                    │ JNI FFI (Catch Unwind Guarded)
┌───────────────────────────────────▼────────────────────────────────────┐
│                           RUST NATIVE ENGINE                           │
│                                                                        │
│   ┌─────────────────────┐  ┌─────────────────────┐  ┌──────────────┐   │
│   │   androidpc-core    │  │androidpc-runtime-api│  │ configuration│   │
│   │ (EngineCore Instance)  │ (Traits & Contracts)│  │ (Serde JSON) │   │
│   └──────────┬──────────┘  └──────────┬──────────┘  └──────────────┘   │
│              │                        │                                │
│              ▼                        ▼                                │
│   ┌─────────────────────┐  ┌─────────────────────┐                     │
│   │  androidpc-logging  │  │   android-bridge    │                     │
│   │ (Tracing + RingBuf) │  │  (C-ABI Native Exp) │                     │
│   └─────────────────────┘  └─────────────────────┘                     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ POSIX PTrace / fork / exec
┌───────────────────────────────────▼────────────────────────────────────┐
│                        ROOTLESS LINUX RUNTIME                          │
│   • PRoot Rootless PTrace Isolation Sandbox                            │
│   • Ubuntu 24.04 LTS / Debian 12 Userspace                             │
│   • Xvnc / TigerVNC Server + XFCE 4 Desktop Environment                │
│   • Internal Storage (/data/data/com.androidpc.environment/files)      │
└────────────────────────────────────────────────────────────────────────┘
```

## 2. Monorepo Crate Boundaries
1. **`androidpc-runtime-api`**:
   - Holds traits (`RuntimeProvider`, `LinuxDistribution`, `EnvironmentManager`, `ProcessSupervisor`, `TerminalSession`, `DisplayBackend`, `ArtifactProvider`).
   - Contains zero concrete engine implementations to maintain compile-time isolation and mockability.
2. **`androidpc-configuration`**:
   - Manages serializable structures: `EngineConfig`, `EnvironmentConfig`, `DisplayConfig`.
   - Validates memory limits, display geometries, and filesystem paths.
3. **`androidpc-logging`**:
   - Tracing subscriber infrastructure and circular in-memory log buffer (`LogBuffer`) for real-time JNI diagnostics.
4. **`androidpc-core`**:
   - The heart of the system. Manages `EngineCore`, state machines, distribution catalogs, and health-check execution.
5. **`androidpc-android-bridge`**:
   - C-ABI JNI exports and safety wrappers (`catch_unwind`).
   - Exposes versioning, initialization, health check, diagnostics, and ring-buffer log streaming.

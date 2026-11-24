# Rust Engine Specification

## 1. Engine Core (`androidpc-core`)
The `EngineCore` struct acts as the singleton orchestrator for the native environment.

### Lifecycle States
```
Uninitialized ──► Initializing ──► Ready ──► Running ──► Terminated
                                    │
                                    ▼
                                 Degraded
```

- **Uninitialized**: Binary loaded, no memory or paths allocated.
- **Ready**: Configuration validated, logging initialized, default distributions registered.
- **Running**: Active rootless guest environment running under process supervision.
- **Degraded**: One or more non-critical subsystems (e.g. logging buffer full, storage warning) reported an issue during health check.

## 2. Health-Check Subsystem
The engine executes multi-stage health checks:
1. **Storage Subsystem**: Validates write permissions and path existence in app-private directories.
2. **CPU & Architecture**: Identifies host instruction set and validates 64-bit alignment (`aarch64`).
3. **Process Engine**: Tests process creation, signal handling harness, and IPC pipes.
4. **Rootless Isolation**: Evaluates Android seccomp filtering and ptrace readiness.

Each subsystem returns a `SubsystemHealth` struct with:
- `name`: Subsystem identifier.
- `level`: `Healthy`, `Degraded`, `Critical`, `Unknown`.
- `latency_ms`: Execution latency in milliseconds.
- `message`: Diagnostic human-readable string.

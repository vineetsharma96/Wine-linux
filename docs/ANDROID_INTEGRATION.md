# Android Integration & JNI Bridge

## 1. Native Bridge Architecture
The native bridge forms the boundary between the Android JVM/ART runtime and the Rust engine.

### Package & Symbol Mapping
Java Package: `com.androidpc.environment.bridge.NativeBridge`
Shared Library Name: `libandroidpc_engine.so`

| JNI Method | Rust Signature | Description |
|---|---|---|
| `nativeInitEngine(configJson: String): Int` | `(JNIEnv, JClass, JString) -> jint` | Validates configuration and spins up `EngineCore` |
| `nativeGetEngineVersion(): String` | `(JNIEnv, JClass) -> jstring` | Returns semantic version string (`0.1.0-alpha.0`) |
| `nativeGetEngineStatus(): String` | `(JNIEnv, JClass) -> jstring` | Returns JSON of `EngineStatusInfo` |
| `nativeRunHealthCheck(): String` | `(JNIEnv, JClass) -> jstring` | Triggers subsystem evaluation, returns `HealthReport` |
| `nativeGetDiagnostics(): String` | `(JNIEnv, JClass) -> jstring` | Returns `SystemDiagnostics` with ptrace & seccomp telemetry |
| `nativeGetRecentLogs(limit: Int): String` | `(JNIEnv, JClass, jint) -> jstring` | Streams log entries from Rust in-memory ring buffer |

## 2. Safety & Exception Handling
- **`std::panic::catch_unwind`**: Every single JNI export is wrapped in `catch_unwind(AssertUnwindSafe(...))`. Rust panics cannot cross the FFI boundary into ART, avoiding SIGABRT.
- **Asynchronous Execution**: In Kotlin, all `NativeBridge` calls are suspended functions running on `Dispatchers.IO`. The Android UI thread is never blocked by native engine operations.

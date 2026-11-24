# Contributing to Android PC Environment

We welcome contributions from systems engineers, Rust developers, and Android engineers!

## Development Principles
1. **Rust-First**: Core logic, process supervision, filesystem operations, and display bridges must be implemented in Rust.
2. **Zero JVM Duplication**: Kotlin handles Android lifecycle and UI. Do not duplicate domain business logic in Kotlin.
3. **No Panics across FFI**: All native JNI functions must be wrapped in `std::panic::catch_unwind`.
4. **Rootless by Design**: Never rely on device root permissions (`su`) or unsafe root access assumptions.
5. **Verified Builds**: All changes must compile with `cargo test --workspace` and cross-compile cleanly with `cargo-ndk`.

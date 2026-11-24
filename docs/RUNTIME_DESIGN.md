# Runtime Design & Abstraction Model

## 1. Runtime Trait Hierarchy
The engine provides a uniform interface for rootless Linux guest execution:

### `RuntimeProvider`
```rust
pub trait RuntimeProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn is_supported(&self, arch: Architecture) -> bool;
    fn health_check(&self) -> Result<HealthReport, RuntimeError>;
    fn check_ptrace_compatibility(&self) -> Result<bool, RuntimeError>;
}
```

### `LinuxDistribution`
```rust
pub trait LinuxDistribution: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn version(&self) -> &str;
    fn supported_architectures(&self) -> &[Architecture];
    fn default_desktop_environment(&self) -> &str;
    fn required_packages(&self) -> &[&str];
}
```

## 2. Supported Guest Distributions
1. **Ubuntu 24.04 LTS (Noble Numbat)**
   - Architectures: `aarch64` (ARM64), `x86_64`
   - Desktop: XFCE 4
   - Display: Xvnc (TigerVNC standalone server)
2. **Debian 12 (Bookworm)**
   - Architectures: `aarch64` (ARM64), `x86_64`, `armv7`
   - Desktop: XFCE 4
   - Display: Xvnc (TigerVNC standalone server)

Both distributions adhere to the identical `LinuxDistribution` trait, enabling the core engine to manage downloads, rootfs unpack, and process supervision without code duplication.

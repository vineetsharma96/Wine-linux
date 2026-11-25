pub mod elf;
pub mod env;
pub mod resolver;
pub mod router;
pub mod suite;

pub use androidpc_configuration::{
    is_android_bionic_path, Box64AppOverride, Box64EnvironmentConfig, Box64LogLevel,
};
pub use elf::{
    inspect_elf_bytes, inspect_guest_binary, ElfArchitecture, ElfClass, ElfEndianness,
    ElfInspection, ElfObjectType,
};
pub use env::{build_box64_env_vars, render_box64_rcfile};
pub use resolver::{resolve_guest_libraries, LibraryResolutionReport};
pub use router::{
    Box64CompatibilityLayer, Box64ErrorKind, Box64ExecutionOutcome, Box64ExecutionPlan,
    Box64ExecutionRecord, Box64InstallationStatus, Box64RouteDecision, CompatibilityError,
};
pub use suite::{
    build_compatibility_test_matrix, CompatibilitySuiteReport, CompatibilityTestCase,
    CompatibilityTestCategory, CompatibilityTestStatus,
};

pub const BOX64_PINNED_VERSION: &str = "0.4.4";
pub const BOX64_LICENSE: &str = "MIT";
pub const BOX64_UPSTREAM_REPO: &str = "https://github.com/ptitSeb/box64";
pub const BOX64_DOCS_URL: &str = "https://box64.org/";

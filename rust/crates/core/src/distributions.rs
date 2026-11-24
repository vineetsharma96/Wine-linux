use androidpc_runtime_api::{Architecture, LinuxDistribution};

pub struct UbuntuDistribution {
    supported: Vec<Architecture>,
    packages: Vec<&'static str>,
}

impl Default for UbuntuDistribution {
    fn default() -> Self {
        Self::new()
    }
}

impl UbuntuDistribution {
    pub fn new() -> Self {
        Self {
            supported: vec![Architecture::Aarch64, Architecture::X86_64],
            packages: vec!["bash", "coreutils", "sudo", "xfce4", "tigervnc-standalone-server"],
        }
    }
}

impl LinuxDistribution for UbuntuDistribution {
    fn id(&self) -> &str {
        "ubuntu"
    }

    fn display_name(&self) -> &str {
        "Ubuntu 24.04 LTS (Noble Numbat)"
    }

    fn version(&self) -> &str {
        "24.04"
    }

    fn supported_architectures(&self) -> &[Architecture] {
        &self.supported
    }

    fn default_desktop_environment(&self) -> &str {
        "XFCE 4"
    }

    fn required_packages(&self) -> &[&str] {
        &self.packages
    }
}

pub struct DebianDistribution {
    supported: Vec<Architecture>,
    packages: Vec<&'static str>,
}

impl Default for DebianDistribution {
    fn default() -> Self {
        Self::new()
    }
}

impl DebianDistribution {
    pub fn new() -> Self {
        Self {
            supported: vec![Architecture::Aarch64, Architecture::X86_64, Architecture::Armv7],
            packages: vec!["bash", "coreutils", "sudo", "xfce4", "tigervnc-standalone-server"],
        }
    }
}

impl LinuxDistribution for DebianDistribution {
    fn id(&self) -> &str {
        "debian"
    }

    fn display_name(&self) -> &str {
        "Debian 12 (Bookworm)"
    }

    fn version(&self) -> &str {
        "12"
    }

    fn supported_architectures(&self) -> &[Architecture] {
        &self.supported
    }

    fn default_desktop_environment(&self) -> &str {
        "XFCE 4"
    }

    fn required_packages(&self) -> &[&str] {
        &self.packages
    }
}

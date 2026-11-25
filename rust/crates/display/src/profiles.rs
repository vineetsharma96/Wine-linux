use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopProfileKind {
    Minimal,
    Standard,
    Development,
}

impl DesktopProfileKind {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "minimal" => Some(Self::Minimal),
            "standard" => Some(Self::Standard),
            "development" | "dev" => Some(Self::Development),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Standard => "standard",
            Self::Development => "development",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DesktopProfileSpec {
    pub kind: DesktopProfileKind,
    pub version: u32,
    pub name: &'static str,
    pub description: &'static str,
    pub ubuntu_packages: &'static [&'static str],
    pub debian_packages: &'static [&'static str],
}

const MINIMAL_PACKAGES: &[&str] = &[
    "xfce4-session",
    "xfwm4",
    "xfce4-panel",
    "xfdesktop4",
    "xfce4-terminal",
    "dbus-x11",
    "x11-xserver-utils",
    "librsvg2-common",
    "tigervnc-standalone-server",
];

const STANDARD_PACKAGES: &[&str] = &[
    "xfce4-session",
    "xfwm4",
    "xfce4-panel",
    "xfdesktop4",
    "xfce4-terminal",
    "dbus-x11",
    "x11-xserver-utils",
    "librsvg2-common",
    "tigervnc-standalone-server",
    "thunar",
    "mousepad",
    "xfce4-settings",
    "adwaita-icon-theme",
    "fonts-dejavu-core",
];

const DEVELOPMENT_PACKAGES: &[&str] = &[
    "xfce4-session",
    "xfwm4",
    "xfce4-panel",
    "xfdesktop4",
    "xfce4-terminal",
    "dbus-x11",
    "x11-xserver-utils",
    "librsvg2-common",
    "tigervnc-standalone-server",
    "thunar",
    "mousepad",
    "xfce4-settings",
    "adwaita-icon-theme",
    "fonts-dejavu-core",
    "git",
    "curl",
    "build-essential",
    "python3",
];

pub fn desktop_profiles() -> Vec<DesktopProfileSpec> {
    vec![
        DesktopProfileSpec {
            kind: DesktopProfileKind::Minimal,
            version: 1,
            name: "Minimal Desktop",
            description: "XFCE 4 core (xfwm4, panel, desktop, terminal), D-Bus, and TigerVNC Xvnc.",
            ubuntu_packages: MINIMAL_PACKAGES,
            debian_packages: MINIMAL_PACKAGES,
        },
        DesktopProfileSpec {
            kind: DesktopProfileKind::Standard,
            version: 1,
            name: "Standard Desktop",
            description: "Minimal Desktop plus Thunar file manager, Mousepad editor, Adwaita icons, and DejaVu fonts.",
            ubuntu_packages: STANDARD_PACKAGES,
            debian_packages: STANDARD_PACKAGES,
        },
        DesktopProfileSpec {
            kind: DesktopProfileKind::Development,
            version: 1,
            name: "Development Desktop",
            description: "Standard Desktop plus Git, curl, build-essential, and Python 3.",
            ubuntu_packages: DEVELOPMENT_PACKAGES,
            debian_packages: DEVELOPMENT_PACKAGES,
        },
    ]
}

pub fn packages_for_profile(kind: DesktopProfileKind, distro_id: &str) -> &'static [&'static str] {
    let spec = desktop_profiles()
        .into_iter()
        .find(|p| p.kind == kind)
        .expect("all DesktopProfileKind variants covered");
    if distro_id.eq_ignore_ascii_case("debian") {
        spec.debian_packages
    } else {
        spec.ubuntu_packages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_are_versioned_and_include_core_xfce_and_xvnc() {
        let profiles = desktop_profiles();
        assert_eq!(profiles.len(), 3);
        for p in &profiles {
            assert_eq!(p.version, 1);
            assert!(p.ubuntu_packages.contains(&"xfce4-session"));
            assert!(p.ubuntu_packages.contains(&"tigervnc-standalone-server"));
            assert!(p.debian_packages.contains(&"xfce4-session"));
            assert!(p.debian_packages.contains(&"tigervnc-standalone-server"));
        }
    }
}

use std::net::{SocketAddr, TcpListener};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkingError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid port forwarding rule: {0}")]
    InvalidPortRule(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardRule {
    pub host_port: u16,
    pub guest_port: u16,
    pub protocol: String,
}

impl PortForwardRule {
    pub fn tcp(host_port: u16, guest_port: u16) -> Result<Self, NetworkingError> {
        if host_port < 1024 || guest_port == 0 {
            return Err(NetworkingError::InvalidPortRule(format!(
                "Unprivileged host_port >= 1024 and guest_port > 0 required (got {host_port}->{guest_port})"
            )));
        }
        Ok(Self {
            host_port,
            guest_port,
            protocol: "tcp".to_string(),
        })
    }

    pub fn to_qemu_hostfwd(&self) -> String {
        format!(
            "hostfwd={}:127.0.0.1:{}-:{}",
            self.protocol, self.host_port, self.guest_port
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestNetworkConfig {
    pub hostname: String,
    pub nameservers: Vec<String>,
}

impl Default for GuestNetworkConfig {
    fn default() -> Self {
        Self {
            hostname: "android-pc".to_string(),
            nameservers: vec!["8.8.8.8".to_string(), "1.1.1.1".to_string()],
        }
    }
}

impl GuestNetworkConfig {
    pub fn write_to_rootfs(&self, rootfs: &Path) -> Result<(), NetworkingError> {
        let etc = rootfs.join("etc");
        std::fs::create_dir_all(&etc)?;

        let resolv = self
            .nameservers
            .iter()
            .map(|ns| format!("nameserver {ns}\n"))
            .collect::<String>();
        std::fs::write(etc.join("resolv.conf"), resolv)?;
        std::fs::write(
            etc.join("hosts"),
            format!("127.0.0.1 localhost {}\n::1 localhost\n", self.hostname),
        )?;
        std::fs::write(etc.join("hostname"), format!("{}\n", self.hostname))?;
        Ok(())
    }
}

/// Checks whether a localhost TCP port is currently available to bind.
pub fn is_localhost_port_available(port: u16) -> bool {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    TcpListener::bind(addr).is_ok()
}

/// Allocates an available ephemeral localhost TCP port.
pub fn allocate_ephemeral_port() -> Result<u16, NetworkingError> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_forward_rule_and_ephemeral_port() {
        assert!(PortForwardRule::tcp(80, 80).is_err());
        let rule = PortForwardRule::tcp(2222, 22).unwrap();
        assert_eq!(rule.to_qemu_hostfwd(), "hostfwd=tcp:127.0.0.1:2222-:22");

        let port = allocate_ephemeral_port().unwrap();
        assert!(port > 0);
    }

    #[test]
    fn test_guest_network_config_write() {
        let tmp = std::env::temp_dir().join("androidpc_net_test");
        let _ = std::fs::remove_dir_all(&tmp);
        GuestNetworkConfig::default().write_to_rootfs(&tmp).unwrap();
        let resolv = std::fs::read_to_string(tmp.join("etc/resolv.conf")).unwrap();
        assert!(resolv.contains("nameserver 8.8.8.8"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

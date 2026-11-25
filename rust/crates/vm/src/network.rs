use crate::config::{NetworkConfig, NetworkMode};

/// Builds rootless Android-compatible QEMU networking CLI arguments.
///
/// Android restricts `/dev/net/tun` (TAP) and Linux bridge creation to root / `CAP_NET_ADMIN`.
/// Therefore, Android PC Environment uses QEMU user-mode networking powered by `libslirp` (`v4.9.7`),
/// binding port-forwards strictly to `127.0.0.1` by default.
pub struct VmNetworkBuilder;

impl VmNetworkBuilder {
    pub fn build_args(net: &NetworkConfig) -> Vec<String> {
        if net.mode == NetworkMode::Disabled {
            return vec!["-nic".to_string(), "none".to_string()];
        }

        let mut netdev = format!("user,id=net0,net={},dns={}", net.guest_cidr, net.dns_server);
        if net.mode == NetworkMode::IsolatedSlirp {
            netdev.push_str(",restrict=on");
        }

        for rule in &net.port_forwards {
            let bind_addr = if rule.bind_loopback_only {
                "127.0.0.1"
            } else {
                ""
            };
            netdev.push_str(&format!(
                ",hostfwd={}:{}:{}-:{}",
                rule.protocol, bind_addr, rule.host_port, rule.guest_port
            ));
        }

        let nic_model = if net.nic_model.trim().is_empty() {
            "virtio-net-pci"
        } else {
            net.nic_model.trim()
        };

        vec![
            "-netdev".to_string(),
            netdev,
            "-device".to_string(),
            format!("{nic_model},netdev=net0"),
        ]
    }

    pub fn rootless_limitations_summary() -> &'static str {
        "Rootless Android uses QEMU user-mode SLIRP (libslirp 4.9.7): outbound TCP/UDP and loopback hostfwd work without root; raw TAP/bridge L2 networking and unprivileged ICMP ping are unavailable without CAP_NET_ADMIN."
    }
}

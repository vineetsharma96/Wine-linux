# Virtual Machine Networking (`VM_NETWORKING.md`)

## 1. Rootless Android Networking Strategy (`libslirp 4.9.7`)
Stock Android denies ordinary apps access to `/dev/net/tun` (TAP interfaces), Linux bridges, and `CAP_NET_ADMIN` / `CAP_NET_RAW`. Therefore, Android PC Environment uses **QEMU user-mode networking (`-netdev user`, backed by `libslirp v4.9.7`)** attached to a paravirtualized `virtio-net-pci` NIC.

## 2. Supported Modes (`rust/crates/vm/src/network.rs`)
| Mode | QEMU Flags | Behavior |
|---|---|---|
| **User SLIRP NAT (`user_slirp`, default)** | `-netdev user,id=net0,net=10.0.2.0/24,dns=10.0.2.3,...` | Outbound guest TCP/UDP + DHCP + DNS via Android's network stack without root; supports loopback `hostfwd` |
| **Isolated SLIRP (`isolated_slirp`)** | `-netdev user,id=net0,...,restrict=on` | Blocks outbound internet/LAN connections while retaining local virtual subnet & explicit loopback port forwards |
| **Disabled (`disabled`)** | `-nic none` | Completely removes virtual network hardware for offline/air-gapped guest testing |

## 3. Port Forwarding & Hardening
- Default rule: `hostfwd=tcp:127.0.0.1:2222-:22` (SSH access from Android host/terminal strictly on loopback).
- `VmProfile::validate()` enforces:
  - Only `tcp` and `udp` protocols are permitted.
  - Host ports `< 1024` are rejected (unprivileged Android apps cannot bind privileged ports).
  - `bind_loopback_only = true` by default so guest services are never inadvertently exposed on the device's Wi-Fi/LAN interface.

## 4. Documented Limitations
- **No L2 TAP / Bridge networking** without root or `CAP_NET_ADMIN`.
- **No unprivileged ICMP `ping` to external hosts** (Android restricts `SOCK_DGRAM` `IPPROTO_ICMP` unless permitted by kernel `ping_group_range`; verify connectivity using TCP/HTTP/DNS tools such as `curl` or `apk update` / `apt-get update`).
- **Guest-to-host isolation**: Guests reach outbound internet through Android's `INTERNET` permission under the app UID, subject to active Android VPN/firewall policies.

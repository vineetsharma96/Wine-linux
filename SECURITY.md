# Security Policy

## Supported Versions
| Version | Supported |
|---|---|
| 0.1.x-alpha | Yes (Active Development) |

## Rootless Security Boundaries
Android PC Environment executes strictly within the unprivileged Android application sandbox:
- Storage is restricted to app-private directory (`/data/data/com.androidpc.environment/files`).
- No elevated Android or Linux system capabilities are requested.
- Execution of guest Linux distributions utilizes ptrace-based user namespaces (`PRoot`).
- All downloaded artifacts and root filesystems must undergo SHA-256 cryptographic checksum validation before execution.

## Reporting a Vulnerability
To report a security vulnerability, please submit an issue or contact the maintainers with reproducible steps and target environment details.

# Linux distributions

The ARM64 catalog offers Ubuntu Base 26.04.1 LTS and Debian 13.8. The app shows the catalog artifact for the selected distribution before installation. Each current catalog contains one approved release per distribution. Release builds require a verifiable Ed25519 catalog signature, and the installer checks the expected archive size, SHA-256 and actual guest release.

The distribution release is recorded on each environment. The PRoot executable version is determined separately and recorded when available. Installing or restoring an environment does not replace the packaged PRoot executable. See [root filesystem distribution](ROOTFS_DISTRIBUTION.md) and the [version inventory](VERSION_INVENTORY.md) for the upstream artifacts and verification process.

# Root filesystem distribution

The development catalog selects Ubuntu Base 26.04.1 from Canonical and the Debian 13.8 slim ARM64 root filesystem from Docker Official Images' pinned Debian source commit. The Ubuntu archive was checked against Canonical's signed SHA-256 list using the published Ubuntu CD Image signing fingerprint. The Debian archive was checked against the ARM64 OCI layer digest from the producer's pinned source. Both inputs are recorded in [arm64-inputs.json](../distribution-builder/arm64-inputs.json).

Each entry supplies a fixed URL, expected file size and SHA-256. The app downloads over HTTPS into its private cache. An interrupted download can resume if the server returns a matching Content-Range, or restart if the server returns a full response. Rust checks both the length and digest before extraction and selects compression from the catalog filename.

The Ubuntu 26.04.1 guest consumed approximately 1.25 GiB in the test device's private storage. The app reserves 1.5 GiB for that Ubuntu installation path. The Debian 13.8 guest consumed approximately 108 MiB of allocated storage.

Release builds require a signed, versioned manifest. Rust verifies the detached Ed25519 signature over the original catalog bytes with the APK's public key before parsing its schema, validating the artifact fields, or offering a download. It matches the booted guest version against the catalog. The unsigned development catalog is available to debug builds without a configured key.

Review and verify the upstream inputs before signing. Prepare a JSON array with one Ubuntu and one Debian entry. Each input includes `distribution`, `release` (guest `VERSION_ID`), `artifact_version`, `filename`, `url`, `archive_format`, `sha256`, `verification_source`, and `min_app_version_code`. The checksum must come from authenticated upstream metadata. The script documents its provenance and verifies that the archive matches it. Acquisition and upstream signature verification are separate steps.

```sh
python distribution-builder/build_rootfs_catalog.py \
  --inputs distribution-builder/arm64-inputs.json --archives-dir verified-archives \
  --ed25519-key /path/to/release-signing-key.pem \
  --output android/app/src/release/assets/rootfs
```

Set `ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX` to the printed public key before building the release APK. Keep the private key and staging files outside source control or under release pipeline control. The repository contains no production signing key.

"""Build a signed Android catalog from reviewed, pinned rootfs inputs."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import urlsplit


def artifact_from_input(source, archive_dir):
    filename = source["filename"]
    if not re.fullmatch(r"[a-zA-Z0-9_-][a-zA-Z0-9._-]{0,127}", filename):
        raise ValueError(f"Invalid archive filename: {filename}")
    archive_format = source["archive_format"]
    if archive_format not in {"tar.gz", "tar.xz"} or not filename.endswith("." + archive_format):
        raise ValueError(f"Archive format mismatch: {filename}")
    url = urlsplit(source["url"])
    if url.scheme != "https" or not url.hostname or url.username or url.fragment:
        raise ValueError(f"Expected a public HTTPS download: {filename}")
    expected = source["sha256"].lower()
    if not re.fullmatch(r"[a-f0-9]{64}", expected) or expected == "0" * 64:
        raise ValueError(f"Missing authenticated checksum: {filename}")
    if not source.get("verification_source", "").startswith("https://"):
        raise ValueError(f"Missing upstream checksum or signature provenance: {filename}")
    archive = archive_dir / filename
    with archive.open("rb") as handle:
        actual = hashlib.file_digest(handle, "sha256").hexdigest()
    if actual != expected:
        raise ValueError(f"Upstream checksum mismatch: {filename}")
    if not archive.stat().st_size:
        raise ValueError(f"Empty archive: {filename}")
    for field in ("release", "artifact_version"):
        if not re.fullmatch(r"[a-zA-Z0-9._-]{1,128}", source[field]):
            raise ValueError(f"Invalid {field}: {filename}")
    if type(source["min_app_version_code"]) is not int or source["min_app_version_code"] < 1:
        raise ValueError(f"Invalid minimum app version: {filename}")
    return {
        "distribution": source["distribution"],
        "release": source["release"],
        "artifact_version": source["artifact_version"],
        "architecture": "aarch64",
        "filename": filename,
        "url": source["url"],
        "archive_format": archive_format,
        "sha256": actual,
        "size_bytes": archive.stat().st_size,
        "min_app_version_code": source["min_app_version_code"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--archives-dir", type=Path, required=True)
    parser.add_argument("--ed25519-key", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--openssl", default="openssl")
    args = parser.parse_args()

    sources = json.loads(args.inputs.read_text(encoding="utf-8"))
    expected_distros = ["debian", "ubuntu"]
    if sorted(source["distribution"] for source in sources) != expected_distros:
        raise ValueError("Exactly one Ubuntu and one Debian input is required")
    rootfs = [artifact_from_input(source, args.archives_dir)
              for source in sorted(sources, key=lambda entry: entry["distribution"])]
    encoded = json.dumps(
        {"schema_version": 1, "rootfs": rootfs}, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")

    public_der = subprocess.check_output([
        args.openssl, "pkey", "-in", str(args.ed25519_key), "-pubout", "-outform", "DER"
    ])
    prefix = bytes.fromhex("302a300506032b6570032100")
    if not public_der.startswith(prefix) or len(public_der) != len(prefix) + 32:
        raise ValueError("Signing key must be Ed25519")
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=args.output) as temporary:
        manifest = Path(temporary) / "catalog.json"
        signature = Path(temporary) / "catalog.sig"
        manifest.write_bytes(encoded)
        subprocess.run([
            args.openssl, "pkeyutl", "-sign", "-rawin", "-inkey", str(args.ed25519_key),
            "-in", str(manifest), "-out", str(signature)
        ], check=True)
        if signature.stat().st_size != 64:
            raise ValueError("Unexpected signature size")
        manifest.replace(args.output / "catalog.json")
        signature.replace(args.output / "catalog.sig")
    print("ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX=" + public_der[len(prefix):].hex())


if __name__ == "__main__":
    main()

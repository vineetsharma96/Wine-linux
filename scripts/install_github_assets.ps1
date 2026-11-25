# Reproducible GitHub / Termux ARM64 Asset Installer for Android PC Environment
# Downloads, verifies SHA-256 checksums, and stages runtime binaries into assets/, runtime/, and android/app/src/main/
param(
    [string]$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"
$Staging = Join-Path $RootDir "assets\staging"
New-Item -ItemType Directory -Force $Staging | Out-Null

$Manifest = @(
    @{ Name = "proot.deb"; Url = "https://packages.termux.dev/apt/termux-main/pool/main/p/proot/proot_5.1.107.102_aarch64.deb"; Sha256 = "10a394b585c2fc2b2c8502544f4e5f1be581dc8bfff79e8ba62f16207423e030" },
    @{ Name = "libtalloc.deb"; Url = "https://packages.termux.dev/apt/termux-main/pool/main/libt/libtalloc/libtalloc_2.5.0_aarch64.deb"; Sha256 = "556591f43bb773ad8777e1a29522640866a55f95dab71914418b94a8c58ad5a7" },
    @{ Name = "libandroid-shmem.deb"; Url = "https://packages.termux.dev/apt/termux-main/pool/main/liba/libandroid-shmem/libandroid-shmem_0.7_aarch64.deb"; Sha256 = "0da3a24d558b93c92bcf8d611e0826a99ff96e396b148e6cdf33b47c47c57ff6" },
    @{ Name = "tigervnc.deb"; Url = "https://packages.termux.dev/apt/termux-x11/pool/main/t/tigervnc/tigervnc_1.16.2-2_aarch64.deb"; Sha256 = "8dacf03fb8a132e56745316c3c0308f6deb7dca252f9e3fd5a4166444d2f8109" },
    @{ Name = "box64-0.4.5.tzst"; Url = "https://raw.githubusercontent.com/brunodev85/winlator/main/installable_components/box64/box64-0.4.5.tzst"; Sha256 = "ba9e8acb06b9d8a33b8c8fadda6b2790b313ee38659232b59c0752ae00209911" },
    @{ Name = "default.box64rc"; Url = "https://raw.githubusercontent.com/brunodev85/winlator-app/main/app/src/main/assets/box64/default.box64rc"; Sha256 = "6ebf56aacc64e3befe68c908559c202ec10cc9bca98e0b9dd27b111d642c7872" },
    @{ Name = "env_vars.json"; Url = "https://raw.githubusercontent.com/brunodev85/winlator-app/main/app/src/main/assets/box64/env_vars.json"; Sha256 = "0da519c92f2ffe7570288013e67e1d81424ac041460719747798d31f557e838e" }
)

foreach ($item in $Manifest) {
    $dest = Join-Path $Staging $item.Name
    Write-Host "Downloading $($item.Name)..."
    curl.exe -sL $item.Url -o $dest
    $actual = (Get-FileHash $dest -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $item.Sha256) {
        throw "SHA-256 mismatch for $($item.Name): $actual != $($item.Sha256)"
    }
    Write-Host "  Verified SHA-256: $actual"
}

Write-Host "All GitHub/Termux assets downloaded and verified into $Staging."

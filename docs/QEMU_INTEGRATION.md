# QEMU Integration Guide (`11.1.3`)

## 1. Pinned Runtime Manifest & Artifacts
The QEMU runtime specification is tracked in:
- `/runtime/qemu/manifest.json`
- `/app/src/main/assets/runtime/qemu/manifest.json` (unpacked by `AssetInstaller.kt` into `filesDir/artifacts/qemu/`)
- `app/build.gradle.kts` (`BuildConfig.QEMU_VERSION = "11.1.3"`, `EDK2_VERSION = "edk2-stable202608"`, `LIBSLIRP_VERSION = "4.9.7"`)
- `rust/crates/downloader/src/artifact.rs` (`qemu-11.1.3-arm64`, `edk2-aavmf-202608`, `edk2-ovmf-202608`, `libslirp-4.9.7`)

## 2. Reproducible ARM64 Build Recipe
Run `/scripts/build_qemu_arm64.sh` on an ARM64 Linux or Android NDK cross-compilation host with `meson`, `ninja`, `glib2`, `pixman`, `zlib`, `zstd`, and `libslirp 4.9.7`:
- Targets built: `aarch64-softmmu`, `x86_64-softmmu`, `qemu-img`.
- Enabled subsystems: `--enable-tcg`, `--enable-slirp`, `--enable-vnc`, `--audio-drv-list=pa,wav`.
- Disabled subsystems: `--disable-gtk`, `--disable-sdl`, `--disable-spice`, `--disable-xen`, `--disable-curses`, `--enable-virtfs=no` (no host-folder sharing by default).

## 3. Generated QEMU Command Profiles
### ARM64 Guest (`qemu-system-aarch64`)
```text
qemu-system-aarch64 \
  -name <vm_id> \
  -machine virt,gic-version=3 \
  -cpu cortex-a72 \
  -accel tcg,thread=multi,tb-size=128 \
  -m <memory_mb>M -smp cores=<cores>,threads=1 \
  -drive if=pflash,format=raw,readonly=on,file=<artifacts>/qemu-firmware/AAVMF_CODE.fd \
  -drive file=<vm_dir>/disk0.qcow2,if=virtio,format=qcow2,cache=writeback \
  -netdev user,id=net0,net=10.0.2.0/24,dns=10.0.2.3,hostfwd=tcp:127.0.0.1:2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -device virtio-gpu-pci,xres=1280,yres=720 \
  -display vnc=127.0.0.1:10 \
  -device qemu-xhci,id=xhci -device usb-tablet,bus=xhci.0 -device usb-kbd,bus=xhci.0 \
  -audiodev none,id=snd0 \
  -qmp tcp:127.0.0.1:4444,server,nowait \
  -nodefaults -no-reboot
```
*(If `/dev/kvm` is verified R/W accessible and the user enables KVM on an ARM64 guest, `-cpu host -accel kvm` replaces TCG.)*

### x86-64 Guest (`qemu-system-x86_64`)
Uses `-machine q35 -cpu max -accel tcg,thread=multi,tb-size=128` with `OVMF_CODE.fd` (UEFI) or SeaBIOS and `virtio-vga`. KVM is **explicitly blocked** for x86-64 guests on ARM64 hosts.

## 4. Current Staging Status
- Rust engine, JNI exports, manifests, build recipe, and Android UI are implemented and Android APK compiles (`compile_applet` PASS).
- Compiled ARM64 `qemu-system-aarch64`, `qemu-system-x86_64`, `qemu-img`, and EDK2 firmware `.fd` binaries are **not** bundled in this repository and must be built/staged before live VM boot testing on hardware.

# Rootless Linux runtime

The production path is the Android Compose screen, the Kotlin JNI bridge, the Rust engine, and the Android-packaged PRoot executable. Guests share the Android kernel, and PRoot translates guest file paths. Root inside a guest is emulated and cannot grant root privileges on the device.

The installer accepts the catalog's Ubuntu and Debian ARM64 archives. Rust checks the archive size and SHA-256, extracts into a private staging directory, configures the userspace, and executes a real guest shell to check the architecture, distribution, release and package manager. After those checks, Rust saves the config and commits the directory by rename. The app opens the terminal only when the install reports success.

The native binary and loaders must be packaged in the APK native library directory. See [device compatibility](PROOT_COMPATIBILITY.md) and [testing](TESTING.md).

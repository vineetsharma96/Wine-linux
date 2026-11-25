# Runtime verification

Run host tests with `cargo test --locked --workspace` from `rust/`. Check Android-specific code with `cargo check --locked --target aarch64-linux-android --workspace`. A full ARM64 Android build also needs the Android NDK and the native engine is linked separately before Gradle packages the APK. Use the pinned toolchain versions in the [version inventory](VERSION_INVENTORY.md).

The default Android instrumentation test checks JNI initialization, basic diagnostics and the asset catalog. The additional real-device test is opt-in. Stage the exact catalog archives in the debug app's private cache as `acceptance-ubuntu.part` and `acceptance-debian.part`, then run:

```sh
adb shell am instrument -w -e rootfsAcceptance true com.androidpc.environment.debug.test/androidx.test.runner.AndroidJUnitRunner
```

The test rejects a corrupt fixture, installs each verified guest under a temporary ID, rejects a duplicate install, attaches the PTY, and asserts `uname -m`, `/etc/os-release`, `apt --version` and the resized PTY dimensions. It closes and deletes the temporary environments. This fixture test exercises native installation and PTY input/output. HTTPS interruption and Compose interactions require separate checks.

For a test-signed debug APK, pass `-e signedCatalog true` to assert that the signed catalog replaced the default. Pass `-e rootfsNetwork true` with the fixtures staged to seed a partial archive and exercise the app's HTTPS resume path. The newer automated test was built. Android declined the test APK install on the available phone, so the current Ubuntu and Debian results were confirmed through the installed Compose app using full HTTPS downloads.

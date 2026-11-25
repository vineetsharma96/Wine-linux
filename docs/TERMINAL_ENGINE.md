# Terminal engine

The Android terminal uses a Unix PTY. Rust launches PRoot with structured arguments and a sanitized environment for the selected guest, and attaches a real shell to the slave end. The bridge supports writing input bytes, polling output, resizing the PTY, reporting exit status and closing the session. The app resizes the PTY to match the viewport. An unavailable guest shell or configuration produces an error.

The device acceptance test uses the same JNI terminal calls as the app. It runs guest commands and checks the dimensions reported by `stty size` after resizing. See [testing](TESTING.md).

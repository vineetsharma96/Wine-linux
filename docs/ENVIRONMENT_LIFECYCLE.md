# Environment lifecycle

A new install owns a hidden staging directory under the app's private environment directory. Rust serializes native install attempts, rejects duplicate IDs, and discards a stale staging directory when the same ID is retried. Staging directories do not appear in the installed list. A successful guest check is followed by config write and atomic directory rename. Failed verification, extraction, and initialization cannot publish a new environment.

The installer exposes verification, extraction and guest initialization status while the native operation runs. A cancellation is applied at the next phase boundary and prevents registration. The UI can cancel a download and retain the partial file. Startup clears interrupted staging.

Registered environments have Rust-backed start, stop, restart, repair, inspect, terminal and delete operations. Repair regenerates runtime setup and runs the guest health check again. Stop closes the guest PTYs and waits for supervised processes. Delete moves the environment out of the registry directory before removing it. Configuration stores a schema number, creation timestamp and the last successful installation check.

# Resource measurements

Storage totals use allocated filesystem blocks on Android and count hard-linked data once. Storage and lifecycle values come from Rust. Details can refresh the number of live supervised processes and PTYs, and samples resident memory and cumulative CPU time through procfs when Android permits it. The samples cover the supervised PRoot processes and PTYs. They do not total all descendant guest processes or enforce a cgroup resource limit.

The dashboard shows engine uptime and health. The environment's last-started time is persisted across app restarts. Its Details view reports elapsed time since that launch only while supervised processes are present. Metrics refresh when the screen opens or when the user requests a refresh. Package output is polled only while a package command is active.

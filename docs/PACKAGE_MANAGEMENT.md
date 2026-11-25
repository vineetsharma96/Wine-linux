# Guest packages

Open Details for a stopped Ubuntu or Debian environment. Rust maps each operation to structured arguments for the guest's own APT or dpkg binary. Installed packages come from `dpkg-query`, search and package information from `apt-cache`, and index refresh, install, removal and upgrade from `apt-get`. Package-changing actions require confirmation. The app never schedules an upgrade.

The package process runs in PRoot with a controlled environment. The app shows its recent stdout, stderr and exit code. An environment rejects operations that would start, stop, delete, restore over, or take a snapshot during conflicting work. Cancel requests termination of the supervised package process. Package managers may leave their usual recovery state after interruption. Reopen Details and explicitly choose **Configure pending packages** to run `dpkg --configure -a` when appropriate.

The package commands target the versions installed in the guest. See the official [APT](https://manpages.debian.org/trixie/apt/apt-get.8.en.html) and [dpkg-query](https://manpages.debian.org/trixie/dpkg/dpkg-query.1.en.html) documentation for command behavior and package lock handling.

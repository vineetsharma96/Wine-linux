use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::{CoreError, EngineCore};
use androidpc_runtime_api::EnvironmentState;

pub(crate) struct PackageJob {
    pub(crate) state: EnvironmentState,
    pub(crate) cancelled: AtomicBool,
    stdout: Mutex<Vec<u8>>,
    stderr: Mutex<Vec<u8>>,
}

#[derive(Serialize)]
pub struct PackageResult {
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub stdout: String,
    pub stderr: String,
}

impl PackageJob {
    pub(crate) fn new(state: EnvironmentState) -> Self {
        Self { state, cancelled: AtomicBool::new(false), stdout: Mutex::new(Vec::new()), stderr: Mutex::new(Vec::new()) }
    }

    fn snapshot(&self, exit_code: Option<i32>) -> PackageResult {
        let read = |buffer: &Mutex<Vec<u8>>| {
            buffer.lock().map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default()
        };
        PackageResult {
            exit_code,
            cancelled: self.cancelled.load(Ordering::Acquire),
            stdout: read(&self.stdout),
            stderr: read(&self.stderr),
        }
    }
}

fn plan(action: &str, query: &str) -> Result<Vec<String>, CoreError> {
    let invalid = |message: &str| CoreError::SubsystemFailure(message.to_owned());
    let packages = || {
        let names = query.split_whitespace().collect::<Vec<_>>();
        if names.is_empty() || names.len() > 32 || names.iter().any(|name| {
            name.len() > 128 || !name.as_bytes()[0].is_ascii_alphanumeric()
                || !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'+' | b'.' | b'-'))
        }) {
            return Err(invalid("Enter Debian package names separated by spaces"));
        }
        Ok(names)
    };
    let words = match action {
        "list" => vec!["/usr/bin/dpkg-query", "-W", "--showformat=${binary:Package}\\t${Version}\\n"],
        "update" => vec!["/usr/bin/apt-get", "update"],
        "upgrade" => vec!["/usr/bin/apt-get", "-y", "upgrade"],
        "repair" => vec!["/usr/bin/dpkg", "--configure", "-a"],
        "install" | "remove" => {
            let mut args = vec!["/usr/bin/apt-get".into(), "-y".into(), action.into(), "--".into()];
            // Architecture qualifiers keep trailing '+' or '-' in package names
            // from becoming apt-get operation modifiers.
            args.extend(packages()?.into_iter().map(|name| format!("{name}:arm64")));
            return Ok(args);
        }
        "install-minimal" => {
            let mut args = vec![
                "/usr/bin/apt-get".into(),
                "-y".into(),
                "--no-install-recommends".into(),
                "install".into(),
                "--".into(),
            ];
            args.extend(packages()?.into_iter().map(|name| format!("{name}:arm64")));
            return Ok(args);
        }
        "details" => {
            let mut args = vec!["/usr/bin/apt-cache".into(), "show".into(), "--".into()];
            args.extend(packages()?.into_iter().map(str::to_owned));
            return Ok(args);
        }
        "search" => {
            let query = query.trim();
            if query.is_empty() || query.len() > 128 || query.chars().any(char::is_control) {
                return Err(invalid("Enter a search term with 1–128 bytes"));
            }
            let literal = query.chars().flat_map(|character| {
                let escape = ".^$*+?()[]{}|\\".contains(character);
                [escape.then_some('\\'), Some(character)].into_iter().flatten()
            }).collect::<String>();
            return Ok(vec!["/usr/bin/apt-cache".into(), "search".into(), "--".into(), literal]);
        }
        _ => return Err(invalid("Unknown package operation")),
    };
    Ok(words.into_iter().map(str::to_owned).collect())
}

impl EngineCore {
    pub fn run_package_operation(&self, id: &str, action: &str, query: &str) -> Result<PackageResult, CoreError> {
        let args = plan(action, query)?;
        let job = Arc::new(PackageJob::new(EnvironmentState::Updating));
        let mut command = {
            // Acquire locks in the same order as Start, Stop and Remove.
            let environments = self.environments.write()
                .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
            let manager = environments.get(id)
                .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
            if manager.is_running() || self.has_live_terminal(id)? || self.has_live_desktop(id)? {
                return Err(CoreError::SubsystemFailure("Stop the environment before running package operations".into()));
            }
            let provider = self.resolve_proot_provider();
            let mut command = Command::new(provider.proot_binary());
            command.args(provider.build_proot_command(manager.config(), &args))
                .env_clear().envs(manager.build_guest_env(&[]))
                .env("DEBIAN_FRONTEND", "noninteractive").env("LC_ALL", "C")
                .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut jobs = self.package_jobs.lock()
                .map_err(|_| CoreError::SubsystemFailure("Package operation lock poisoned".into()))?;
            if jobs.contains_key(id) {
                return Err(CoreError::SubsystemFailure("A package operation is already running".into()));
            }
            jobs.insert(id.to_owned(), job.clone());
            command
        };
        let result = run(&mut command, &job);
        if let Ok(mut jobs) = self.package_jobs.lock() {
            jobs.remove(id);
        }
        result.map_err(CoreError::from)
    }

    pub fn package_operation_progress(&self, id: &str) -> Option<PackageResult> {
        let job = self.package_jobs.lock().ok()?.get(id)?.clone();
        Some(job.snapshot(None))
    }

    pub fn cancel_package_operation(&self, id: &str) -> bool {
        let Some(job) = self.package_jobs.lock().ok().and_then(|jobs| jobs.get(id).cloned()) else {
            return false;
        };
        job.cancelled.store(true, Ordering::Release);
        true
    }

    pub(crate) fn ensure_no_package_operation(&self, id: &str) -> Result<(), CoreError> {
        let jobs = self.package_jobs.lock()
            .map_err(|_| CoreError::SubsystemFailure("Package operation lock poisoned".into()))?;
        if jobs.contains_key(id) {
            return Err(CoreError::SubsystemFailure("Wait for the environment operation to finish".into()));
        }
        Ok(())
    }

    pub(crate) fn package_state(&self, id: &str) -> Option<EnvironmentState> {
        self.package_jobs.lock().ok()?.get(id).map(|job| job.state)
    }
}

fn drain(mut stream: impl Read, output: &Mutex<Vec<u8>>) {
    const MAX_OUTPUT: usize = 64 * 1024;
    let mut chunk = [0; 4096];
    while let Ok(count) = stream.read(&mut chunk) {
        if count == 0 { break; }
        if let Ok(mut buffer) = output.lock() {
            let excess = (buffer.len() + count).saturating_sub(MAX_OUTPUT);
            buffer.drain(..excess);
            buffer.extend_from_slice(&chunk[..count]);
        }
    }
}

fn run(command: &mut Command, job: &PackageJob) -> std::io::Result<PackageResult> {
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("stdout configured");
    let stderr = child.stderr.take().expect("stderr configured");
    let status = std::thread::scope(|scope| {
        scope.spawn(|| drain(stdout, &job.stdout));
        scope.spawn(|| drain(stderr, &job.stderr));
        let mut stopping = None;
        loop {
            let status = match child.try_wait() {
                Ok(status) => status,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(error);
                }
            };
            if let Some(status) = status { break Ok(status); }
            if job.cancelled.load(Ordering::Acquire) {
                if stopping.is_none() {
                    #[cfg(unix)]
                    unsafe { libc::kill(child.id() as i32, libc::SIGTERM); }
                    #[cfg(not(unix))]
                    let _ = child.kill();
                    stopping = Some(Instant::now());
                } else if stopping.is_some_and(|start| start.elapsed() >= Duration::from_secs(2)) {
                    let _ = child.kill();
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    })?;
    #[cfg(unix)]
    let code = {
        use std::os::unix::process::ExitStatusExt;
        status.code().or_else(|| status.signal().map(|signal| 128 + signal))
    };
    #[cfg(not(unix))]
    let code = status.code();
    Ok(job.snapshot(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_requests_use_structured_arguments() {
        assert_eq!(plan("install", "g++ bash").unwrap(), ["/usr/bin/apt-get", "-y", "install", "--", "g++:arm64", "bash:arm64"]);
        assert!(plan("remove", "-o unsafe").is_err());
        assert!(plan("install", "bash; sh").is_err());
        assert_eq!(plan("list", "").unwrap()[0], "/usr/bin/dpkg-query");
    }

    #[test]
    #[cfg(any(unix, windows))]
    fn cancellation_reaps_a_live_child() {
        #[cfg(unix)]
        let mut command = {
            let mut command = Command::new("sleep");
            command.arg("20");
            command
        };
        #[cfg(windows)]
        let mut command = {
            let mut command = Command::new("ping.exe");
            command.args(["-n", "20", "127.0.0.1"]);
            command
        };
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let job = PackageJob::new(EnvironmentState::Updating);
        let result = std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                job.cancelled.store(true, Ordering::Release);
            });
            run(&mut command, &job).unwrap()
        });
        assert!(result.cancelled);
        assert_ne!(result.exit_code, Some(0));
    }
}

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, RwLock};
use tracing::{error, info};

use androidpc_runtime_api::{
    ProcessCommand, ProcessId, ProcessInfo, ProcessSupervisor, RuntimeError, Signal,
};
use crate::signal::SignalTranslator;

pub struct ManagedProcess {
    pub pid: ProcessId,
    pub command: String,
    pub child: Option<Child>,
}

#[derive(Clone)]
pub struct DefaultProcessSupervisor {
    processes: Arc<RwLock<HashMap<ProcessId, ManagedProcess>>>,
}

impl Default for DefaultProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultProcessSupervisor {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ProcessSupervisor for DefaultProcessSupervisor {
    fn spawn_process(&self, command: &ProcessCommand) -> Result<ProcessId, RuntimeError> {
        let mut cmd = Command::new(&command.program);
        cmd.args(&command.args);

        for (k, v) in &command.env {
            cmd.env(k, v);
        }

        if let Some(ref cwd) = command.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| {
            error!("Failed to spawn process '{}': {e}", command.program);
            RuntimeError::SupervisorError(format!("Spawn failed: {e}"))
        })?;

        let pid = child.id() as ProcessId;
        info!("Spawned process '{}' with PID {}", command.program, pid);

        let managed = ManagedProcess {
            pid,
            command: format!("{} {}", command.program, command.args.join(" ")),
            child: Some(child),
        };

        if let Ok(mut map) = self.processes.write() {
            map.insert(pid, managed);
        }

        Ok(pid)
    }

    fn send_signal(&self, pid: ProcessId, signal: Signal) -> Result<(), RuntimeError> {
        info!("Sending signal {:?} to PID {}", signal, pid);

        #[cfg(unix)]
        {
            let sig_num = SignalTranslator::to_unix_signal(signal);
            let ret = unsafe { libc::kill(pid as i32, sig_num) };
            if ret != 0 {
                return Err(RuntimeError::SupervisorError(format!(
                    "kill({pid}, {sig_num}) failed: errno {}",
                    std::io::Error::last_os_error()
                )));
            }
            Ok(())
        }

        #[cfg(not(unix))]
        {
            if let Ok(mut map) = self.processes.write() {
                if let Some(p) = map.get_mut(&pid) {
                    if let Some(ref mut child) = p.child {
                        let _ = child.kill();
                    }
                }
            }
            Ok(())
        }
    }

    fn is_alive(&self, pid: ProcessId) -> bool {
        if let Ok(mut map) = self.processes.write() {
            if let Some(p) = map.get_mut(&pid) {
                if let Some(ref mut child) = p.child {
                    match child.try_wait() {
                        Ok(Some(_status)) => false,
                        Ok(None) => true,
                        Err(_) => false,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn list_processes(&self) -> Vec<ProcessInfo> {
        let mut list = Vec::new();
        if let Ok(map) = self.processes.read() {
            for (pid, p) in map.iter() {
                list.push(ProcessInfo {
                    pid: *pid,
                    command: p.command.clone(),
                    running: true,
                    memory_rss_bytes: 0,
                });
            }
        }
        list
    }
}

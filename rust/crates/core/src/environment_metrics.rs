use serde::Serialize;

use crate::{CoreError, EngineCore};
use androidpc_runtime_api::EnvironmentManager;

#[derive(Serialize)]
pub struct EnvironmentMetrics {
    pub processes: usize,
    pub terminal_sessions: usize,
    pub memory_rss_bytes: Option<u64>,
    pub cpu_time_ms: Option<u64>,
    pub seconds_since_launch: Option<u64>,
}

impl EngineCore {
    pub fn environment_metrics(&self, id: &str) -> Result<EnvironmentMetrics, CoreError> {
        let environments = self.environments.read()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock unavailable".into()))?;
        let manager = environments.get(id)
            .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
        let mut terminals = self.terminal_sessions.write()
            .map_err(|_| CoreError::SubsystemFailure("Terminal lock unavailable".into()))?;
        let terminal_pids = terminals.values_mut().filter(|session| session.belongs_to_environment(id))
            .filter_map(|session| session.live_pid()).collect::<Vec<_>>();
        let pids = manager.supervisor().list_processes().into_iter().filter(|process| process.running)
            .map(|process| process.pid).chain(terminal_pids.iter().copied())
            .collect::<std::collections::HashSet<_>>();
        let (memory_rss_bytes, cpu_time_ms) = measure(&pids);
        let started = manager.config().last_started_at_epoch_ms;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .ok().map(|duration| duration.as_millis() as u64);
        Ok(EnvironmentMetrics {
            processes: pids.len(), terminal_sessions: terminal_pids.len(), memory_rss_bytes, cpu_time_ms,
            seconds_since_launch: if pids.is_empty() { None } else { now.zip(started).map(|(now, start)| now.saturating_sub(start) / 1000) },
        })
    }
}

#[cfg(unix)]
fn measure(pids: &std::collections::HashSet<u32>) -> (Option<u64>, Option<u64>) {
    if pids.is_empty() { return (None, None); }
    let rss = pids.iter().try_fold(0u64, |sum, pid| {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let kib = status.lines().find_map(|line| line.strip_prefix("VmRSS:"))?
            .split_whitespace().next()?.parse::<u64>().ok()?;
        Some(sum.saturating_add(kib.saturating_mul(1024)))
    });
    let ticks = pids.iter().try_fold(0u64, |sum, pid| {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let mut fields = stat.rsplit_once(')')?.1.split_whitespace();
        let user_ticks = fields.nth(11)?.parse::<u64>().ok()?;
        let system_ticks = fields.next()?.parse::<u64>().ok()?;
        Some(sum.saturating_add(user_ticks).saturating_add(system_ticks))
    });
    let frequency = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    let cpu = if frequency > 0 { ticks.map(|ticks| ticks.saturating_mul(1000) / frequency as u64) } else { None };
    (rss, cpu)
}

#[cfg(not(unix))]
fn measure(_: &std::collections::HashSet<u32>) -> (Option<u64>, Option<u64>) {
    (None, None)
}

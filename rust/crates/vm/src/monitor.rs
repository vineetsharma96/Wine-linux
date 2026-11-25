use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::{json, Value};

use crate::error::VmError;

/// JSON QMP (QEMU Machine Protocol) client communicating strictly over `127.0.0.1:<qmp_port>`.
pub struct QmpClient;

impl QmpClient {
    /// Executes a single QMP command after negotiating `qmp_capabilities`.
    pub fn execute(qmp_port: u16, command: &str, arguments: Option<Value>) -> Result<Value, VmError> {
        let addr: SocketAddr = ([127, 0, 0, 1], qmp_port).into();
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(750))
            .map_err(|e| VmError::Monitor(format!("QMP connect 127.0.0.1:{qmp_port} failed: {e}")))?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(1200)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(750)));

        let mut reader = BufReader::new(
            stream
                .try_clone()
                .map_err(|e| VmError::Monitor(format!("QMP stream clone failed: {e}")))?,
        );

        // 1. Read greeting banner
        let mut line = String::new();
        let _ = reader.read_line(&mut line);

        // 2. Negotiate capabilities
        stream
            .write_all(b"{\"execute\":\"qmp_capabilities\"}\n")
            .map_err(|e| VmError::Monitor(format!("QMP capabilities write failed: {e}")))?;
        stream.flush()?;
        line.clear();
        let _ = reader.read_line(&mut line);

        // 3. Dispatch command
        let payload = Self::format_command(command, arguments);
        stream
            .write_all(format!("{payload}\n").as_bytes())
            .map_err(|e| VmError::Monitor(format!("QMP command '{command}' write failed: {e}")))?;
        stream.flush()?;

        // 4. Read response (skipping asynchronous QMP event lines if any arrive first)
        for _ in 0..8 {
            line.clear();
            let n = reader.read_line(&mut line).unwrap_or(0);
            if n == 0 {
                break;
            }
            if let Ok(val) = serde_json::from_str::<Value>(line.trim()) {
                if val.get("event").is_some() {
                    continue;
                }
                if let Some(err) = val.get("error") {
                    return Err(VmError::Monitor(format!("QMP '{command}' error: {err}")));
                }
                return Ok(val);
            }
        }

        Err(VmError::Monitor(format!(
            "No QMP response received for '{command}' on 127.0.0.1:{qmp_port}"
        )))
    }

    pub fn format_command(command: &str, arguments: Option<Value>) -> String {
        match arguments {
            Some(args) => json!({ "execute": command, "arguments": args }).to_string(),
            None => json!({ "execute": command }).to_string(),
        }
    }

    pub fn query_status(qmp_port: u16) -> Result<String, VmError> {
        let val = Self::execute(qmp_port, "query-status", None)?;
        Ok(val.to_string())
    }

    pub fn pause(qmp_port: u16) -> Result<(), VmError> {
        Self::execute(qmp_port, "stop", None)?;
        Ok(())
    }

    pub fn resume(qmp_port: u16) -> Result<(), VmError> {
        Self::execute(qmp_port, "cont", None)?;
        Ok(())
    }

    pub fn system_powerdown(qmp_port: u16) -> Result<(), VmError> {
        Self::execute(qmp_port, "system_powerdown", None)?;
        Ok(())
    }

    pub fn system_reset(qmp_port: u16) -> Result<(), VmError> {
        Self::execute(qmp_port, "system_reset", None)?;
        Ok(())
    }

    pub fn quit(qmp_port: u16) -> Result<(), VmError> {
        Self::execute(qmp_port, "quit", None)?;
        Ok(())
    }

    pub fn human_monitor_command(qmp_port: u16, command_line: &str) -> Result<String, VmError> {
        let val = Self::execute(
            qmp_port,
            "human-monitor-command",
            Some(json!({ "command-line": command_line })),
        )?;
        Ok(val
            .get("return")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }
}

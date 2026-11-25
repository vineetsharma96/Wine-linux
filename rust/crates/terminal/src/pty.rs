use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

use androidpc_runtime_api::{RuntimeError, TerminalSession};

pub struct DefaultTerminalSession {
    session_id: String,
    cols: u16,
    rows: u16,
    is_closed: Arc<AtomicBool>,
    #[cfg(unix)]
    master_fd: i32,
    #[cfg(unix)]
    child_pid: Option<libc::pid_t>,
    #[cfg(not(unix))]
    mock_buffer: Vec<u8>,
}

impl DefaultTerminalSession {
    pub fn new(session_id: impl Into<String>, cols: u16, rows: u16) -> Result<Self, RuntimeError> {
        let id = session_id.into();

        #[cfg(unix)]
        {
            let fd = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC) };
            if fd < 0 {
                return Err(RuntimeError::TerminalError(format!(
                    "posix_openpt failed: {}",
                    std::io::Error::last_os_error()
                )));
            }

            if unsafe { libc::grantpt(fd) } != 0 {
                unsafe { libc::close(fd) };
                return Err(RuntimeError::TerminalError("grantpt failed".to_string()));
            }

            if unsafe { libc::unlockpt(fd) } != 0 {
                unsafe { libc::close(fd) };
                return Err(RuntimeError::TerminalError("unlockpt failed".to_string()));
            }

            let mut session = Self {
                session_id: id,
                cols,
                rows,
                is_closed: Arc::new(AtomicBool::new(false)),
                master_fd: fd,
                child_pid: None,
            };

            session.resize(cols, rows)?;
            info!("Unix PTY session allocated successfully (master_fd: {})", fd);
            Ok(session)
        }

        #[cfg(not(unix))]
        {
            info!("Host mock PTY session allocated for '{}'", id);
            Ok(Self {
                session_id: id,
                cols,
                rows,
                is_closed: Arc::new(AtomicBool::new(false)),
                mock_buffer: Vec::new(),
            })
        }
    }

    #[cfg(unix)]
    pub fn master_fd(&self) -> i32 {
        self.master_fd
    }

    #[cfg(not(unix))]
    pub fn master_fd(&self) -> i32 {
        -1
    }

    #[cfg(unix)]
    pub fn pts_name(&self) -> Result<String, RuntimeError> {
        let mut buf = [0u8; 128];
        let ret = unsafe { libc::ptsname_r(self.master_fd, buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
        if ret != 0 {
            return Err(RuntimeError::TerminalError(format!(
                "ptsname_r failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        let cstr = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const libc::c_char) };
        Ok(cstr.to_string_lossy().to_string())
    }

    #[cfg(not(unix))]
    pub fn pts_name(&self) -> Result<String, RuntimeError> {
        Ok(format!("/dev/pts/mock_{}", self.session_id))
    }

    /// Spawns an interactive shell or PRoot command attached to this PTY's slave device.
    #[cfg(unix)]
    pub fn spawn_shell(
        &mut self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
        cwd: Option<&str>,
    ) -> Result<u32, RuntimeError> {
        use std::ffi::CString;

        let pts = self.pts_name()?;
        let pts_c = CString::new(pts).map_err(|_| RuntimeError::TerminalError("Invalid pts path".into()))?;
        let prog_c = CString::new(program).map_err(|_| RuntimeError::TerminalError("Invalid program".into()))?;

        let mut argv_c = Vec::with_capacity(args.len() + 1);
        argv_c.push(prog_c.clone());
        for arg in args {
            argv_c.push(
                CString::new(arg.as_str())
                    .map_err(|_| RuntimeError::TerminalError("Invalid arg".into()))?,
            );
        }
        let mut argv_ptrs: Vec<*const libc::c_char> = argv_c.iter().map(|s| s.as_ptr()).collect();
        argv_ptrs.push(std::ptr::null());

        let mut env_c = Vec::with_capacity(env.len());
        for (k, v) in env {
            if let Ok(entry) = CString::new(format!("{k}={v}")) {
                env_c.push(entry);
            }
        }
        let cwd_c = cwd.and_then(|c| CString::new(c).ok());

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(RuntimeError::Io(std::io::Error::last_os_error()));
        }

        if pid == 0 {
            unsafe {
                libc::setsid();
                let slave_fd = libc::open(pts_c.as_ptr(), libc::O_RDWR);
                if slave_fd < 0 {
                    libc::_exit(126);
                }
                let _ = libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0);
                libc::dup2(slave_fd, libc::STDIN_FILENO);
                libc::dup2(slave_fd, libc::STDOUT_FILENO);
                libc::dup2(slave_fd, libc::STDERR_FILENO);
                if slave_fd > libc::STDERR_FILENO {
                    libc::close(slave_fd);
                }
                if self.master_fd >= 0 {
                    libc::close(self.master_fd);
                }
                if let Some(ref dir) = cwd_c {
                    libc::chdir(dir.as_ptr());
                }
                for kv in &env_c {
                    // putenv takes ownership of a persistent buffer; leak is safe in child pre-exec
                    let ptr = libc::strdup(kv.as_ptr());
                    if !ptr.is_null() {
                        libc::putenv(ptr);
                    }
                }
                libc::execvp(prog_c.as_ptr(), argv_ptrs.as_ptr());
                libc::_exit(127);
            }
        }

        self.child_pid = Some(pid);
        info!("Spawned interactive PTY child '{}' with PID {}", program, pid);
        Ok(pid as u32)
    }

    #[cfg(not(unix))]
    pub fn spawn_shell(
        &mut self,
        program: &str,
        _args: &[String],
        _env: &[(String, String)],
        _cwd: Option<&str>,
    ) -> Result<u32, RuntimeError> {
        self.mock_buffer
            .extend_from_slice(format!("[mock shell attached: {program}]\n").as_bytes());
        Ok(1000)
    }
}

impl TerminalSession for DefaultTerminalSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), RuntimeError> {
        self.cols = cols;
        self.rows = rows;

        #[cfg(unix)]
        {
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let ret = unsafe { libc::ioctl(self.master_fd, libc::TIOCSWINSZ as _, &ws) };
            if ret != 0 {
                return Err(RuntimeError::TerminalError(format!(
                    "ioctl TIOCSWINSZ failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }

        Ok(())
    }

    fn write_input(&mut self, data: &[u8]) -> Result<usize, RuntimeError> {
        if self.is_closed.load(Ordering::SeqCst) {
            return Err(RuntimeError::TerminalError("Session is closed".to_string()));
        }

        #[cfg(unix)]
        {
            let n = unsafe {
                libc::write(
                    self.master_fd,
                    data.as_ptr() as *const libc::c_void,
                    data.len(),
                )
            };
            if n < 0 {
                return Err(RuntimeError::Io(std::io::Error::last_os_error()));
            }
            Ok(n as usize)
        }

        #[cfg(not(unix))]
        {
            self.mock_buffer.extend_from_slice(data);
            Ok(data.len())
        }
    }

    fn read_output(&mut self, buf: &mut [u8]) -> Result<usize, RuntimeError> {
        if self.is_closed.load(Ordering::SeqCst) {
            return Ok(0);
        }

        #[cfg(unix)]
        {
            let mut pfd = libc::pollfd {
                fd: self.master_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut pfd, 1, 0) };
            if ready <= 0 {
                return Ok(0);
            }
            if (pfd.revents & libc::POLLIN) == 0 {
                return Ok(0);
            }
            let n = unsafe {
                libc::read(
                    self.master_fd,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EAGAIN) || err.raw_os_error() == Some(libc::EIO) {
                    return Ok(0);
                }
                return Err(RuntimeError::Io(err));
            }
            Ok(n as usize)
        }

        #[cfg(not(unix))]
        {
            let len = std::cmp::min(buf.len(), self.mock_buffer.len());
            buf[..len].copy_from_slice(&self.mock_buffer[..len]);
            self.mock_buffer.drain(..len);
            Ok(len)
        }
    }

    fn close(&mut self) -> Result<(), RuntimeError> {
        self.is_closed.store(true, Ordering::SeqCst);

        #[cfg(unix)]
        {
            if let Some(pid) = self.child_pid.take() {
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                    let mut status: libc::c_int = 0;
                    libc::waitpid(pid, &mut status, libc::WNOHANG);
                }
            }
            if self.master_fd >= 0 {
                unsafe { libc::close(self.master_fd) };
                self.master_fd = -1;
            }
        }

        Ok(())
    }
}

impl Drop for DefaultTerminalSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_session_lifecycle() {
        let mut term = DefaultTerminalSession::new("test-pty-1", 80, 24).unwrap();
        assert_eq!(term.session_id(), "test-pty-1");
        assert!(term.resize(120, 40).is_ok());

        let written = term.write_input(b"echo 'hello'\n").unwrap();
        assert_eq!(written, 13);

        let mut out = [0u8; 64];
        let read = term.read_output(&mut out).unwrap();
        assert_eq!(read, 13);
        assert_eq!(&out[..13], b"echo 'hello'\n");

        assert!(term.close().is_ok());
    }

    #[test]
    fn test_terminal_shell_spawn_non_unix_or_pty() {
        let mut term = DefaultTerminalSession::new("test-pty-shell", 80, 24).unwrap();
        let pid = term
            .spawn_shell("/bin/sh", &[], &[("TERM".into(), "xterm-256color".into())], None)
            .unwrap();
        assert!(pid > 0);
        assert!(term.close().is_ok());
    }
}


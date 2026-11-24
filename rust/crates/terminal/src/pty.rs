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
            let n = unsafe {
                libc::read(
                    self.master_fd,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };
            if n < 0 {
                return Err(RuntimeError::Io(std::io::Error::last_os_error()));
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
}


use std::collections::VecDeque;
use std::io::{ErrorKind, Read};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Audio sink IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Audio sink already running on port {0}")]
    AlreadyRunning(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStatusInfo {
    pub running: bool,
    pub listen_address: String,
    pub port: u16,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub active_clients: u64,
    pub buffered_bytes: usize,
    pub total_bytes_received: u64,
}

const MAX_RING_BYTES: usize = 48000 * 4 * 2; // ~2 seconds of 48kHz stereo S16LE

pub struct AudioTcpSink {
    listen_address: String,
    port: u16,
    sample_rate_hz: u32,
    channels: u16,
    running: Arc<AtomicBool>,
    active_clients: Arc<AtomicU64>,
    total_bytes_received: Arc<AtomicU64>,
    ring: Arc<Mutex<VecDeque<u8>>>,
    accept_thread: Option<JoinHandle<()>>,
}

impl Default for AudioTcpSink {
    fn default() -> Self {
        Self::new("127.0.0.1", 4713, 44100, 2)
    }
}

impl AudioTcpSink {
    pub fn new(
        listen_address: impl Into<String>,
        port: u16,
        sample_rate_hz: u32,
        channels: u16,
    ) -> Self {
        Self {
            listen_address: listen_address.into(),
            port,
            sample_rate_hz,
            channels,
            running: Arc::new(AtomicBool::new(false)),
            active_clients: Arc::new(AtomicU64::new(0)),
            total_bytes_received: Arc::new(AtomicU64::new(0)),
            ring: Arc::new(Mutex::new(VecDeque::with_capacity(65536))),
            accept_thread: None,
        }
    }

    pub fn start(&mut self) -> Result<u16, AudioError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(self.port);
        }

        let listener = TcpListener::bind((self.listen_address.as_str(), self.port))?;
        let bound_port = listener.local_addr()?.port();
        self.port = bound_port;
        listener.set_nonblocking(true)?;

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let ring = self.ring.clone();
        let clients = self.active_clients.clone();
        let total = self.total_bytes_received.clone();

        let handle = thread::spawn(move || {
            info!("PulseAudio/PCM TCP audio sink listening on port {}", bound_port);
            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, addr)) => {
                        info!("Audio client connected from {}", addr);
                        clients.fetch_add(1, Ordering::SeqCst);
                        let c_running = running.clone();
                        let c_ring = ring.clone();
                        let c_clients = clients.clone();
                        let c_total = total.clone();
                        thread::spawn(move || {
                            Self::handle_client(stream, c_running, c_ring, c_total);
                            c_clients.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(e) => {
                        warn!("Audio sink accept error: {}", e);
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
            info!("Audio TCP sink listener stopped");
        });

        self.accept_thread = Some(handle);
        Ok(bound_port)
    }

    fn handle_client(
        mut stream: TcpStream,
        running: Arc<AtomicBool>,
        ring: Arc<Mutex<VecDeque<u8>>>,
        total: Arc<AtomicU64>,
    ) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let mut buf = [0u8; 4096];

        while running.load(Ordering::SeqCst) {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    total.fetch_add(n as u64, Ordering::SeqCst);
                    if let Ok(mut guard) = ring.lock() {
                        if guard.len() + n > MAX_RING_BYTES {
                            let drop_count = (guard.len() + n) - MAX_RING_BYTES;
                            let to_drop = drop_count.min(guard.len());
                            guard.drain(..to_drop);
                        }
                        guard.extend(&buf[..n]);
                    }
                }
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
                {
                    continue;
                }
                Err(_) => break,
            }
        }
    }

    pub fn read_samples(&self, out: &mut [u8]) -> usize {
        if let Ok(mut guard) = self.ring.lock() {
            let count = out.len().min(guard.len());
            for (i, byte) in guard.drain(..count).enumerate() {
                out[i] = byte;
            }
            count
        } else {
            0
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.accept_thread.take() {
            let _ = handle.join();
        }
    }

    pub fn status(&self) -> AudioStatusInfo {
        let buffered_bytes = self.ring.lock().map(|g| g.len()).unwrap_or(0);
        AudioStatusInfo {
            running: self.running.load(Ordering::SeqCst),
            listen_address: self.listen_address.clone(),
            port: self.port,
            sample_rate_hz: self.sample_rate_hz,
            channels: self.channels,
            active_clients: self.active_clients.load(Ordering::SeqCst),
            buffered_bytes,
            total_bytes_received: self.total_bytes_received.load(Ordering::SeqCst),
        }
    }
}

impl Drop for AudioTcpSink {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_audio_tcp_sink_streaming() {
        let mut sink = AudioTcpSink::new("127.0.0.1", 0, 44100, 2);
        let port = sink.start().expect("start audio sink");
        assert!(sink.status().running);

        let mut client = TcpStream::connect(("127.0.0.1", port)).expect("connect audio client");
        let payload = [0x12u8, 0x34u8, 0x56u8, 0x78u8, 0x9au8, 0xbcu8, 0xdeu8, 0xf0u8];
        client.write_all(&payload).unwrap();
        client.flush().unwrap();
        drop(client);

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while sink.status().buffered_bytes < payload.len() && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }

        let mut out = [0u8; 16];
        let read = sink.read_samples(&mut out);
        assert_eq!(read, payload.len());
        assert_eq!(&out[..read], &payload);

        sink.stop();
        assert!(!sink.status().running);
    }
}

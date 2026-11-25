use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use tracing::info;

use crate::rfb::{
    ClientMessageType, FramebufferUpdateRequest, KeyEvent, PixelFormat, PointerEvent,
    RfbSecurityType, RFB_VERSION_3_8,
};

#[derive(Debug, thiserror::Error)]
pub enum DisplayError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("RFB protocol violation: {0}")]
    Protocol(String),

    #[error("RFB authentication failed: {0}")]
    AuthFailed(String),

    #[error("Client is disconnected")]
    Disconnected,
}

pub struct RfbClient {
    stream: Option<TcpStream>,
    pub width: u16,
    pub height: u16,
    pub pixel_format: PixelFormat,
    pub server_name: String,
    pub framebuffer: Vec<u8>, // RGBA 32-bit (width * height * 4)
    pub is_connected: bool,
}

impl RfbClient {
    pub fn new_offline(width: u16, height: u16) -> Self {
        let mut client = Self {
            stream: None,
            width,
            height,
            pixel_format: PixelFormat::default(),
            server_name: "Offline Simulated Display".to_string(),
            framebuffer: vec![0u8; (width as usize) * (height as usize) * 4],
            is_connected: false,
        };
        client.generate_test_card();
        client
    }

    pub fn connect(address: &str, port: u16) -> Result<Self, DisplayError> {
        info!("Connecting RFB client to {}:{}", address, port);
        let mut stream = TcpStream::connect((address, port))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;

        // 1. Handshake: Read Server Version (12 bytes)
        let mut server_version = [0u8; 12];
        stream.read_exact(&mut server_version)?;
        if !server_version.starts_with(b"RFB ") {
            return Err(DisplayError::Protocol("Invalid RFB server banner".to_string()));
        }

        // Send Client Version (12 bytes)
        stream.write_all(RFB_VERSION_3_8)?;
        stream.flush()?;

        // 2. Security Handshake
        let mut num_security_types = [0u8; 1];
        stream.read_exact(&mut num_security_types)?;
        if num_security_types[0] == 0 {
            return Err(DisplayError::AuthFailed("Server reported 0 security types".to_string()));
        }

        let mut sec_types = vec![0u8; num_security_types[0] as usize];
        stream.read_exact(&mut sec_types)?;

        if !sec_types.contains(&(RfbSecurityType::None as u8)) {
            return Err(DisplayError::AuthFailed("Server does not permit 'None' security".to_string()));
        }

        stream.write_all(&[RfbSecurityType::None as u8])?;
        stream.flush()?;

        // Read SecurityResult (4 bytes BE: 0 = OK)
        let mut sec_result = [0u8; 4];
        stream.read_exact(&mut sec_result)?;
        let result_code = u32::from_be_bytes(sec_result);
        if result_code != 0 {
            return Err(DisplayError::AuthFailed(format!(
                "SecurityResult returned error code {}",
                result_code
            )));
        }

        // 3. ClientInit: Shared flag = 1
        stream.write_all(&[1u8])?;
        stream.flush()?;

        // 4. ServerInit: width(2), height(2), pixel_format(16), name_len(4)
        let mut init_header = [0u8; 20];
        stream.read_exact(&mut init_header)?;

        let width = u16::from_be_bytes([init_header[0], init_header[1]]);
        let height = u16::from_be_bytes([init_header[2], init_header[3]]);

        let pixel_format = PixelFormat {
            bits_per_pixel: init_header[4],
            depth: init_header[5],
            big_endian_flag: init_header[6],
            true_color_flag: init_header[7],
            red_max: u16::from_be_bytes([init_header[8], init_header[9]]),
            green_max: u16::from_be_bytes([init_header[10], init_header[11]]),
            blue_max: u16::from_be_bytes([init_header[12], init_header[13]]),
            red_shift: init_header[14],
            green_shift: init_header[15],
            blue_shift: init_header[16],
        };

        let mut actual_name_len = [0u8; 4];
        stream.read_exact(&mut actual_name_len)?;
        let name_len = u32::from_be_bytes(actual_name_len);

        let mut name_buf = vec![0u8; (name_len.min(256)) as usize];
        stream.read_exact(&mut name_buf)?;
        let server_name = String::from_utf8_lossy(&name_buf).to_string();

        info!(
            "RFB session established: '{}' ({}x{}, {} bpp)",
            server_name, width, height, pixel_format.bits_per_pixel
        );

        // 5. Send SetPixelFormat to request standard 32-bit RGBA/BGRA true color if needed,
        // and SetEncodings: Raw (0), CopyRect (1), DesktopSize (-223)
        let set_encodings_msg: [u8; 16] = [
            ClientMessageType::SetEncodings as u8,
            0,
            0,
            3, // 3 encodings
            0, 0, 0, 0, // Raw = 0
            0, 0, 0, 1, // CopyRect = 1
            0xff, 0xff, 0xff, 0x21, // DesktopSize = -223 (0xFFFFFF21)
        ];
        stream.write_all(&set_encodings_msg)?;
        stream.flush()?;

        // Short timeout for non-blocking-style frame polling
        stream.set_read_timeout(Some(Duration::from_millis(8)))?;

        let buffer_size = (width as usize) * (height as usize) * 4;
        let mut client = Self {
            stream: Some(stream),
            width,
            height,
            pixel_format,
            server_name,
            framebuffer: vec![0u8; buffer_size],
            is_connected: true,
        };

        // Request initial full framebuffer
        let _ = client.request_frame(false);
        Ok(client)
    }

    /// Polls any pending RFB Server-to-Client messages and updates the local RGBA framebuffer.
    /// Returns the number of rectangles decoded in this poll cycle.
    pub fn poll_updates(&mut self) -> Result<usize, DisplayError> {
        let mut stream = match self.stream.take() {
            Some(s) => s,
            None => return Ok(0),
        };

        let mut rects_decoded = 0usize;
        let result = self.poll_updates_inner(&mut stream, &mut rects_decoded);
        self.stream = Some(stream);

        match result {
            Ok(()) => Ok(rects_decoded),
            Err(DisplayError::Io(e))
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
            {
                Ok(rects_decoded)
            }
            Err(e) => Err(e),
        }
    }

    fn poll_updates_inner(
        &mut self,
        stream: &mut TcpStream,
        rects_decoded: &mut usize,
    ) -> Result<(), DisplayError> {
        loop {
            let mut msg_type = [0u8; 1];
            match stream.read_exact(&mut msg_type) {
                Ok(()) => {}
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
                {
                    return Ok(());
                }
                Err(e) => return Err(DisplayError::Io(e)),
            }

            // Temporarily raise timeout while reading the rest of an in-flight message
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let res = match msg_type[0] {
                0 => self.handle_framebuffer_update(stream, rects_decoded),
                1 => Self::skip_colour_map_entries(stream),
                2 => Ok(()), // Bell (1 byte total)
                3 => Self::skip_server_cut_text(stream),
                other => Err(DisplayError::Protocol(format!(
                    "Unsupported RFB server message type: {other}"
                ))),
            };
            let _ = stream.set_read_timeout(Some(Duration::from_millis(8)));
            res?;
        }
    }

    fn handle_framebuffer_update(
        &mut self,
        stream: &mut TcpStream,
        rects_decoded: &mut usize,
    ) -> Result<(), DisplayError> {
        let mut header = [0u8; 3]; // 1 byte padding + 2 bytes number-of-rectangles
        stream.read_exact(&mut header)?;
        let num_rects = u16::from_be_bytes([header[1], header[2]]);

        let bpp_bytes = (self.pixel_format.bits_per_pixel / 8).max(1) as usize;

        for _ in 0..num_rects {
            let mut rect_hdr = [0u8; 12];
            stream.read_exact(&mut rect_hdr)?;
            let rx = u16::from_be_bytes([rect_hdr[0], rect_hdr[1]]);
            let ry = u16::from_be_bytes([rect_hdr[2], rect_hdr[3]]);
            let rw = u16::from_be_bytes([rect_hdr[4], rect_hdr[5]]);
            let rh = u16::from_be_bytes([rect_hdr[6], rect_hdr[7]]);
            let encoding = i32::from_be_bytes([rect_hdr[8], rect_hdr[9], rect_hdr[10], rect_hdr[11]]);

            match encoding {
                0 => {
                    // Raw encoding
                    let byte_len = (rw as usize) * (rh as usize) * bpp_bytes;
                    let mut raw_buf = vec![0u8; byte_len];
                    stream.read_exact(&mut raw_buf)?;
                    self.decode_raw_rect(rx, ry, rw, rh, &raw_buf);
                    *rects_decoded += 1;
                }
                1 => {
                    // CopyRect encoding: 2 bytes src_x, 2 bytes src_y
                    let mut src = [0u8; 4];
                    stream.read_exact(&mut src)?;
                    let sx = u16::from_be_bytes([src[0], src[1]]);
                    let sy = u16::from_be_bytes([src[2], src[3]]);
                    self.copy_rect(sx, sy, rx, ry, rw, rh);
                    *rects_decoded += 1;
                }
                -223 => {
                    // DesktopSize pseudo-encoding
                    if rw > 0 && rh > 0 {
                        self.width = rw;
                        self.height = rh;
                        self.framebuffer
                            .resize((rw as usize) * (rh as usize) * 4, 0xff);
                    }
                }
                other => {
                    return Err(DisplayError::Protocol(format!(
                        "Unsupported RFB rectangle encoding: {other}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn skip_colour_map_entries(stream: &mut TcpStream) -> Result<(), DisplayError> {
        let mut hdr = [0u8; 5]; // padding(1), first-colour(2), num-colours(2)
        stream.read_exact(&mut hdr)?;
        let num = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
        let mut discard = vec![0u8; num * 6];
        stream.read_exact(&mut discard)?;
        Ok(())
    }

    fn skip_server_cut_text(stream: &mut TcpStream) -> Result<(), DisplayError> {
        let mut hdr = [0u8; 7]; // padding(3), length(4)
        stream.read_exact(&mut hdr)?;
        let len = u32::from_be_bytes([hdr[3], hdr[4], hdr[5], hdr[6]]) as usize;
        let mut discard = vec![0u8; len.min(1024 * 1024)];
        stream.read_exact(&mut discard)?;
        Ok(())
    }

    fn decode_raw_rect(&mut self, rx: u16, ry: u16, rw: u16, rh: u16, raw: &[u8]) {
        let bpp = self.pixel_format.bits_per_pixel;
        let bytes_per_pixel = (bpp / 8).max(1) as usize;
        let mut rgba = Vec::with_capacity((rw as usize) * (rh as usize) * 4);

        for chunk in raw.chunks_exact(bytes_per_pixel) {
            let pixel: u32 = match bytes_per_pixel {
                4 => {
                    if self.pixel_format.big_endian_flag != 0 {
                        u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                    } else {
                        u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                    }
                }
                2 => {
                    let v = if self.pixel_format.big_endian_flag != 0 {
                        u16::from_be_bytes([chunk[0], chunk[1]])
                    } else {
                        u16::from_le_bytes([chunk[0], chunk[1]])
                    };
                    v as u32
                }
                _ => chunk[0] as u32,
            };

            let r_raw = (pixel >> self.pixel_format.red_shift) & (self.pixel_format.red_max as u32);
            let g_raw =
                (pixel >> self.pixel_format.green_shift) & (self.pixel_format.green_max as u32);
            let b_raw =
                (pixel >> self.pixel_format.blue_shift) & (self.pixel_format.blue_max as u32);

            let r = if self.pixel_format.red_max > 0 {
                ((r_raw * 255) / self.pixel_format.red_max as u32) as u8
            } else {
                0
            };
            let g = if self.pixel_format.green_max > 0 {
                ((g_raw * 255) / self.pixel_format.green_max as u32) as u8
            } else {
                0
            };
            let b = if self.pixel_format.blue_max > 0 {
                ((b_raw * 255) / self.pixel_format.blue_max as u32) as u8
            } else {
                0
            };

            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(0xff);
        }

        self.blit_rect(rx, ry, rw, rh, &rgba);
    }

    fn copy_rect(&mut self, sx: u16, sy: u16, dx: u16, dy: u16, w: u16, h: u16) {
        let stride = (self.width as usize) * 4;
        let mut temp = vec![0u8; (w as usize) * (h as usize) * 4];
        for row in 0..h {
            let y_src = (sy + row) as usize;
            if y_src >= self.height as usize {
                break;
            }
            let src_start = y_src * stride + (sx as usize) * 4;
            let src_end = src_start + (w as usize) * 4;
            let dst_start = (row as usize) * (w as usize) * 4;
            let dst_end = dst_start + (w as usize) * 4;
            if src_end <= self.framebuffer.len() && dst_end <= temp.len() {
                temp[dst_start..dst_end].copy_from_slice(&self.framebuffer[src_start..src_end]);
            }
        }
        self.blit_rect(dx, dy, w, h, &temp);
    }

    pub fn blit_rect(&mut self, rx: u16, ry: u16, rw: u16, rh: u16, rgba_data: &[u8]) {
        let stride = (self.width as usize) * 4;
        for row in 0..rh {
            let y_coord = (ry + row) as usize;
            if y_coord >= self.height as usize {
                break;
            }

            let target_start = y_coord * stride + (rx as usize) * 4;
            let target_end = target_start + (rw as usize) * 4;

            let source_start = (row as usize) * (rw as usize) * 4;
            let source_end = source_start + (rw as usize) * 4;

            if target_end <= self.framebuffer.len() && source_end <= rgba_data.len() {
                self.framebuffer[target_start..target_end]
                    .copy_from_slice(&rgba_data[source_start..source_end]);
            }
        }
    }

    pub fn generate_test_card(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * 4;
                if idx + 3 < self.framebuffer.len() {
                    if y < 40 {
                        // Top bar (Dark Slate Gray)
                        self.framebuffer[idx] = 0x1e; // R
                        self.framebuffer[idx + 1] = 0x22; // G
                        self.framebuffer[idx + 2] = 0x2d; // B
                        self.framebuffer[idx + 3] = 0xff; // A
                    } else if y >= h - 48 {
                        // Bottom XFCE panel
                        self.framebuffer[idx] = 0x0f;
                        self.framebuffer[idx + 1] = 0x17;
                        self.framebuffer[idx + 2] = 0x2a;
                        self.framebuffer[idx + 3] = 0xff;
                    } else {
                        // Desktop workspace gradient
                        let grad = ((x * 32) / w.max(1)) as u8;
                        self.framebuffer[idx] = 0x18 + (grad / 4);
                        self.framebuffer[idx + 1] = 0x28 + (grad / 2);
                        self.framebuffer[idx + 2] = 0x3f + (grad / 2);
                        self.framebuffer[idx + 3] = 0xff;
                    }
                }
            }
        }
    }

    pub fn send_pointer(&mut self, button_mask: u8, x: u16, y: u16) -> Result<(), DisplayError> {
        let ptr = PointerEvent { button_mask, x, y };
        if let Some(ref mut s) = self.stream {
            s.write_all(&ptr.to_bytes())?;
            s.flush()?;
            Ok(())
        } else {
            Ok(()) // Offline mode accepts inputs gracefully
        }
    }

    pub fn send_key(&mut self, down_flag: u8, key: u32) -> Result<(), DisplayError> {
        let k = KeyEvent { down_flag, key };
        if let Some(ref mut s) = self.stream {
            s.write_all(&k.to_bytes())?;
            s.flush()?;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn request_frame(&mut self, incremental: bool) -> Result<(), DisplayError> {
        let req = FramebufferUpdateRequest {
            incremental: if incremental { 1 } else { 0 },
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        };
        if let Some(ref mut s) = self.stream {
            s.write_all(&req.to_bytes())?;
            s.flush()?;
        }
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(s) = self.stream.take() {
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
        self.is_connected = false;
    }
}

impl Drop for RfbClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_live_rfb_handshake_and_raw_rect_decode() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock rfb");
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            sock.write_all(RFB_VERSION_3_8).unwrap();
            let mut client_ver = [0u8; 12];
            sock.read_exact(&mut client_ver).unwrap();

            // Security types: 1 type = None (1)
            sock.write_all(&[1, 1]).unwrap();
            let mut chosen = [0u8; 1];
            sock.read_exact(&mut chosen).unwrap();
            // SecurityResult OK
            sock.write_all(&0u32.to_be_bytes()).unwrap();

            // ClientInit
            let mut shared = [0u8; 1];
            sock.read_exact(&mut shared).unwrap();

            // ServerInit: 4x4, default 32bpp little-endian BGRX (red_shift=16, green_shift=8, blue_shift=0)
            let mut init = Vec::new();
            init.extend_from_slice(&4u16.to_be_bytes());
            init.extend_from_slice(&4u16.to_be_bytes());
            init.push(32); // bpp
            init.push(24); // depth
            init.push(0); // little-endian
            init.push(1); // true color
            init.extend_from_slice(&255u16.to_be_bytes());
            init.extend_from_slice(&255u16.to_be_bytes());
            init.extend_from_slice(&255u16.to_be_bytes());
            init.push(16); // red_shift
            init.push(8); // green_shift
            init.push(0); // blue_shift
            init.extend_from_slice(&[0, 0, 0]); // padding
            init.extend_from_slice(&4u32.to_be_bytes());
            init.extend_from_slice(b"Test");
            sock.write_all(&init).unwrap();

            // Read SetEncodings (16 bytes) and initial FramebufferUpdateRequest (10 bytes)
            let mut buf = [0u8; 26];
            sock.read_exact(&mut buf).unwrap();

            // Send FramebufferUpdate with 1 Raw rect (1x1 at (0,0), pixel BGRX = 0x00112233 -> R=0x11, G=0x22, B=0x33)
            let mut update = vec![0u8, 0u8]; // type=0, padding=0
            update.extend_from_slice(&1u16.to_be_bytes()); // 1 rect
            update.extend_from_slice(&0u16.to_be_bytes()); // x
            update.extend_from_slice(&0u16.to_be_bytes()); // y
            update.extend_from_slice(&1u16.to_be_bytes()); // w
            update.extend_from_slice(&1u16.to_be_bytes()); // h
            update.extend_from_slice(&0i32.to_be_bytes()); // Raw encoding
            // Little-endian 32-bit pixel with R=0x11 (shift 16), G=0x22 (shift 8), B=0x33 (shift 0): 0x00112233
            update.extend_from_slice(&0x0011_2233u32.to_le_bytes());
            sock.write_all(&update).unwrap();
            sock.flush().unwrap();
            thread::sleep(Duration::from_millis(50));
        });

        let mut client = RfbClient::connect("127.0.0.1", port).expect("connect mock rfb");
        assert_eq!(client.width, 4);
        assert_eq!(client.height, 4);
        assert_eq!(client.server_name, "Test");

        thread::sleep(Duration::from_millis(20));
        let decoded = client.poll_updates().expect("poll updates");
        assert_eq!(decoded, 1);
        assert_eq!(&client.framebuffer[0..4], &[0x11, 0x22, 0x33, 0xff]);
        server.join().unwrap();
    }
}

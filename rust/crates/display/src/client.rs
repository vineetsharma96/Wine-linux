use std::io::{Read, Write};
use std::net::TcpStream;
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

        // Find None (type 1)
        if !sec_types.contains(&(RfbSecurityType::None as u8)) {
            return Err(DisplayError::AuthFailed("Server does not permit 'None' security".to_string()));
        }

        // Choose None
        stream.write_all(&[RfbSecurityType::None as u8])?;
        stream.flush()?;

        // Read SecurityResult (4 bytes BE: 0 = OK)
        let mut sec_result = [0u8; 4];
        stream.read_exact(&mut sec_result)?;
        let result_code = u32::from_be_bytes(sec_result);
        if result_code != 0 {
            return Err(DisplayError::AuthFailed(format!("SecurityResult returned error code {}", result_code)));
        }

        // 3. ClientInit: Shared flag = 1 (do not disconnect other clients)
        stream.write_all(&[1u8])?;
        stream.flush()?;

        // 4. ServerInit:
        //   fb_width: u16
        //   fb_height: u16
        //   pixel_format: 16 bytes
        //   name_len: u32
        //   name_string: [u8; name_len]
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

        let mut name_len_bytes = [0u8; 4];
        name_len_bytes.copy_from_slice(&init_header[16..20]);
        // Actually name_length is 4 bytes starting at offset 20
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

        // 5. Send SetEncodings: Request Raw encoding (0)
        let set_encodings_msg = [
            ClientMessageType::SetEncodings as u8,
            0, // padding
            0, 1, // number-of-encodings = 1
            0, 0, 0, 0, // Raw encoding = 0
        ];
        stream.write_all(&set_encodings_msg)?;
        stream.flush()?;

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

        client.generate_test_card();
        Ok(client)
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
                        self.framebuffer[idx] = 0x1e;     // R
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

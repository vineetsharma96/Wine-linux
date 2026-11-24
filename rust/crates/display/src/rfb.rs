use serde::{Deserialize, Serialize};

pub const RFB_VERSION_3_8: &[u8; 12] = b"RFB 003.008\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RfbSecurityType {
    Invalid = 0,
    None = 1,
    VncAuth = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ClientMessageType {
    SetPixelFormat = 0,
    SetEncodings = 2,
    FramebufferUpdateRequest = 3,
    KeyEvent = 4,
    PointerEvent = 5,
    ClientCutText = 6,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelFormat {
    pub bits_per_pixel: u8,
    pub depth: u8,
    pub big_endian_flag: u8,
    pub true_color_flag: u8,
    pub red_max: u16,
    pub green_max: u16,
    pub blue_max: u16,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

impl Default for PixelFormat {
    fn default() -> Self {
        Self {
            bits_per_pixel: 32,
            depth: 24,
            big_endian_flag: 0,
            true_color_flag: 1,
            red_max: 255,
            green_max: 255,
            blue_max: 255,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerEvent {
    pub button_mask: u8, // bit 0: left, bit 1: middle, bit 2: right, bit 3: scroll up, bit 4: scroll down
    pub x: u16,
    pub y: u16,
}

impl PointerEvent {
    pub fn to_bytes(&self) -> [u8; 6] {
        let x_bytes = self.x.to_be_bytes();
        let y_bytes = self.y.to_be_bytes();
        [
            ClientMessageType::PointerEvent as u8,
            self.button_mask,
            x_bytes[0],
            x_bytes[1],
            y_bytes[0],
            y_bytes[1],
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub down_flag: u8, // 1 for pressed, 0 for released
    pub key: u32,      // X11 keysym
}

impl KeyEvent {
    pub fn to_bytes(&self) -> [u8; 8] {
        let key_bytes = self.key.to_be_bytes();
        [
            ClientMessageType::KeyEvent as u8,
            self.down_flag,
            0,
            0, // padding
            key_bytes[0],
            key_bytes[1],
            key_bytes[2],
            key_bytes[3],
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramebufferUpdateRequest {
    pub incremental: u8, // 0 for full refresh, 1 for incremental
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl FramebufferUpdateRequest {
    pub fn to_bytes(&self) -> [u8; 10] {
        let x = self.x.to_be_bytes();
        let y = self.y.to_be_bytes();
        let w = self.width.to_be_bytes();
        let h = self.height.to_be_bytes();
        [
            ClientMessageType::FramebufferUpdateRequest as u8,
            self.incremental,
            x[0],
            x[1],
            y[0],
            y[1],
            w[0],
            w[1],
            h[0],
            h[1],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfb_handshake_constants() {
        assert_eq!(RFB_VERSION_3_8, b"RFB 003.008\n");
    }

    #[test]
    fn test_pointer_event_serialization() {
        let ptr = PointerEvent {
            button_mask: 0x01, // Left click
            x: 640,
            y: 480,
        };
        let bytes = ptr.to_bytes();
        assert_eq!(bytes[0], ClientMessageType::PointerEvent as u8);
        assert_eq!(bytes[1], 0x01);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 640);
        assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), 480);
    }

    #[test]
    fn test_key_event_serialization() {
        let key = KeyEvent {
            down_flag: 1,
            key: 0xff0d, // Return / Enter keysym
        };
        let bytes = key.to_bytes();
        assert_eq!(bytes[0], ClientMessageType::KeyEvent as u8);
        assert_eq!(bytes[1], 1);
        assert_eq!(u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 0xff0d);
    }
}

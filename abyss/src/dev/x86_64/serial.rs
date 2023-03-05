//! Serial device driver.
use crate::x86_64::pio::Pio;

/// Initialize a serial.
pub unsafe fn init() {
    Pio::new(0x3f8 + 2).write_u8(0);
    Pio::new(0x3f8 + 3).write_u8(0x80);
    Pio::new(0x3f8).write_u8((115200 / 9600) as u8);
    Pio::new(0x3f8 + 1).write_u8(0);
    Pio::new(0x3f8 + 3).write_u8(0x3 & !0x80);
    Pio::new(0x3f8 + 4).write_u8(0);
    Pio::new(0x3f8 + 1).write_u8(1);
    Pio::new(0x3f8 + 2).read_u8();
    Pio::new(0x3f8).read_u8();
}

pub(crate) fn write_str(s: &str) {
    for b in s.as_bytes() {
        for _ in 0..12800 {
            if Pio::new(0x3f8 + 5).read_u8() & 0x20 != 0 {
                break;
            }
            // delay
            Pio::new(0x84).read_u8();
            Pio::new(0x84).read_u8();
            Pio::new(0x84).read_u8();
            Pio::new(0x84).read_u8();
        }
        Pio::new(0x3f8).write_u8(*b);
    }
}

pub struct Serial {
    _p: (),
}

impl Serial {
    /// Create a new serial device interface.
    pub const fn new() -> Self {
        Serial { _p: () }
    }
}

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}

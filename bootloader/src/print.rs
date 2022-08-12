//! Print semantics

use crate::BOOT_KERN;

/// Dummy type to implement `Write` on
pub struct Serial;

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut serial = BOOT_KERN.serial.lock();
        if let Some(serial) = &mut *serial {
            serial.write(s.as_bytes());
        }
        Ok(())
    }
}

/// Serial `print!()` support
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _ = core::fmt::write(&mut $crate::print::Serial,
                                 format_args!($($arg)*));
    }}
}

/// Dummy type to implement `Write` on.
pub struct SerialShatter;

impl core::fmt::Write for SerialShatter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            let serial = BOOT_KERN.serial.shatter();
            if let Some(serial) = &mut *serial {
                serial.write(s.as_bytes());
            }
        }
        Ok(())
    }
}

/// **UNSAFE!**
/// Serial `print!()` that shatters the serial lock on print and as such
/// is unsafe. Meant to be used in panics.
#[macro_export]
macro_rules! print_shatter {
    ($($arg:tt)*) => {{
        let _ = core::fmt::write(&mut $crate::print::SerialShatter,
                                 format_args!($($arg)*));
    }}
}

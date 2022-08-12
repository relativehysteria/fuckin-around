//! Variables that the kernel and the bootloader commonly share.
//! As such, these are never freed from the memory.

#![no_std]

use spinlock::SpinLock;
use serial_driver::Serial;

/// Variables that the kernel and the bootloader commonly share.
/// Since this structure passes between both the 32-bit and 64-bit modes,
/// its size must be identical (no pointers, references, usizes).
#[repr(C)]
pub struct BootKernCommon {
    /// A spinlock-guarded serial driver.
    pub serial: SpinLock<Option<Serial>>,
}

impl BootKernCommon {
    /// Returns a new, *UNINITIALIZED* `BootKernCommon` struct.
    pub const fn new() -> Self {
        Self {
            serial: SpinLock::new(None),
        }
    }
}

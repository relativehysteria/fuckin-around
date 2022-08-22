//! Variables that the kernel and the bootloader commonly share.
//! As such, these are never freed from the memory.

#![no_std]

use spinlock::SpinLock;
use serial_driver::Serial;
use range_set::RangeSet;

/// Variables that the kernel and the bootloader commonly share.
/// Since this structure passes between both the 32-bit and 64-bit modes,
/// its size must be identical (no pointers, references, usizes).
#[repr(C)]
pub struct BootKernCommon {
    /// A spinlock-guarded serial driver.
    pub serial: SpinLock<Option<Serial>>,

    /// Memory available for use by the kernel and the bootloader.
    /// Since the memory can be used by both of them at the same time,
    /// you have to make sure that you do not run into a deadlock!
    free_memory: SpinLock<Option<RangeSet>>,
}

impl BootKernCommon {
    /// Returns a new, *UNINITIALIZED* `BootKernCommon` struct.
    pub const fn new() -> Self {
        Self {
            serial:      SpinLock::new(None),
            free_memory: SpinLock::new(None),
        }
    }

    /// Returns a reference to the locked free memory.
    /// Since the memory can be used by both the bootloader and the kernel
    /// at the same time, you have to make sure that you do not run into
    /// a deadlock as a result of this.
    pub unsafe fn free_memory_ref(&self) -> &SpinLock<Option<RangeSet>> {
        &self.free_memory
    }
}

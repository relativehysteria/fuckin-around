#![no_std]
#![no_main]

extern crate core_reqs;

use core::panic::PanicInfo;
use core::hint::spin_loop;
use serial_driver::Serial;
use boot_kern_common::BootKernCommon;

pub static BOOT_KERN: BootKernCommon = BootKernCommon::new();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        let serial = &mut *BOOT_KERN.serial.shatter();
        if let Some(serial) = serial {
            serial.write(b"\nPANIC!\n");
        }
        core::arch::asm!("cli", "hlt");
        loop { spin_loop(); }
    }
}

#[no_mangle]
#[export_name="_start"]
extern fn entry() -> ! {
    // Initialize the serial driver
    {
        let mut serial = BOOT_KERN.serial.lock();
        *serial = Some(Serial::init());

        let serial = serial.as_mut().unwrap();
        serial.write(b"Serial driver initialized.\n");
    }
    panic!();
}

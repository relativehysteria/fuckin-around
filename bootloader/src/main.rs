#![no_std]
#![no_main]

#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate core_reqs;
mod realmode;
mod mm;

use core::panic::PanicInfo;
use core::hint::spin_loop;
use serial_driver::Serial;
use boot_kern_common::BootKernCommon;

#[macro_use] pub mod print;

pub static BOOT_KERN: BootKernCommon = BootKernCommon::new();

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        // Print the panic header
        print_shatter!("\n---- PANIC! ---- ");

        // Print the location information
        if let Some(info) = info.location() {
            print_shatter!(
                "{} {}:{} ----",
                info.file(),
                info.column(),
                info.line(),
            );
        }

        // Print the panic payload
        if let Some(info) = info.message() {
            print_shatter!(" {}  ----", info);
        }

        // End the panic message
        print_shatter!("\n");

        // Halt
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
    }
    print!("Serial initialized!\n");

    // Initialize the primitive memory manager
    mm::init();
    print!("Memory manager initialized!\n");

    panic!();
}

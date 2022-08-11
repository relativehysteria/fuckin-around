#![no_std]
#![no_main]

extern crate core_reqs;

use core::panic::PanicInfo;
use serial_driver::Serial;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { core::arch::asm!("cli", "hlt"); }
    loop {}
}

#[no_mangle]
#[export_name="_start"]
extern fn entry() -> ! {
    // Initialize the serial driver
    let mut serial = Serial::init();
    loop {
        if let Some(byte) = serial.read_byte() {
            serial.write(&[byte]);
        }
    }
}

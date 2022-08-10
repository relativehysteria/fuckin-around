#![no_std]
#![no_main]

extern crate core_reqs;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
#[export_name="_start"]
pub fn entry() -> ! {
    panic!();
}

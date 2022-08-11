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
    unsafe {
        for i in 0..(80*25) {
            core::ptr::write((0xb8000 as *mut u16).offset(i), 0x0F41);
        }
        core::arch::asm!("cli", "hlt");
    }
    panic!();
}

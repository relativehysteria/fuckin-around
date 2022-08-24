#![no_std]
#![no_main]

#![feature(lang_items)]

extern crate core_reqs;

use core::panic::PanicInfo;
use core::hint::spin_loop;

#[lang = "eh_personality"]
fn eh_personality() {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { core::arch::asm!("cli", "hlt"); }
    loop { spin_loop(); }
}

#[no_mangle]
#[export_name="_start"]
extern fn entry() -> ! {
    panic!();
}

//! x86 CPU routines

#![no_std]

use core::arch::asm;

/// Write a byte to I/O port `addr`
#[inline]
pub unsafe fn out8(addr: u16, byte: u8) {
    asm!("out dx, al", in("dx") addr, in("al") byte);
}

/// Read a byte from I/O port `addr`
#[inline]
pub unsafe fn in8(addr: u16) -> u8 {
    let mut byte: u8;
    asm!("in al, dx", in("dx") addr, out("al") byte);
    byte
}

/// Write 4 bytes to I/O port `addr`
#[inline]
pub unsafe fn out32(addr: u16, bytes: u32) {
    asm!("out dx, eax", in("dx") addr, in("eax") bytes);
}

/// Input a byte from I/O port `addr`
#[inline]
pub unsafe fn in32(addr: u16) -> u32 {
    let mut bytes: u32;
    asm!("in eax, dx", in("dx") addr, out("eax") bytes);
    bytes
}

//! Requirements for The Rust Core Library™.
#![no_std]
use core::arch::asm;

#[no_mangle]
pub unsafe extern fn memcpy(dest: *mut u8, src: *mut u8, n: usize) -> *mut u8 {
    // If the `src` is placed before `dest`, copy the memory backwards.
    // Thus the memory won't overwrite itself as it copies bytes.
    if src < dest {
        let mut i = n;
        while i != 0 {
            i -= 1;
            *dest.offset(i as isize) = *src.offset(i as isize);
        }
    } else {
        let mut i = 0;
        while i < n {
            *dest.offset(i as isize) = *src.offset(i as isize);
            i += 1;
        }
    }
    dest
}

#[no_mangle]
pub unsafe extern fn memcmp(s1: *mut u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.offset(i as isize);
        let b = *s2.offset(i as isize);
        if a != b {
            return (a - b) as i32;
        }
        i += 1;
    }
    0
}

#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub unsafe extern fn memset(s: *const u8, c: i32, n: usize) -> *const u8 {
    if n == 0 { return s; }
    asm!("rep stosb", in("rax") c, inout("rdi") s => _, inout("rcx") n => _);
    s
}

#[no_mangle]
#[cfg(target_arch = "x86")]
pub unsafe extern fn memset(s: *const u8, c: i32, n: usize) -> *const u8 {
    if n == 0 { return s; }
    asm!("rep stosb", in("eax") c, inout("edi") s => _, inout("ecx") n => _);
    s
}

#[no_mangle]
pub unsafe extern fn strlen(s: *const u8) -> usize {
    let mut i = 0;
    while *s.offset(i as isize) != b'\0' {
        i += 1;
    }
    i
}

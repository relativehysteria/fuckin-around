//! Parser for executable ELF files.
//! It is expected that these files are static executable binaries and not
//! for example dynamic libraries.
#![no_std]

use core::convert::TryInto;

/// Signifies that a file is aimed at 32-bit systems
pub const BITNESS_32B: u8 = 1;
/// Signifies that a file is aimed at 64-bit systems
pub const BITNESS_64B: u8 = 2;

/// Signifies that a segment is executable
pub const SEGMENT_EXECUTABLE: u32 = 1 << 0;
/// Signifies that a segment is writable
pub const SEGMENT_WRITABLE:   u32 = 1 << 1;
/// Signifies that a segment is readable
pub const SEGMENT_READABLE:   u32 = 1 << 2;

/// Read bytes and little-endian interpret them as a given type
#[macro_export]
macro_rules! get_bytes {
    ($type:ty, $bytes:expr, $offset:expr) => {{
        use core::mem::size_of;
        let range = $offset..($offset.checked_add(size_of::<$type>())?);
        <$type>::from_le_bytes($bytes.get(range)?.try_into().ok()?)
    }}
}

/// A validated ELF file with information extracted out of it for ease
/// of usability.
pub struct ElfParser<'a> {
    /// The raw bytes of the ELF file
    bytes: &'a [u8],

    /// Offset to where the program header table begins
    phdr_off: usize,

    /// Size of a program header table entry
    phent_size: usize,

    /// Number of program headers in this file
    phdr_num: usize,

    /// Bitness of the file
    bitness: u8,

    /// Virtual address of the entry point
    pub entry: u64,
}

impl<'a> ElfParser<'a> {
    /// Parse an ELF file and returned its parsed representation.
    /// This function expects the file to be in the little endian format
    /// and the ELF version to be `1`.
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        let bytes: &[u8] = bytes.as_ref();

        // Check for the ELF header
        if bytes.get(..4) != Some(b"\x7FELF") {
            return None;
        }

        // Get the bitness of the file
        let bitness = *bytes.get(4)?;
        if bitness != BITNESS_32B && bitness != BITNESS_64B { return None; }

        // Verify the endianness
        if bytes.get(5) != Some(&1) { return None; }

        // Verify the ELF version
        if bytes.get(6) != Some(&1) { return None; }

        // Get the entry point
        let entry: u64 = match bitness {
            BITNESS_32B => get_bytes!(u32, bytes, 24usize).into(),
            BITNESS_64B => get_bytes!(u64, bytes, 24usize).into(),
            ___________ => unreachable!(),
        };

        // Get the phdr table offset
        let phdr_off: usize = match bitness {
            BITNESS_32B => get_bytes!(u32, bytes, 28usize).try_into().ok()?,
            BITNESS_64B => get_bytes!(u64, bytes, 32usize).try_into().ok()?,
            ___________ => unreachable!(),
        };

        // Get the size of a phdr table entry
        let phent_size: usize = match bitness {
            BITNESS_32B => get_bytes!(u16, bytes, 42usize).into(),
            BITNESS_64B => get_bytes!(u16, bytes, 52usize).into(),
            ___________ => unreachable!(),
        };

        // Get the number of phdr table entries
        let phdr_num: usize = match bitness {
            BITNESS_32B => get_bytes!(u16, bytes, 44usize).into(),
            BITNESS_64B => get_bytes!(u16, bytes, 54usize).into(),
            ___________ => unreachable!(),
        };

        // Make sure that all the program headers are in bounds of the bytes
        let phdr_table_size = phent_size.checked_mul(phdr_num)?;
        if phdr_off.checked_add(phdr_table_size.into())? >= bytes.len() {
            return None;
        }

        Some(Self {
            bytes,
            phdr_off,
            phent_size,
            phdr_num,
            bitness,
            entry,
        })
    }

    /// Invoke a closure on every LOAD program header with the format
    /// (vaddr, memsz, raw_segment_bytes, read, write, execute)
    pub fn headers<F>(&self, mut closure: F) -> Option<()>
    where F: FnMut(usize, usize, &[u8], bool, bool, bool) -> Option <()> {
        let bytes = self.bytes;

        // Iterate through every program header
        for phdr in 0..self.phdr_num {
            // Get the beginning of this header.
            // These calculations will not overflow because they've been checked
            // during parsing.
            let seg_off = self.phdr_off + (phdr * self.phent_size);

            // If we don't have a LOAD segment, get another one
            if get_bytes!(u32, bytes, seg_off) != 1 {
                continue;
            }

            // Get the segment flags, offsets and sizes
            let flags:    u32;
            let f_off:  usize;
            let f_sz:   usize;
            let vaddr:  usize;
            let mem_sz: usize;

            if self.bitness == BITNESS_32B {
                f_off  = get_bytes!(u32, bytes, seg_off +0x4).try_into().ok()?;
                vaddr  = get_bytes!(u32, bytes, seg_off +0x8).try_into().ok()?;
                f_sz   = get_bytes!(u32, bytes, seg_off +0x10).try_into().ok()?;
                mem_sz = get_bytes!(u32, bytes, seg_off +0x14).try_into().ok()?;
                flags  = get_bytes!(u32, bytes, seg_off +0x18).try_into().ok()?;
            } else if self.bitness == BITNESS_64B {
                flags  = get_bytes!(u32, bytes, seg_off +0x4).try_into().ok()?;
                f_off  = get_bytes!(u64, bytes, seg_off +0x8).try_into().ok()?;
                vaddr  = get_bytes!(u64, bytes, seg_off +0x10).try_into().ok()?;
                f_sz   = get_bytes!(u64, bytes, seg_off +0x20).try_into().ok()?;
                mem_sz = get_bytes!(u64, bytes, seg_off +0x28).try_into().ok()?;
            } else {
                unreachable!()
            };

            // Truncate the file size if it exceeds the segment size
            let f_sz: usize = core::cmp::min(f_sz, mem_sz).try_into().ok()?;

            // Invoke the closure
            closure(
                vaddr,
                mem_sz,
                bytes.get(f_off..f_off.checked_add(f_sz)?)?,
                (flags & SEGMENT_READABLE)   != 0,
                (flags & SEGMENT_WRITABLE)   != 0,
                (flags & SEGMENT_EXECUTABLE) != 0,
            );
        }

        Some(())
    }
}

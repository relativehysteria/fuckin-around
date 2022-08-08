//! Parser for executable ELF files.
//! It is expected that these files are static executable binaries and not
//! for example dynamic libraries.
#![no_std]

use core::convert::TryInto;

/// A validated ELF file with information extracted out of it for ease
/// of usability.
pub struct ElfParser<'a> {
    /// The raw bytes of the ELF file
    bytes: &'a [u8],

    /// Offset to where the program header table begins
    phdr_off: usize,

    /// Size of a program header table entry
    phent_size: u16,

    /// Number of program headers in this file
    phdr_num: u16,

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
        if bitness != 1 && bitness != 2 { return None; }

        // Verify the endianness
        if bytes.get(5) != Some(&1) { return None; }

        // Verify the ELF version
        if bytes.get(6) != Some(&1) { return None; }

        // Get the entry point
        let entry: u64 = if bitness == 1 {
            u32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?).into()
        } else {
            u64::from_le_bytes(bytes.get(24..32)?.try_into().ok()?)
        };

        // Get the phdr table offset
        let phdr_off: usize = if bitness == 1 {
            u32::from_le_bytes(bytes.get(28..32)?.try_into().ok()?)
                .try_into().ok()?
        } else {
            u64::from_le_bytes(bytes.get(32..40)?.try_into().ok()?)
                .try_into().ok()?
        };

        // Get the size of a phdr table entry
        let phent_size: u16 = if bitness == 1 {
            u16::from_le_bytes(bytes.get(42..44)?.try_into().ok()?)
        } else {
            u16::from_le_bytes(bytes.get(52..54)?.try_into().ok()?)
        };

        // Get the number of phdr table entries
        let phdr_num: u16 = if bitness == 1 {
            u16::from_le_bytes(bytes.get(44..46)?.try_into().ok()?)
        } else {
            u16::from_le_bytes(bytes.get(54..56)?.try_into().ok()?)
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
            entry,
        })
    }
}

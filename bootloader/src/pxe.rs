//! Real-mode PXE API implementation. This API only works with PXE version >2.1.
//!
//! The 16-bit API is used instead of the 32-bit one, because it's much better
//! defined. Too many things don't work in 32-bit...

use alloc::vec::Vec;
use spinlock::SpinLock;
use crate::print;
use crate::realmode;
use crate::realmode::pxe_invoke;

/// A guard that prevents more than one PXE routine running at once
static GUARD: SpinLock<()> = SpinLock::new(());

/// Converts `seg:off` into a linear address
fn seg_off(seg: u16, off: u16) -> usize {
    (seg as usize * 0x10) + off as usize
}

/// Download a file over TFTP using the 16-bit PXE API.
pub fn download(filename: &[u8]) -> Option<Vec<u8>> {
    // Lock the GUARD to make sure we are the only one using the PXE interface
    let _guard = GUARD.lock();

    // The common buffer size used for all PXE operations
    const BUFFER_SIZE: u16 = 512;

    // Create a new empty register state for the interrupt
    let mut registers = realmode::RegisterState::default();
    registers.eax = 0x5650;

    // Invoke the PXE check interrupt
    unsafe { realmode::invoke(0x1A, &mut registers); }

    // Check if PXE is present
    if registers.eax != 0x564E || (registers.efl & 1) != 0 {
        return None;
    }

    // Read the PXENV+ structure
    let pxenv = seg_off(registers.es, registers.ebx as u16);
    let pxenv = unsafe {
        core::slice::from_raw_parts(pxenv as *const u8, 0x2C)
    };

    // Extract the fields needed to validate the PXENV+ structure
    let signature = &pxenv[..0x6];
    let version   = u16::from_le_bytes(pxenv[0x6..0x8].try_into().ok()?);
    let length    = pxenv[0x8];
    let sum       = pxenv.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));

    // Correctness check
    if signature != b"PXENV+" || version < 0x201 || length != 0x2C || sum != 0 {
        return None;
    }

    // Read the PXE! structure
    let off = u16::from_le_bytes(pxenv[0x28..0x2A].try_into().ok()?);
    let seg = u16::from_le_bytes(pxenv[0x2A..0x2C].try_into().ok()?);
    let pxe = seg_off(seg, off);
    let pxe = unsafe {
        core::slice::from_raw_parts(pxe as *const u8, 0x58)
    };

    // Extract the fields needed to validate the !PXE structure
    let signature = &pxe[..0x4];
    let length    = pxe[0x4];
    let sum       = pxe.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));

    // Correctness check
    if signature != b"!PXE" || length != 0x58 || sum != 0 {
        return None;
    }

    // Get the PXE API entry point
    let entry_off = u16::from_le_bytes(pxe[0x10..0x12].try_into().ok()?);
    let entry_seg = u16::from_le_bytes(pxe[0x12..0x14].try_into().ok()?);

    // CS must not be 0
    if entry_seg == 0 {
        return None;
    }

    // Retrieve the server IP address from the packet that was cached during
    // the PXE boot process.
    let server_ip: [u8; 4] = {
        const GET_CACHED_INFO: u16 = 0x71;
        const PACKET_TYPE_DHCP_ACK: u16 = 2;

        #[derive(Default)]
        #[repr(C)]
        struct GetCachedInfo {
            status:      u16,
            packet_type: u16,
            buf_size:    u16,
            buf_off:     u16,
            buf_seg:     u16,
            buf_limit:   u16,
        }

        // Create the request
        let mut request     = GetCachedInfo::default();
        request.packet_type = PACKET_TYPE_DHCP_ACK;
        unsafe {
            pxe_invoke(entry_seg, entry_off, GET_CACHED_INFO, 0,
                       &mut request as *mut _ as u16);
        }

        // Check whether this call was successful
        if request.status != 0 {
            return None;
        }

        // Read the packet
        let packet = unsafe {
            core::slice::from_raw_parts(
                seg_off(request.buf_seg, request.buf_off) as *const u8,
                request.buf_size as usize)
        };

        // Extract the IP
        packet[0x14..0x18].try_into().ok()?
    };

    // Get the file size
    let file_size = {
        const TFTP_GET_FILE_SIZE: u16 = 0x25;

        #[repr(C, packed)]
        struct GetFileSize {
            status:     u16,
            server_ip:  [u8; 4],
            gateway_ip: [u8; 4],
            filename:   [u8; 128],
            file_size:  u32,
        }

        // Create request
        let mut request = GetFileSize {
            status:     0,
            server_ip:  server_ip,
            gateway_ip: [0; 4],
            filename:   [0; 128],
            file_size:  0,
        };

        // Check that we have enough room for the file name + NUL.
        if filename.len() + 1 > request.filename.len() {
            return None;
        }

        // ANOTHER BUG: XXX XXX XXX XXX XXX XXX XXX XXX XXX XXX XXX XXX XXX XXX

        // Copy the file name
        request.filename[..filename.len()].copy_from_slice(filename);

        // Invoke the request
        unsafe {
            pxe_invoke(entry_seg, entry_off, TFTP_GET_FILE_SIZE, 0,
                       &mut request as *mut _ as u16);
        }

        // Check whether this call was successful
        if request.status != 0 {
            return None;
        }

        request.file_size as usize
    };

    // Open the file
    {
        const TFTP_OPEN: u16 = 0x20;

        #[repr(C)]
        struct TftpOpen {
            status:      u16,
            server_ip:   [u8; 4],
            gateway_ip:  [u8; 4],
            filename:    [u8; 128],
            tftp_port:   u16,
            packet_size: u16,
        }

        // Create the request
        let mut request = TftpOpen {
            status:      0,
            server_ip:   server_ip,
            gateway_ip:  [0; 4],
            filename:    [0; 128],
            tftp_port:   69u16.to_be(), // Nice
            packet_size: BUFFER_SIZE,
        };

        // Copy the file name
        request.filename[..filename.len()].copy_from_slice(filename);

        // Invoke the request
        unsafe {
            pxe_invoke(entry_seg, entry_off, TFTP_OPEN, 0,
                       &mut request as *mut _ as u16);
        }

        // Check whether this call was successful
        if request.status != 0 || request.packet_size != 512 {
            return None;
        }
    }

    // Read the file
    let mut download = Vec::with_capacity(file_size);
    loop {
        const TFTP_READ: u16 = 0x22;

        #[repr(C)]
        struct TftpRead {
            status:     u16,
            packet_num: u16,
            bytes_read: u16,
            buf_off:    u16,
            buf_seg:    u16,
        }

        // Prepare the buffer needed for this request
        let mut buffer = [0u8; BUFFER_SIZE as usize];

        // Create the request
        let mut request = TftpRead {
            status:     0,
            packet_num: 0,
            bytes_read: 0,
            buf_off:    &mut buffer as *mut _ as u16,
            buf_seg:    0,
        };

        // Invoke the request
        unsafe {
            pxe_invoke(entry_seg, entry_off, TFTP_READ, 0,
                       &mut request as *mut _ as u16);
        }

        // Get the number of bytes read
        let bytes_read = request.bytes_read as usize;

        // Check whether this call was successful
        if request.status != 0 || bytes_read > buffer.len() {
            return None;
        }

        // Make sure we don't overflow
        if download.len() + bytes_read > download.capacity() {
            return None;
        }

        // Save the downloaded bytes
        download.extend_from_slice(&buffer[..bytes_read]);

        // If this was the last packet, stop reading the file
        if bytes_read < buffer.len() {
            break;
        }
    }

    // Close the file
    {
        const TFTP_CLOSE: u16 = 0x21;

        // Status taken by the pxe call
        let mut status: u16 = 0;

        // Invoke the request
        unsafe {
            pxe_invoke(entry_seg, entry_off, TFTP_CLOSE, 0,
                       &mut status as *mut _ as u16);
        }

        // Check whether the file closed successfully
        if status != 0 {
            return None;
        }
    }

    Some(download)
}

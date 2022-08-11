//! Serial 8250 UART driver
// USB is annoying. Suck my dongle.

#![no_std]

/// Serial ports identified by the BIOS.
#[repr(C)]
pub struct Serial {
    pub devices: [Option<u16>; 4],
}

/// Flag that marks the serial as initialized.
/// This is used to check whether we are trying to initialize the driver twice.
static mut SERIAL_INITIALIZED: bool = false;

impl Serial{
    /// Initialize the serial ports on the system to 28800n1.
    /// The driver can't be initialized more than once.
    pub fn init() -> Self {
        // Make sure that we haven't initialized the driver already
        unsafe {
            assert!(!SERIAL_INITIALIZED, "Serial driver initialized twice.");
        }

        // Base address of the BIOS Data Area
        let bda_addr = 0x400 as *const u16;

        // Allocate space for serial ports
        let mut ports = Self {
            devices: [None; 4],
        };

        // Go through each COM port
        for (id, device) in ports.devices.iter_mut().enumerate() {
            // Get the COM port I/O address
            let port = unsafe { *bda_addr.offset(id as isize) };

            // Check if the port is present. If not, proceed to the next one
            if port == 0 { continue; }

            // Initialize the port
            unsafe {
                cpu::out8(port + 1, 0x00); // Disable all interrupts
                cpu::out8(port + 3, 0x80); // Enable DLAB (set baud divisor)
                cpu::out8(port + 0, 0x04); // (low byte) Divisor = 115200 / this
                cpu::out8(port + 1, 0x00); // (high byte)
                cpu::out8(port + 3, 0x03); // 8 bits, no parity, one stop bit
                cpu::out8(port + 4, 0x03); // IRQs disabled, RTS/DSR set
            }

            // Save the port
            *device = Some(port);
        }

        // Mark the driver as initialized
        unsafe { SERIAL_INITIALIZED = true };
        ports
    }

    /// Read a byte from the first COM port that has a byte available
    pub fn read_byte(&mut self) -> Option<u8> {
        // Iterate through the devices
        for port in self.devices.iter() {
            // Check whether the device is present
            if let Some(port) = *port {
                unsafe {
                    // Check if there is a byte available.
                    // If yes, read and return it
                    if (cpu::in8(port + 5) & 1) != 0 {
                        return Some(cpu::in8(port));
                    }
                }
            }
        }

        // No bytes to read
        None
    }

    /// Write a byte to a COM port
    fn write_byte(&mut self, port: usize, byte: u8) {
        // Check if this port exists
        if let Some(&Some(port)) = self.devices.get(port) {
            unsafe {
                // Wait for the transmit to be empty
                while cpu::in8(port + 5) & 0x20 == 0 {};

                // Write the byte
                cpu::out8(port, byte);
            }
        }
    }

    /// Write bytes to all mapped serial devices
    pub fn write(&mut self, bytes: &[u8]) {
        // Iterate through the bytes
        for &byte in bytes {

            // Write the byte to all mapped serial devices
            for port in 0..self.devices.len() {
                // Handle newlines correctly
                if byte == b'\n' { self.write_byte(port, b'\r'); }

                // Write the byte
                self.write_byte(port, byte);
            }
        }
    }
}

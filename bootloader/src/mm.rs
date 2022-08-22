//! Memory allocator/management for the bootloader in protected mode.

use range_set::{ RangeSet, Range };
use core::alloc::{ GlobalAlloc, Layout };
use crate::{ realmode, BOOT_KERN };

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    panic!("Out of memory.");
}

/// The global allocator in the bootloader space. Physical memory is used as a
/// backing and fragmentation is NOT handled.
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

/// The structure used in `GLOBAL_ALLOCATOR` that implements the `GlobalAlloc`
/// trait.
struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut physical_memory = BOOT_KERN.free_memory_ref().lock();

        physical_memory.as_mut().and_then(|mem| {
            mem.allocate(layout.size() as u64, layout.align() as u64, None)
        }).unwrap_or(0) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut physical_memory = BOOT_KERN.free_memory_ref().lock();

        physical_memory.as_mut().and_then(|mem| {
            let end = (ptr as u64)
                .checked_add(layout.size().checked_sub(1)? as u64)?;
            mem.insert(Range::new(ptr as u64, end));
            Some(())
        }).expect("The memory manager isn't initialized. Can't free memory.")
    }
}

/// Initialize the bootloader physical memory manager.
///
/// The initial memory map is retrieved through E820 and the first 1 MiB of
/// memory is marked as reserved.
// http://www.uruk.org/orig-grub/mem64mb.html
pub fn init() {
    // Get a handle to the free physical memory
    let mut physical_memory = unsafe { BOOT_KERN.free_memory_ref().lock() };

    // Create a new empty range set for tracking free physical memory
    let mut free_memory = RangeSet::new();

    // Loop through the memory as reported by the BIOS, twice.
    // We only care about the memory the BIOS flags as free.
    // However, sometimes memory is flagged as both free and reserved...
    // The first pass of this loop gets all the free memory,
    // the second pass gets rid of everything that is both free and reserved.
    for add in [true, false] {
        // Prepare a new register state for the upcoming E820 calls
        let mut registers = realmode::RegisterState::default();
        registers.ebx = 0;

        loop {
            /// The struct taken and returned by E820 in ES:DI.
            #[derive(Default)]
            #[repr(C)]
            struct AddressRangeDescriptor {
                base: u64,
                size: u64,
                typ:  u8,
            }

            // Create a new E820 entry
            let mut descriptor = AddressRangeDescriptor::default();

            // Prepare the registers for the E820 call
            registers.eax = 0xE820;
            registers.edi = &mut descriptor as *const AddressRangeDescriptor as u32;
            registers.ecx = core::mem::size_of_val(&descriptor) as u32;
            registers.edx = u32::from_be_bytes(*b"SMAP");

            // Invoke the E820 interrupt
            unsafe { realmode::invoke(0x15, &mut registers); }

            // If we got an error, panic
            if (registers.efl & 1) != 0 {
                panic!("Error on E820");
            }

            if add && descriptor.typ == 1 && descriptor.size > 0 {
                // If we are in the first pass and the memory is marked free,
                // track it.
                free_memory.insert(Range::new(
                    descriptor.base,
                    descriptor.base.checked_add(descriptor.size - 1).unwrap(),
                ));
            } else if !add && descriptor.typ != 1 && descriptor.size > 0 {
                // If we are in the second pass and the memory is reserved,
                // stop tracking it
                free_memory.remove(Range::new(
                    descriptor.base,
                    descriptor.base.checked_add(descriptor.size - 1).unwrap(),
                ));
            }

            // If the BIOS tells us to stop, do so
            if registers.ebx == 0 {
                break;
            }
        }

        // Mark the first 1 MiB of memory as reserved
        free_memory.remove(Range::new(0, 1024 * 1024 - 1));

        *physical_memory = Some(free_memory);
    }
}

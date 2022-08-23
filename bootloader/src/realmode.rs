//! Functions to perform 16-bit calls from 32-bit land

/// General purpose 32-bit x86 registers
#[repr(C)]
#[derive(Default, Debug)]
pub struct RegisterState {
	pub eax: u32,
	pub ecx: u32,
	pub edx: u32,
	pub ebx: u32,
	pub esp: u32,
	pub ebp: u32,
	pub esi: u32,
	pub edi: u32,
	pub efl: u32,
	pub es: u16,
	pub ds: u16,
	pub fs: u16,
	pub gs: u16,
	pub ss: u16,
}

extern {
    /// Invokes a real mode software interrupt `interrupt_number` with a given
    /// register state.
    pub fn invoke(interrupt_number: u8, registers: *mut RegisterState);

    /// Invokes a PXE routine `pxe_opcode`.
    pub fn pxe_invoke(entry_segment: u16, entry_offset: u16, pxe_opcode: u16,
                      parameter_segment: u16, parameter_offset: u16);
}

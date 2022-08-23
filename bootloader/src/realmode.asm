; Real mode routines for invoking real mode software interrupts (`invoke()`).
; PXE API calling convention is also covered (`pxe_invoke()`).

[bits 32]

struc register_state
    ; u32 registers
    .eax: resd 1
    .ecx: resd 1
    .edx: resd 1
    .ebx: resd 1
    .esp: resd 1
    .ebp: resd 1
    .esi: resd 1
    .edi: resd 1
    .efl: resd 1

    ; u16 registers
    .es: resw 1
    .ds: resw 1
    .fs: resw 1
    .gs: resw 1
    .ss: resw 1
endstruc

section .text

global invoke

; Invoke a real mode software interrupt with given `register_state`
; fn invoke(interrupt_number: u8, registers: *mut RegisterState);
invoke:
    ; Disable interrupts
    cli

    ; Save the registers
    pushad

    ; Set up the data selectors
    mov ax, 0x10 ; 0x10 is the 16-bit data entry in the GDT
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; Start executing 16-bit instructions.
    ; origin is defined at build time and is usually set to 0x7c00
    jmp 0x8:(.bits16 - origin)

[bits 16]

.bits16:
    ; Disable protected mode
    mov eax, cr0
    and eax, ~1
    mov cr0, eax

    ; Clear the data selectors
    xor ax, ax
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; In Real-Address Mode, the IRET instruction preforms a far return to the
    ; interrupted program or procedure. During this operation, the processor
    ; pops the return instruction pointer, return cs selector, and EFLAGS from
    ; the stack to the EIP, CS, and EFLAGS registers, respectively...
    ;
    ; Set up a fake interrupt frame to perform a long jump to .sw_interrupt
    pushfd                              ; EFLAGS
    push dword (origin >> 4)            ; CS
    push dword (.sw_interrupt - origin) ; EIP
    iretd

.sw_interrupt:
    ; Get the arguments passed to `invoke()`
    movzx ebx, byte  [esp + (4*0x9)] ; interrupt_number
    shl   ebx, 2
    mov   eax, dword [esp + (4*0xa)] ; *mut RegisterState

    ; Set up an interrupt stack frame for the return.
    ; Once the interrupt finishes execution, this is where we are going
    ; to return to (that is, .return - origin)
    mov ebp, (.return - origin)
    pushfw
    push cs
    push bp

    ; Prepare for the interrupt by loading the the contents of the IVT
    ; based on the interrupt number specified
    pushfw
    push word [bx+2]
    push word [bx+0]

    ; Load the register state passed to `invoke()`
    mov ecx, dword [eax + register_state.ecx]
    mov edx, dword [eax + register_state.edx]
    mov ebx, dword [eax + register_state.ebx]
    mov ebp, dword [eax + register_state.ebp]
    mov esi, dword [eax + register_state.esi]
    mov edi, dword [eax + register_state.edi]
    mov eax, dword [eax + register_state.eax]

    ; Execute the interrupt
    iretw

.return:
    ; Save off all registers
    push eax
    push ecx
    push edx
    push ebx
    push ebp
    push esi
    push edi
    pushfd
    push es
    push ds
    push fs
    push gs
    push ss

    ; Get a pointer to the register state passed to `invoke()`.
    ; (4*0xa) = pointer to the register state on the stack
    ; (8*4)   = 8 4-byte registers that we have just pushed to the stack
    ; (5*2)   = 4 2-byte registers that we have just pushed to the stack
    mov eax, dword [esp + (4*0xa) + (8*4) + (5*2)]

    ; Update the register state with what we were left after the interrupt
    pop  word [eax + register_state.ss]
    pop  word [eax + register_state.gs]
    pop  word [eax + register_state.fs]
    pop  word [eax + register_state.ds]
    pop  word [eax + register_state.es]
    pop dword [eax + register_state.efl]
    pop dword [eax + register_state.edi]
    pop dword [eax + register_state.esi]
    pop dword [eax + register_state.ebp]
    pop dword [eax + register_state.ebx]
    pop dword [eax + register_state.edx]
    pop dword [eax + register_state.ecx]
    pop dword [eax + register_state.eax]

    ; Enable protected mode
    mov eax, cr0
    or  eax, 1
    mov cr0, eax

    ; Set up the data selectors
    mov ax, 0x20  ; 0x20 is the 32-bit data entry in the GDT
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; Interrupt frame (i.e. a long jump) back to protected mode
    pushfd                           ; EFLAGS
    push dword 0x18                  ; CS = 32-bit code entry in the GDT
    push dword protected_mode_return ; EIP
    iretd

[bits 32]

global pxe_invoke

; Call a given `pxe_opcode` PXE routine.
; fn pxe_invoke(entry_segment: u16, entry_offset: u16, pxe_opcode: u16,
;               parameter_segment: u16, parameter_offset: u16);
pxe_invoke:
    ; Disable interrupts
    cli

    ; Save the registers
    pushad

    ; Set up the data selectors
    mov ax, 0x10 ; 0x10 is the 16-bit data entry in the GDT
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; Start executing 16-bit instructions.
    ; origin is defined at build time and is usually set to 0x7c00
    jmp 0x8:(.bits16 - origin)

[bits 16]

.bits16:
    ; Disable protected mode
    mov eax, cr0
    and eax, ~1
    mov cr0, eax

    ; Clear the data selectors
    xor ax, ax
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; Set up a fake interrupt frame to perform a long jump to .pxe_call.
    ; This code is explained in the `invoke()` code above.
    pushfd                          ; EFLAGS
    push dword (origin >> 4)        ; CS
    push dword (.pxe_call - origin) ; EIP
    iretd

.pxe_call:
    ; Get the arguments passed to `pxe_invoke()`
    movzx eax, word [esp + (4*0x9)] ; entry_segment
    movzx ebx, word [esp + (4*0xa)] ; entry_offset
    movzx ecx, word [esp + (4*0xb)] ; pxe_opcode
    movzx edx, word [esp + (4*0xc)] ; parameter_segment
    movzx esi, word [esp + (4*0xd)] ; parameter_offset

    ; Set up the PXE call parameters
    push dx
    push si
    push cx

    ; Set up an interrupt stack frame for the return.
    ; Once the interrupt finishes execution, this is where we are going
    ; to return to (that is, .return - origin)
    mov ebp, (.return - origin)
    push cs
    push bp

    ; Execute the PXE routine
    pushfw
    push ax
    push bx
    iretw

.return:
    ; Disable the interrupt in case they have been enabled after the call
    cli

    ; Clean up the stack
    add sp, 6

    ; Enable protected mode
    mov eax, cr0
    or  eax, 1
    mov cr0, eax

    ; Set up the data selectors
    mov ax, 0x20  ; 0x20 is the 32-bit data entry in the GDT
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; Interrupt frame (i.e. a long jump) back to protected mode
    pushfd                           ; EFLAGS
    push dword 0x18                  ; CS = 32-bit code entry in the GDT
    push dword protected_mode_return ; EIP
    iretd

[bits 32]

protected_mode_return:
    popad
    ret

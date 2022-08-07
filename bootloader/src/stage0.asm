[org 0x7c00]
[bits 16]

entry:
    ; Disable interrupts, clear direction flag
    cli
    cld

    ; Set the A20
    in    al, 0x92
    or    al, 2
    out 0x92, al

    ; DS is unpredictable after boot -- set it to 0
    xor ax, ax
    mov ds, ax
    mov gs, ax

    ; Load the GDT
    lgdt [gdt]

    ; Enable protected mode
    mov eax, cr0
    or  eax, 1
    mov cr0, eax

    ; Jump to protected mode
    jmp 0x18:protected_mode_entry ; 0x18 is the 32-bit code entry in the GDT

;-------------------------------------------------------------------------------

[bits 32]

protected_mode_entry:
    ; Set up the data selectors
    mov ax, 0x10
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    cli
    hlt

;-------------------------------------------------------------------------------

align 8
gdt_base:
    dq 0x0000000000000000 ; Null descriptor
    dq 0x00009a007c00ffff ; 16-bit code, present, base 0x7c00
    dq 0x000092000000ffff ; 16-bit data, present, base 0x0
    dq 0x00cf9a000000ffff ; 32-bit code, present, base 0x0
    dq 0x00cf92000000ffff ; 32-bit data, present, base 0x0

    ; dq 0x00009a0000000000 ; 64-bit code, present, base 0x0
    ; dq 0x0000920000000000 ; 64-bit data, present, base 0x0

gdt:
    dw (gdt - gdt_base) -1
    dd gdt_base

;-------------------------------------------------------------------------------

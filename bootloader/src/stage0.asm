; origin is defined at build time and should be usually set to 0x7c00
[org origin]
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
    mov ax, 0x20  ; 0x20 is the 32-bit data entry in the GDT
    mov es, ax
    mov ds, ax
    mov gs, ax
    mov fs, ax
    mov ss, ax

    ; Set up a stack
    mov esp, origin

    ; entry_point is defined at build time
    jmp entry_point

;-------------------------------------------------------------------------------

align 8
gdt_base:
    dq 0x0000000000000000 ; 0x00, Null descriptor
    dq 0x00009a007c00ffff ; 0x08, 16-bit code, present, base 0x7c00
    dq 0x000092000000ffff ; 0x10, 16-bit data, present, base 0x0
    dq 0x00cf9a000000ffff ; 0x18, 32-bit code, present, base 0x0
    dq 0x00cf92000000ffff ; 0x20, 32-bit data, present, base 0x0
    dq 0x00009a0000000000 ; 0x28, 64-bit code, present, base 0x0
    dq 0x0000920000000000 ; 0x30, 64-bit data, present, base 0x0

gdt:
    dw (gdt - gdt_base) -1
    dd gdt_base

;-------------------------------------------------------------------------------

; base_address is defined at build time
times ((base_address - origin) - ($-$$)) db 0x0

incbin "build/bootloader"

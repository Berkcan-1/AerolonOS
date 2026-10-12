bits 16
org 0x7C00

PAYLOAD_SEGMENT equ 0x0880
PAYLOAD_ENTRY equ 0x8800
SECTOR_PARAGRAPHS equ 0x80

start:
    jmp short entry
    nop
    times 8 - ($ - $$) db 0

boot_info_pvd:    dd 0
boot_info_file:   dd 0
boot_info_length: dd 0
boot_info_csum:   dd 0
boot_info_pad:    times 10 dd 0

entry:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    cld
    mov [boot_drive], dl

    mov ah, 0x41
    mov bx, 0x55AA
    int 0x13
    jc disk_fail
    cmp bx, 0xAA55
    jne disk_fail

    mov eax, [boot_info_length]
    add eax, 2047
    shr eax, 11
    dec eax
    jz disk_fail
    mov [sectors_left], ax

    mov eax, [boot_info_file]
    inc eax
    mov [dap_lba], eax

.read_loop:
    mov si, dap
    mov dl, [boot_drive]
    mov ah, 0x42
    int 0x13
    jc disk_fail
    add word [dap_segment], SECTOR_PARAGRAPHS
    inc dword [dap_lba]
    dec word [sectors_left]
    jnz .read_loop

    mov dl, [boot_drive]
    jmp 0x0000:PAYLOAD_ENTRY

disk_fail:
    mov si, message_fail
.print:
    lodsb
    test al, al
    jz .halt
    mov ah, 0x0E
    mov bx, 0x0007
    int 0x10
    jmp .print
.halt:
    cli
    hlt
    jmp .halt

boot_drive:   db 0
sectors_left: dw 0

dap:
    db 0x10
    db 0
dap_count:   dw 1
dap_offset:  dw 0
dap_segment: dw PAYLOAD_SEGMENT
dap_lba:     dd 0
             dd 0

message_fail: db 'NetizenCore: disk okuma hatasi', 0

times 2048 - ($ - $$) db 0

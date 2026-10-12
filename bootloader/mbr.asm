bits 16
org 0x7C00

mbr_start:
    jmp short mbr_entry
    nop
    db "NETIZEN"

mbr_entry:
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

    mov word [sectors_left], 1200

.read_loop:
    cmp word [sectors_left], 0
    jbe .done_reading

    mov ax, [sectors_left]
    cmp ax, 64
    jbe .count_ok
    mov ax, 64
.count_ok:
    mov [dap_count], ax
    sub [sectors_left], ax

    mov si, dap
    mov dl, [boot_drive]
    mov ah, 0x42
    int 0x13
    jc disk_fail

    mov ax, [dap_count]
    shl ax, 5
    add [dap_segment], ax

    mov ax, [dap_count]
    add [dap_lba], ax
    adc word [dap_lba + 2], 0

    jmp .read_loop

.done_reading:
    mov dword [0x7C10], 595968
    mov dl, [boot_drive]
    jmp 0x0000:0x8800

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

boot_drive:    db 0
sectors_left:  dw 0

align 4
dap:
    db 0x10
    db 0
dap_count:
    dw 0
dap_offset:
    dw 0x0000
dap_segment:
    dw 0x0880
dap_lba:
    dd 2048
    dd 0

message_fail:  db "NetizenCore: MBR okuma hatasi", 0

times 446 - ($ - $$) db 0

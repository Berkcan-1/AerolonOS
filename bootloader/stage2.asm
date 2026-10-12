bits 16
org 0x8800

%ifndef PREFERRED_WIDTH
%define PREFERRED_WIDTH 1024
%endif
%ifndef PREFERRED_HEIGHT
%define PREFERRED_HEIGHT 768
%endif
%ifndef FALLBACK_WIDTH
%define FALLBACK_WIDTH 1024
%endif
%ifndef FALLBACK_HEIGHT
%define FALLBACK_HEIGHT 768
%endif

PML4_ADDRESS equ 0x1000
PDPT_ADDRESS equ 0x2000
PDT_ADDRESS equ 0x3000
PAGE_TABLE_PAGES equ 6
VBE_INFO equ 0x0600
MODE_INFO equ 0x0800
BOOT_INFO equ 0x0900
BOOT_INFO_MAGIC equ 0x4942434E
BOOT_INFO_VERSION equ 1
BOOT_INFO_WORDS equ 20
KERNEL_SOURCE equ 0x9800
KERNEL_BASE equ 0x100000
BOOT_TABLE_LENGTH equ 0x7C10
STAGE1_SIZE equ 2048
STAGE2_SIZE equ 4096

stage2_start:
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    cld

    mov ax, 0x2401
    int 0x15
    in al, 0x92
    or al, 2
    and al, 0xFE
    out 0x92, al

    pushfd
    pop eax
    mov ecx, eax
    xor eax, 0x200000
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
    xor eax, ecx
    jz no_long_mode

    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb no_long_mode
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    jz no_long_mode

    call setup_video
    jc video_failed

    cli
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:protected_entry

setup_video:
    mov dword [VBE_INFO], 0x32454256
    mov ax, 0x4F00
    mov di, VBE_INFO
    int 0x10
    cmp ax, 0x004F
    jne .fail
    cmp dword [VBE_INFO], 0x41534556
    jne .fail

    mov ax, PREFERRED_WIDTH
    mov bx, PREFERRED_HEIGHT
    call find_mode
    jnc .found
    mov ax, FALLBACK_WIDTH
    mov bx, FALLBACK_HEIGHT
    call find_mode
    jc .fail

.found:
    mov bx, dx
    or bx, 0x4000
    mov ax, 0x4F02
    int 0x10
    cmp ax, 0x004F
    jne .fail
    call fill_boot_info
    clc
    ret

.fail:
    stc
    ret

find_mode:
    mov [want_width], ax
    mov [want_height], bx
    mov si, [VBE_INFO + 14]
    mov ax, [VBE_INFO + 16]
    mov [mode_list_segment], ax

.next:
    mov fs, [mode_list_segment]
    mov cx, [fs:si]
    add si, 2
    cmp cx, 0xFFFF
    je .none
    mov [candidate_mode], cx
    push si
    mov ax, 0x4F01
    mov di, MODE_INFO
    int 0x10
    pop si
    cmp ax, 0x004F
    jne .next
    mov ax, [MODE_INFO]
    and ax, 0x0091
    cmp ax, 0x0091
    jne .next
    cmp byte [MODE_INFO + 25], 32
    jne .next
    cmp byte [MODE_INFO + 27], 6
    jne .next
    mov ax, [MODE_INFO + 18]
    cmp ax, [want_width]
    jne .next
    mov ax, [MODE_INFO + 20]
    cmp ax, [want_height]
    jne .next
    mov dx, [candidate_mode]
    clc
    ret

.none:
    stc
    ret

fill_boot_info:
    xor ax, ax
    mov di, BOOT_INFO
    mov cx, BOOT_INFO_WORDS
    rep stosw
    mov dword [BOOT_INFO], BOOT_INFO_MAGIC
    mov dword [BOOT_INFO + 4], BOOT_INFO_VERSION
    mov eax, [MODE_INFO + 40]
    mov [BOOT_INFO + 0x08], eax
    movzx eax, word [MODE_INFO + 16]
    mov [BOOT_INFO + 0x10], eax
    movzx eax, word [MODE_INFO + 18]
    mov [BOOT_INFO + 0x14], eax
    movzx eax, word [MODE_INFO + 20]
    mov [BOOT_INFO + 0x18], eax
    mov al, [MODE_INFO + 25]
    mov [BOOT_INFO + 0x1C], al
    mov al, [MODE_INFO + 32]
    mov [BOOT_INFO + 0x1D], al
    mov al, [MODE_INFO + 31]
    mov [BOOT_INFO + 0x1E], al
    mov al, [MODE_INFO + 34]
    mov [BOOT_INFO + 0x1F], al
    mov al, [MODE_INFO + 33]
    mov [BOOT_INFO + 0x20], al
    mov al, [MODE_INFO + 36]
    mov [BOOT_INFO + 0x21], al
    mov al, [MODE_INFO + 35]
    mov [BOOT_INFO + 0x22], al
    ret

print_string:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    mov bx, 0x0007
    int 0x10
    jmp print_string
.done:
    ret

no_long_mode:
    mov si, message_no_long_mode
    jmp fatal

video_failed:
    mov si, message_video_failed

fatal:
    call print_string
.halt:
    cli
    hlt
    jmp .halt

want_width: dw 0
want_height: dw 0
candidate_mode: dw 0
mode_list_segment: dw 0

message_no_long_mode: db 'NetizenCore: 64 bit islemci gerekli', 0
message_video_failed: db 'NetizenCore: VBE grafik modu bulunamadi', 0

gdt_start:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
    dq 0x00AF9A000000FFFF
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

bits 32
protected_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x7C00

    mov edi, PML4_ADDRESS
    xor eax, eax
    mov ecx, PAGE_TABLE_PAGES * 4096 / 4
    rep stosd

    mov dword [PML4_ADDRESS], PDPT_ADDRESS | 0x03
    mov dword [PDPT_ADDRESS], PDT_ADDRESS | 0x03
    mov dword [PDPT_ADDRESS + 8], (PDT_ADDRESS + 0x1000) | 0x03
    mov dword [PDPT_ADDRESS + 16], (PDT_ADDRESS + 0x2000) | 0x03
    mov dword [PDPT_ADDRESS + 24], (PDT_ADDRESS + 0x3000) | 0x03

    mov edi, PDT_ADDRESS
    xor eax, eax
    mov ecx, 2048
.fill_pdt:
    mov edx, 0x83
    cmp ecx, 512
    ja .store_entry
    mov edx, 0x93
.store_entry:
    or edx, eax
    mov [edi], edx
    add eax, 0x200000
    add edi, 8
    loop .fill_pdt

    mov ecx, 147456
    mov esi, KERNEL_SOURCE
    mov edi, KERNEL_BASE
    cld
    rep movsd

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov eax, PML4_ADDRESS
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    jmp 0x18:long_entry

bits 64
long_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor eax, eax
    mov fs, ax
    mov gs, ax
    mov rsp, 0x90000
    mov rax, KERNEL_BASE
    jmp rax

times 4096 - ($ - $$) db 0

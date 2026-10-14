use core::arch::global_asm;

global_asm!(
    r#"
    .pushsection .text.entry,"ax",@progbits
    .global _start
_start:
    cli
    cld
    mov rax, cr0
    and ax, 0xFFFB
    or ax, 0x0002
    mov cr0, rax
    mov rax, cr4
    or ax, 3 << 9
    mov cr4, rax
    lea rsp, [rip + __stack_top]
    xor ebp, ebp
    call kernel_main
.Lhalt:
    cli
    hlt
    jmp .Lhalt
    .popsection

    .pushsection .bss.stack,"aw",@nobits
    .balign 16
__stack_bottom:
    .skip 524288
__stack_top:
    .popsection
"#
);
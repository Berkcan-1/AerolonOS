use core::arch::global_asm;

global_asm!(
    r#"
    .pushsection .text.entry,"ax",@progbits
    .global _start
_start:
    cli
    cld
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
    .skip 65536
__stack_top:
    .popsection
"#
);

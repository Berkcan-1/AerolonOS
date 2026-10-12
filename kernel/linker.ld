ENTRY(_start)
SECTIONS
{
    . = 0x100000;
    __kernel_bin_start = .;

    .text : ALIGN(16)
    {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }

    .rodata : ALIGN(16)
    {
        *(.rodata .rodata.*)
        *(.data.rel.ro .data.rel.ro.*)
    }

    .data : ALIGN(16)
    {
        *(.data .data.*)
        *(.got .got.*)
    }

    .bss : ALIGN(16)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    . = ALIGN(16);
    __kernel_bin_end = .;

    /DISCARD/ :
    {
        *(.eh_frame*)
        *(.comment)
        *(.note*)
    }
}

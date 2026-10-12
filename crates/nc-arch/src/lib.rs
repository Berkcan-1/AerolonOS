#![no_std]

use core::arch::asm;

pub mod serial;
pub mod time;

pub unsafe fn disable_pic() {
    outb(0x21, 0xFF); // Master PIC tüm kesmeleri maskele
    outb(0xA1, 0xFF); // Slave PIC tüm kesmeleri maskele
}

pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

pub unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
}

pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

pub unsafe fn outl(port: u16, value: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
}

pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    value
}

pub fn halt_forever() -> ! {
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

pub fn reboot() -> ! {
    unsafe {
        outb(0x0CF9, 0x02);
        outb(0x0CF9, 0x06);
        let mut temp: u8 = 0x02;
        let mut spins = 0u32;
        while temp & 0x02 != 0 && spins < 100_000 {
            temp = inb(0x64);
            spins += 1;
        }
        outb(0x64, 0xFE);
    }
    loop {
        core::hint::spin_loop();
    }
}

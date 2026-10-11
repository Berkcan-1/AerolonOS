#![no_std]
#![no_main]

mod entry;
mod platform;

use core::panic::PanicInfo;

use nc_arch::halt_forever;
use nc_gfx::{present, Canvas};
use nc_hal::{log, BootInfo, Platform};

const BACK_BUFFER_ADDRESS: usize = 0x0040_0000;
const MAX_PIXELS: usize = 1920 * 1200;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    let mut active = platform::create();
    active.init();
    log!(&active, "NetizenCore cekirdegi basladi, platform: {}", active.name());

    let boot = match unsafe { BootInfo::read_from_boot_address() } {
        Some(boot) => boot,
        None => {
            log!(&active, "boot bilgisi gecersiz");
            halt_forever()
        }
    };

    let framebuffer = boot.framebuffer;
    log!(
        &active,
        "ekran {}x{} adres {:#x} pitch {}",
        framebuffer.width,
        framebuffer.height,
        framebuffer.address,
        framebuffer.pitch
    );

    let width = framebuffer.width as usize;
    let height = framebuffer.height as usize;
    let pixel_count = width * height;
    if pixel_count == 0 || pixel_count > MAX_PIXELS {
        log!(&active, "desteklenmeyen ekran boyutu");
        halt_forever()
    }

    let pixels = unsafe { core::slice::from_raw_parts_mut(BACK_BUFFER_ADDRESS as *mut u32, pixel_count) };
    let mut canvas = match Canvas::new(pixels, width, height) {
        Some(canvas) => canvas,
        None => halt_forever(),
    };

    nc_splash::draw(&mut canvas);
    unsafe {
        present(&canvas, &framebuffer);
    }
    log!(&active, "acilis ekrani cizildi");

    halt_forever()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut active = platform::create();
    active.init();
    log!(&active, "PANIC: {}", info);
    halt_forever()
}

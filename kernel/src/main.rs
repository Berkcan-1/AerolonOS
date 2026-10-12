#![no_std]
#![no_main]

extern crate alloc;

mod devices;
mod entry;
mod platform;
mod prepare;

use alloc::boxed::Box;
use core::panic::PanicInfo;
use devices::DeviceManager;
use nc_arch::halt_forever;
use nc_arch::time::Clock;
use nc_desktop::{DesktopAction, DesktopState};
use nc_gfx::compose::lerp_canvases;
use nc_gfx::{present, present_region, Canvas, Rect};
use nc_hal::{log, BootInfo, DeviceType, Platform};
use nc_math::{ratio256, smoothstep256};
use nc_mem::Arena;
use nc_setup::{SetupAction, SetupState};
use nc_splash::{Phase, Splash, SplashOptions};



extern "C" {
    static __kernel_bin_start: u8;
    static __kernel_bin_end: u8;
}

#[global_allocator]
static ALLOCATOR: nc_mem::LockedHeap = nc_mem::LockedHeap::empty();

const ARENA_BASE: usize = 0x0040_0000;
const ARENA_SIZE: usize = 0x03C0_0000;
const HEAP_SIZE: usize = 8 * 1024 * 1024;
const FRAME_MS: u32 = 16;
const TRANSITION_MS: u32 = 600;

fn new_canvas(arena: &mut Arena, width: usize, height: usize) -> Option<Canvas<'static>> {
    let pixels = arena.alloc_pixels(width * height)?;
    Canvas::new(pixels, width, height)
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    unsafe {
        core::arch::asm!("cli");
        nc_arch::disable_pic();
    }
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
    let mut clock = Clock::calibrate();
    let width = framebuffer.width as usize;
    let height = framebuffer.height as usize;
    let mut arena = unsafe { Arena::new(ARENA_BASE, ARENA_SIZE) };

    let heap_mem = match arena.alloc_bytes(HEAP_SIZE) {
        Some(mem) => mem,
        None => {
            log!(&active, "heap bellegi ayrilamadi");
            halt_forever()
        }
    };
    unsafe {
        ALLOCATOR.init(heap_mem.as_mut_ptr() as usize, heap_mem.len());
    }
    log!(&active, "dinamik heap baslatildi (8 MB)");

    let mut is_installed = false;
    log!(&active, "DEBUG: ATA kanali taranacak...");
    if let Some(mut dev) = nc_storage::scan_channel(0) {
        log!(&active, "DEBUG: Kanal 0 bulundu, sektor 0 okunuyor...");
        let installed = nc_storage::is_installed(&mut dev);
        log!(&active, "DEBUG: is_installed sonucu: {}", if installed { "EVET" } else { "HAYIR" });
        if installed {
            is_installed = true;
        }
    } else {
        log!(&active, "DEBUG: Kanal 0 tespit edilemedi.");
    }

  
    let mut screen = match new_canvas(&mut arena, width, height) {
        Some(canvas) => canvas,
        None => halt_forever(),
    };
    let mut snapshot = match new_canvas(&mut arena, width, height) {
        Some(canvas) => canvas,
        None => halt_forever(),
    };
    let mut work_screen = match new_canvas(&mut arena, width, height) {
        Some(canvas) => canvas,
        None => halt_forever(),
    };

    nc_ui::draw_backdrop(&mut screen);
    unsafe {
        present(&screen, &framebuffer);
    }

    let splash_opts = if is_installed {
        log!(&active, "sistem diskte kurulu algilandi (kurulum atlaniyor)");
        SplashOptions { subtitle: None }
    } else {
        log!(&active, "kurulum gerekli, sihirbaz baslatilacak");
        SplashOptions { subtitle: Some("PREPARING SETUP") }
    };

    let mut splash = match Splash::new(&screen, &mut arena, splash_opts) {
        Some(splash) => splash,
        None => halt_forever(),
    };

    let mut preparation = prepare::Preparation::new(clock.now_ms());
    loop {
        let now = clock.now_ms();
        if preparation.poll(now) {
            splash.request_finish();
        }
        let update = splash.frame(&mut screen, now);
        for index in 0..update.count {
            unsafe {
                present_region(&screen, &framebuffer, update.rects[index]);
            }
        }
        if splash.phase() == Phase::Done {
            break;
        }
        clock.sleep_until(now + FRAME_MS);
    }

    let mut device_manager = DeviceManager::new();
    let disks = preparation.take_disks();
    for dev in disks {
        device_manager.register_block_device(dev);
    }

    nc_input::init();
    let ps2 = nc_input::Ps2Input::new(width as i32 / 2, height as i32 / 2);
    device_manager.register_input_device(Box::new(ps2));

    if !is_installed {
        let block_infos = device_manager.block_infos();
        let mut setup_state = SetupState::new(&block_infos);

        snapshot.copy_rect_from(&screen, 0, 0, 0, 0, width as i32, height as i32);
        let (mouse, _) = device_manager.poll_input(width as i32, height as i32);
        nc_setup::draw(&mut work_screen, active.name(), &setup_state, &mouse);

        let transition_start = clock.now_ms();
        loop {
            let now = clock.now_ms();
            let progress = ratio256(now.wrapping_sub(transition_start), TRANSITION_MS);
            lerp_canvases(&mut screen, &snapshot, &work_screen, smoothstep256(progress));
            unsafe {
                present_region(&screen, &framebuffer, Rect::new(0, 0, width as i32, height as i32));
            }
            if progress >= 256 {
                break;
            }
            clock.sleep_until(now + FRAME_MS);
        }

        let mut written_partitions = false;
        loop {
            let now = clock.now_ms();
            let (mouse, key) = device_manager.poll_input(width as i32, height as i32);
            let action = setup_state.update(&mouse, key, width as i32, height as i32);

            if setup_state.step == 2 && !written_partitions {
                written_partitions = true;
                let stage2_slice = unsafe { core::slice::from_raw_parts(0x8800 as *const u8, 4096) };
                let kernel_bytes = unsafe {
                    (&__kernel_bin_end as *const u8 as usize) - (&__kernel_bin_start as *const u8 as usize)
                };
                let kernel_slice = unsafe { core::slice::from_raw_parts(0x100000 as *const u8, kernel_bytes) };
                
                // Tüm blok cihazları yerine sadece seçilen hedef sabit diske (Kanal 0) yazın:
                if let Some(mut ata_dev) = nc_storage::scan_channel(0) {
                    if ata_dev.info.device_type == DeviceType::HardDisk {
                        if nc_storage::install_to_disk(&mut ata_dev, stage2_slice, kernel_slice).is_ok() {
                            log!(&active, "Sabit disk: MBR, stage2 ve kernel basariyla yazildi!");
                        } else {
                            log!(&active, "Sabit disk: Kurulum yazma hatasi!");
                        }
                      }
                    }
                }
               
            

            nc_setup::draw(&mut work_screen, active.name(), &setup_state, &mouse);
            screen.copy_rect_from(&work_screen, 0, 0, 0, 0, width as i32, height as i32);
            nc_ui::draw_cursor(&mut screen, mouse.x, mouse.y);

            unsafe {
                present(&screen, &framebuffer);
            }

            if action == SetupAction::Reboot {
                log!(&active, "kurulum bitti, diskten baslatmak icin yeniden baslatiliyor");
                active.reboot();
            }

            clock.sleep_until(now + FRAME_MS);
        }
    }

    snapshot.copy_rect_from(&screen, 0, 0, 0, 0, width as i32, height as i32);
    let mut desktop_state = DesktopState::new();
    let (mouse, _) = device_manager.poll_input(width as i32, height as i32);
    nc_desktop::draw(&mut work_screen, active.name(), &desktop_state, &mouse, 0);

    let desktop_trans_start = clock.now_ms();
    loop {
        let now = clock.now_ms();
        let progress = ratio256(now.wrapping_sub(desktop_trans_start), TRANSITION_MS);
        lerp_canvases(&mut screen, &snapshot, &work_screen, smoothstep256(progress));
        unsafe {
            present_region(&screen, &framebuffer, Rect::new(0, 0, width as i32, height as i32));
        }
        if progress >= 256 {
            break;
        }
        clock.sleep_until(now + FRAME_MS);
    }

    let desktop_start_time = clock.now_ms();
    loop {
        let now = clock.now_ms();
        let (mouse, key) = device_manager.poll_input(width as i32, height as i32);
        let action = desktop_state.update(&mouse, key, width as i32, height as i32);

        let elapsed_secs = now.wrapping_sub(desktop_start_time) / 1000;
        nc_desktop::draw(&mut work_screen, active.name(), &desktop_state, &mouse, elapsed_secs);
        screen.copy_rect_from(&work_screen, 0, 0, 0, 0, width as i32, height as i32);
        nc_ui::draw_cursor(&mut screen, mouse.x, mouse.y);

        unsafe {
            present(&screen, &framebuffer);
        }

        match action {
            DesktopAction::Shutdown => {
                log!(&active, "kullanici kapatti, platform kapatiliyor");
                active.shutdown(0);
            }
            DesktopAction::Reboot => {
                log!(&active, "kullanici yeniden baslatti, sistem reboot ediliyor");
                active.reboot();
            }
            DesktopAction::None => {}
        }

        clock.sleep_until(now + FRAME_MS);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut active = platform::create();
    active.init();
    log!(&active, "PANIC: {}", info);
    halt_forever()
}

#![no_std]
#![no_main]

extern crate alloc;

mod devices;
mod entry;
mod platform;
mod prepare;
mod process;

use alloc::boxed::Box;
use core::panic::PanicInfo;
use devices::DeviceManager;
use process::ProcessManager;
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
const FRAME_MS: u32 = 16;
const TRANSITION_MS: u32 = 600;

fn new_canvas(arena: &mut Arena, width: usize, height: usize) -> Option<Canvas<'static>> {
    let pixels = arena.alloc_pixels(width * height)?;
    Canvas::new(pixels, width, height)
}

fn detect_ram_bounds(boot: &BootInfo) -> (usize, usize) {
    let mut total_usable: u64 = 0;
    let mut max_usable_end: u64 = 0;
    for region in boot.memory_regions() {
        if region.is_usable() {
            total_usable = total_usable.saturating_add(region.length);
            let end = region.base.saturating_add(region.length);
            if region.base <= 0x0040_0000 && end > 0x0040_0000 {
                if end > max_usable_end {
                    max_usable_end = end;
                }
            }
        }
    }
    if max_usable_end == 0 {
        max_usable_end = 64 * 1024 * 1024;
    }
    if total_usable == 0 {
        total_usable = max_usable_end;
    }
    let total_mb = (total_usable / (1024 * 1024)).max(64) as usize;
    let usable_limit = (max_usable_end.min(0xC000_0000)) as usize;
    (total_mb, usable_limit)
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

    let (total_ram_mb, usable_limit) = detect_ram_bounds(&boot);
    let total_avail = usable_limit.saturating_sub(ARENA_BASE);
    let canvas_bytes = width * height * 4;
    let arena_size = (canvas_bytes * 10).max(28 * 1024 * 1024).min(total_avail / 2);
    let heap_size = total_avail.saturating_sub(arena_size);
    let heap_base = ARENA_BASE + arena_size;

    let mut arena = unsafe { Arena::new(ARENA_BASE, arena_size) };
    unsafe {
        ALLOCATOR.init(heap_base, heap_size);
    }
    log!(
        &active,
        "Dinamik bellek kuruldu: {} MB RAM (Arena: {} MB, Heap: {} MB)",
        total_ram_mb,
        arena_size / (1024 * 1024),
        heap_size / (1024 * 1024)
    );

let drv_reg = nc_driver::init();
    log!(&active, "{}", drv_reg.status_log);

    let is_installed = nc_storage::find_installed_disk().is_some();
    
   
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
    let mut wallpaper = match new_canvas(&mut arena, width, height) {
        Some(canvas) => canvas,
        None => halt_forever(),
    };

    nc_ui::draw_backdrop(&mut screen);
    unsafe {
        present(&screen, &framebuffer);
    }

    let splash_opts = if is_installed {
        SplashOptions { subtitle: None }
    } else {
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

        // kernel/src/main.rs içindeki kurulum döngüsü:
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
                if let Some(mut ata_dev) = nc_storage::find_install_target_disk(Some(setup_state.selected_disk as u8)) {
                    let _ = nc_storage::install_to_disk(&mut ata_dev, stage2_slice, kernel_slice);
                }
            }
            nc_setup::draw(&mut work_screen, active.name(), &setup_state, &mouse);
            screen.copy_rect_from(&work_screen, 0, 0, 0, 0, width as i32, height as i32);
            nc_ui::draw_cursor(&mut screen, mouse.x, mouse.y);
            unsafe {
                present(&screen, &framebuffer);
            }
            if action == SetupAction::Reboot {
                active.reboot();
            }
            clock.sleep_until(now + FRAME_MS);
        }
    }

    snapshot.copy_rect_from(&screen, 0, 0, 0, 0, width as i32, height as i32);
    let mut desktop_state = DesktopState::new(active.name(), total_ram_mb as u32, nc_apps::builtin());
    nc_desktop::draw_wallpaper(&mut wallpaper, desktop_state.wallpaper_index());
    desktop_state.take_wallpaper_dirty();
    nc_desktop::draw(&mut work_screen, &wallpaper, &desktop_state);
    desktop_state.take_dirty();

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

    let cursor_width = nc_ui::CURSOR_WIDTH as i32;
    let cursor_height = nc_ui::CURSOR_HEIGHT as i32;
    let desktop_start_time = clock.now_ms();
    let (first_mouse, _) = device_manager.poll_input(width as i32, height as i32);
    screen.copy_rect_from(&work_screen, 0, 0, 0, 0, width as i32, height as i32);
    nc_ui::draw_cursor(&mut screen, first_mouse.x, first_mouse.y);
    unsafe {
        present(&screen, &framebuffer);
    }

    let mut process_manager = ProcessManager::new();
    let mut last_cursor = (first_mouse.x, first_mouse.y);

    loop {
        let now = clock.now_ms();
        process_manager.poll_all();
        let (mouse, key) = device_manager.poll_input(width as i32, height as i32);
        let uptime_ms = now.wrapping_sub(desktop_start_time);
        let action = desktop_state.update(&mouse, key, width as i32, height as i32, uptime_ms);

        if desktop_state.take_wallpaper_dirty() {
            nc_desktop::draw_wallpaper(&mut wallpaper, desktop_state.wallpaper_index());
        }

        if desktop_state.take_dirty() {
            nc_desktop::draw(&mut work_screen, &wallpaper, &desktop_state);
            screen.copy_rect_from(&work_screen, 0, 0, 0, 0, width as i32, height as i32);
            nc_ui::draw_cursor(&mut screen, mouse.x, mouse.y);
            unsafe {
                present(&screen, &framebuffer);
            }
            last_cursor = (mouse.x, mouse.y);
        } else if (mouse.x, mouse.y) != last_cursor {
            screen.copy_rect_from(&work_screen, last_cursor.0, last_cursor.1, last_cursor.0, last_cursor.1, cursor_width, cursor_height);
            nc_ui::draw_cursor(&mut screen, mouse.x, mouse.y);
            unsafe {
                present_region(&screen, &framebuffer, Rect::new(last_cursor.0, last_cursor.1, cursor_width, cursor_height));
                present_region(&screen, &framebuffer, Rect::new(mouse.x, mouse.y, cursor_width, cursor_height));
            }
            last_cursor = (mouse.x, mouse.y);
        }

        match action {
            DesktopAction::Shutdown => {
                active.shutdown(0);
            }
            DesktopAction::Reboot => {
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

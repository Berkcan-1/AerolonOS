#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;
use nc_hal::{BootInfo, FramebufferInfo, MemoryRegion, BOOT_INFO_ADDRESS, BOOT_INFO_MAGIC, BOOT_INFO_VERSION};

type EfiStatus = usize;
type EfiHandle = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x9042a9de,
    data2: 0x23dc,
    data3: 0x4a38,
    data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[repr(C)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct EfiSystemTable {
    pub hdr: EfiTableHeader,
    pub firmware_vendor: *const u16,
    pub firmware_revision: u32,
    pub console_in_handle: EfiHandle,
    pub con_in: *mut c_void,
    pub console_out_handle: EfiHandle,
    pub con_out: *mut c_void,
    pub standard_error_handle: EfiHandle,
    pub std_err: *mut c_void,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut EfiBootServices,
}

#[repr(C)]
pub struct EfiMemoryDescriptor {
    pub r#type: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,
    pub raise_tpl: usize,
    pub restore_tpl: usize,
    pub allocate_pages: usize,
    pub free_pages: usize,
    pub get_memory_map: unsafe extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut EfiMemoryDescriptor,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EfiStatus,
    pub allocate_pool: usize,
    pub free_pool: usize,
    pub create_event: usize,
    pub set_timer: usize,
    pub wait_for_event: usize,
    pub signal_event: usize,
    pub close_event: usize,
    pub check_event: usize,
    pub install_protocol_interface: usize,
    pub reinstall_protocol_interface: usize,
    pub uninstall_protocol_interface: usize,
    pub handle_protocol: usize,
    pub reserved: usize,
    pub register_protocol_notify: usize,
    pub locate_handle: usize,
    pub locate_device_path: usize,
    pub install_configuration_table: usize,
    pub load_image: usize,
    pub start_image: usize,
    pub exit: usize,
    pub unload_image: usize,
    pub exit_boot_services: unsafe extern "efiapi" fn(image_handle: EfiHandle, map_key: usize) -> EfiStatus,
    pub get_next_monotonic_count: usize,
    pub stall: usize,
    pub set_watchdog_timer: usize,
    pub connect_controller: usize,
    pub disconnect_controller: usize,
    pub open_protocol: usize,
    pub close_protocol: usize,
    pub open_protocol_information: usize,
    pub protocols_per_handle: usize,
    pub locate_handle_buffer: usize,
    pub locate_protocol: unsafe extern "efiapi" fn(
        protocol: *const EfiGuid,
        registration: *mut c_void,
        interface: *mut *mut c_void,
    ) -> EfiStatus,
}

#[repr(C)]
pub struct EfiGopModeInfo {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: u32,
    pub pixel_information: [u32; 4],
    pub pixels_per_scan_line: u32,
}

#[repr(C)]
pub struct EfiGopMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *mut EfiGopModeInfo,
    pub size_of_info: usize,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: usize,
}

#[repr(C)]
pub struct EfiGraphicsOutputProtocol {
    pub query_mode: unsafe extern "efiapi" fn(
        this: *mut EfiGraphicsOutputProtocol,
        mode_number: u32,
        size_of_info: *mut usize,
        info: *mut *mut EfiGopModeInfo,
    ) -> EfiStatus,
    pub set_mode: unsafe extern "efiapi" fn(this: *mut EfiGraphicsOutputProtocol, mode_number: u32) -> EfiStatus,
    pub blt: usize,
    pub mode: *mut EfiGopMode,
}

static KERNEL_BLOB: &[u8] = include_bytes!(env!("NC_KERNEL_BIN"));

#[no_mangle]
pub unsafe extern "efiapi" fn efi_main(image_handle: EfiHandle, system_table: *mut EfiSystemTable) -> EfiStatus {
    let bs = (*system_table).boot_services;

    let mut gop_ptr: *mut c_void = core::ptr::null_mut();
    let status = ((*bs).locate_protocol)(
        &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
        core::ptr::null_mut(),
        &mut gop_ptr,
    );
    if status != 0 || gop_ptr.is_null() {
        loop {
            core::arch::asm!("hlt");
        }
    }

    let gop = gop_ptr as *mut EfiGraphicsOutputProtocol;
    let max_mode = (*(*gop).mode).max_mode;
    let mut best_mode: Option<u32> = None;

    for m in 0..max_mode {
        let mut info_size = 0usize;
        let mut info_ptr: *mut EfiGopModeInfo = core::ptr::null_mut();
        if ((*gop).query_mode)(gop, m, &mut info_size, &mut info_ptr) == 0 && !info_ptr.is_null() {
            let w = (*info_ptr).horizontal_resolution;
            let h = (*info_ptr).vertical_resolution;
            let fmt = (*info_ptr).pixel_format;
            if (fmt == 0 || fmt == 1) && w == 1280 && h == 720 {
                best_mode = Some(m);
                break;
            } else if (fmt == 0 || fmt == 1) && w == 1024 && h == 768 && best_mode.is_none() {
                best_mode = Some(m);
            }
        }
    }

    if let Some(m) = best_mode {
        let _ = ((*gop).set_mode)(gop, m);
    }

    let mode = (*gop).mode;
    let info = (*mode).info;
    let is_bgr = (*info).pixel_format == 1;

    let fb_info = FramebufferInfo {
        address: (*mode).frame_buffer_base,
        pitch: (*info).pixels_per_scan_line * 4,
        width: (*info).horizontal_resolution,
        height: (*info).vertical_resolution,
        bpp: 32,
        red_position: if is_bgr { 16 } else { 0 },
        red_size: 8,
        green_position: 8,
        green_size: 8,
        blue_position: if is_bgr { 0 } else { 16 },
        blue_size: 8,
        reserved: 0,
    };

    let mut mmap_buf = [0u8; 16384];
    let mut mmap_size = mmap_buf.len();
    let mut map_key = 0usize;
    let mut desc_size = 0usize;
    let mut desc_ver = 0u32;

    let _ = ((*bs).get_memory_map)(
        &mut mmap_size,
        mmap_buf.as_mut_ptr() as *mut EfiMemoryDescriptor,
        &mut map_key,
        &mut desc_size,
        &mut desc_ver,
    );
    let _ = ((*bs).exit_boot_services)(image_handle, map_key);

    core::arch::asm!("cli");

    let e820_target = 0x0A00 as *mut MemoryRegion;
    let mut region_count = 0u32;
    if desc_size >= core::mem::size_of::<EfiMemoryDescriptor>() {
        let count = mmap_size / desc_size;
        for i in 0..count {
            if region_count >= 32 {
                break;
            }
            let d = &*(mmap_buf.as_ptr().add(i * desc_size) as *const EfiMemoryDescriptor);
            let r_type = if d.r#type == 7 || d.r#type == 3 || d.r#type == 4 { 1 } else { 2 };
            core::ptr::write(
                e820_target.add(region_count as usize),
                MemoryRegion {
                    base: d.physical_start,
                    length: d.number_of_pages * 4096,
                    region_type: r_type,
                    acpi_extended: 1,
                },
            );
            region_count += 1;
        }
    }

    let boot_info = BootInfo {
        magic: BOOT_INFO_MAGIC,
        version: BOOT_INFO_VERSION,
        framebuffer: fb_info,
        memory_map_addr: 0x0A00,
        memory_map_count: region_count,
    };
    core::ptr::write(BOOT_INFO_ADDRESS as *mut BootInfo, boot_info);

    let kernel_dst = 0x0010_0000 as *mut u8;
    core::ptr::copy_nonoverlapping(KERNEL_BLOB.as_ptr(), kernel_dst, KERNEL_BLOB.len());

    core::arch::asm!(
        "mov rsp, 0x90000",
        "mov rax, 0x100000",
        "jmp rax",
        options(noreturn)
    );
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

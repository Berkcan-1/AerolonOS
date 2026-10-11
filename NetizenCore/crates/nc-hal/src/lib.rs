#![no_std]

use core::fmt;

pub const BOOT_INFO_ADDRESS: usize = 0x0900;
pub const BOOT_INFO_MAGIC: u32 = 0x4942_434E;
pub const BOOT_INFO_VERSION: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pub address: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub red_position: u8,
    pub red_size: u8,
    pub green_position: u8,
    pub green_size: u8,
    pub blue_position: u8,
    pub blue_size: u8,
    pub reserved: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootInfo {
    pub magic: u32,
    pub version: u32,
    pub framebuffer: FramebufferInfo,
}

impl BootInfo {
    pub unsafe fn read_from_boot_address() -> Option<BootInfo> {
        let info = core::ptr::read(BOOT_INFO_ADDRESS as *const BootInfo);
        if info.magic == BOOT_INFO_MAGIC && info.version == BOOT_INFO_VERSION {
            Some(info)
        } else {
            None
        }
    }
}

pub trait Platform {
    fn name(&self) -> &'static str;
    fn init(&mut self);
    fn write_log(&self, text: &str);
    fn shutdown(&self, code: u8) -> !;
}

pub struct LogWriter<'a, P: Platform>(pub &'a P);

impl<'a, P: Platform> fmt::Write for LogWriter<'a, P> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write_log(text);
        Ok(())
    }
}

#[macro_export]
macro_rules! log {
    ($platform:expr, $($argument:tt)*) => {{
        use core::fmt::Write as _;
        let mut writer = $crate::LogWriter($platform);
        let _ = writeln!(writer, $($argument)*);
    }};
}

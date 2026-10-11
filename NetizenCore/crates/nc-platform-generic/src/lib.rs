#![no_std]

use nc_arch::{halt_forever, serial};
use nc_hal::Platform;

pub struct GenericPlatform;

impl GenericPlatform {
    pub const fn new() -> GenericPlatform {
        GenericPlatform
    }
}

impl Platform for GenericPlatform {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn init(&mut self) {
        serial::init();
    }

    fn write_log(&self, text: &str) {
        if serial::present() {
            serial::write_str(text);
        }
    }

    fn shutdown(&self, _code: u8) -> ! {
        halt_forever()
    }
}

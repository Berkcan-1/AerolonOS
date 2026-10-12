use alloc::boxed::Box;
use alloc::vec::Vec;
use nc_hal::BlockDevice;
use nc_storage::scan_channel;

const MIN_DURATION_MS: u32 = 4500;

pub struct Preparation {
    started: u32,
    finished: bool,
    scanned_channels: u8,
    disks: Vec<Box<dyn BlockDevice>>,
}

impl Preparation {
    pub fn new(now: u32) -> Preparation {
        Preparation {
            started: now,
            finished: false,
            scanned_channels: 0,
            disks: Vec::new(),
        }
    }

    pub fn poll(&mut self, now: u32) -> bool {
        let elapsed = now.wrapping_sub(self.started);
        if self.scanned_channels < 4 {
            let target_time = (self.scanned_channels as u32 + 1) * 600;
            if elapsed >= target_time {
                if let Some(dev) = scan_channel(self.scanned_channels) {
                    self.disks.push(Box::new(dev));
                }
                self.scanned_channels += 1;
            }
        }
        if !self.finished && self.scanned_channels >= 4 && elapsed >= MIN_DURATION_MS {
            self.finished = true;
        }
        self.finished
    }

    pub fn take_disks(&mut self) -> Vec<Box<dyn BlockDevice>> {
        core::mem::take(&mut self.disks)
    }
}

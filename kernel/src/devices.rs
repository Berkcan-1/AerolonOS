use alloc::boxed::Box;
use alloc::vec::Vec;
use nc_hal::{BlockDevice, BlockDeviceInfo, InputDevice, KeyCode, MouseState};

pub struct DeviceManager {
    block_devices: Vec<Box<dyn BlockDevice>>,
    input_devices: Vec<Box<dyn InputDevice>>,
}

impl DeviceManager {
    pub fn new() -> DeviceManager {
        DeviceManager {
            block_devices: Vec::new(),
            input_devices: Vec::new(),
        }
    }

    pub fn register_block_device(&mut self, device: Box<dyn BlockDevice>) {
        self.block_devices.push(device);
    }

    #[allow(dead_code)]
    pub fn block_count(&self) -> usize {
        self.block_devices.len()
    }

    #[allow(dead_code)]
    pub fn block_device(&mut self, index: usize) -> Option<&mut (dyn BlockDevice + 'static)> {
        self.block_devices.get_mut(index).map(|d| d.as_mut())
    }

    pub fn block_infos(&self) -> Vec<BlockDeviceInfo> {
        let mut infos = Vec::with_capacity(self.block_devices.len());
        for dev in &self.block_devices {
            infos.push(dev.info());
        }
        infos
    }

    pub fn register_input_device(&mut self, input: Box<dyn InputDevice>) {
        self.input_devices.push(input);
    }

    pub fn poll_input(&mut self, width: i32, height: i32) -> (MouseState, Option<KeyCode>) {
        let mut mouse = MouseState {
            x: width / 2,
            y: height / 2,
            left_down: false,
            left_clicked: false,
            right_down: false,
            right_clicked: false,
        };
        let mut key = None;
        for input in &mut self.input_devices {
            let (m, k) = input.poll(width, height);
            mouse = m;
            if k.is_some() {
                key = k;
            }
        }
        (mouse, key)
    }
}

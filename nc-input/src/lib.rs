#![no_std]

use nc_arch::{inb, outb};
use nc_hal::{InputDevice, KeyCode, MouseState};

pub struct Ps2Input {
    mouse_x: i32,
    mouse_y: i32,
    mouse_left_down: bool,
    mouse_right_down: bool,
    mouse_packet: [u8; 3],
    mouse_cycle: usize,
    last_key: Option<KeyCode>,
    extended: bool,
}

fn wait_write() {
    let mut spins = 0u32;
    while spins < 100_000 {
        if unsafe { inb(0x64) } & 0x02 == 0 {
            return;
        }
        spins += 1;
    }
}

fn wait_read() -> bool {
    let mut spins = 0u32;
    while spins < 100_000 {
        if unsafe { inb(0x64) } & 0x01 != 0 {
            return true;
        }
        spins += 1;
    }
    false
}

pub fn init() {
    wait_write();
    unsafe { outb(0x64, 0xA8); }
    wait_write();
    unsafe { outb(0x64, 0x20); }
    if wait_read() {
        let mut status = unsafe { inb(0x60) };
        status |= 0x02;
        status &= !0x20;
        wait_write();
        unsafe { outb(0x64, 0x60); }
        wait_write();
        unsafe { outb(0x60, status); }
    }
    wait_write();
    unsafe { outb(0x64, 0xD4); }
    wait_write();
    unsafe { outb(0x60, 0xF6); }
    if wait_read() {
        let _ = unsafe { inb(0x60) };
    }
    wait_write();
    unsafe { outb(0x64, 0xD4); }
    wait_write();
    unsafe { outb(0x60, 0xF4); }
    if wait_read() {
        let _ = unsafe { inb(0x60) };
    }
}

impl Ps2Input {
    pub const fn new(initial_x: i32, initial_y: i32) -> Ps2Input {
        Ps2Input {
            mouse_x: initial_x,
            mouse_y: initial_y,
            mouse_left_down: false,
            mouse_right_down: false,
            mouse_packet: [0; 3],
            mouse_cycle: 0,
            last_key: None,
            extended: false,
        }
    }

    pub fn poll_state(&mut self, screen_width: i32, screen_height: i32) -> (MouseState, Option<KeyCode>) {
        let mut left_clicked = false;
        let mut right_clicked = false;
        self.last_key = None;

        while unsafe { inb(0x64) } & 0x01 != 0 {
            let status = unsafe { inb(0x64) };
            let data = unsafe { inb(0x60) };

            if status & 0x20 != 0 {
                match self.mouse_cycle {
                    0 => {
                        if data & 0x08 != 0 {
                            self.mouse_packet[0] = data;
                            self.mouse_cycle = 1;
                        }
                    }
                    1 => {
                        self.mouse_packet[1] = data;
                        self.mouse_cycle = 2;
                    }
                    2 => {
                        self.mouse_packet[2] = data;
                        self.mouse_cycle = 0;
                        let flags = self.mouse_packet[0];
                        let raw_dx = self.mouse_packet[1] as i32;
                        let raw_dy = self.mouse_packet[2] as i32;
                        let dx = if flags & 0x10 != 0 { raw_dx | !0xFF } else { raw_dx };
                        let dy = if flags & 0x20 != 0 { raw_dy | !0xFF } else { raw_dy };

                        self.mouse_x = (self.mouse_x + dx).clamp(0, screen_width - 1);
                        self.mouse_y = (self.mouse_y - dy).clamp(0, screen_height - 1);

                        let l_down = flags & 0x01 != 0;
                        if l_down && !self.mouse_left_down {
                            left_clicked = true;
                        }
                        self.mouse_left_down = l_down;

                        let r_down = flags & 0x02 != 0;
                        if r_down && !self.mouse_right_down {
                            right_clicked = true;
                        }
                        self.mouse_right_down = r_down;
                    }
                    _ => self.mouse_cycle = 0,
                }
            } else {
                if data == 0xE0 {
                    self.extended = true;
                    continue;
                }
                let release = data & 0x80 != 0;
                let code = data & 0x7F;
                let ext = self.extended;
                self.extended = false;

                if !release {
                    let key = match (ext, code) {
                        (false, 0x1C) => KeyCode::Enter,
                        (false, 0x01) => KeyCode::Escape,
                        (false, 0x39) => KeyCode::Space,
                        (false, 0x0F) => KeyCode::Tab,
                        (false, 0x0E) => KeyCode::Backspace,
                        (true, 0x48) | (false, 0x48) => KeyCode::Up,
                        (true, 0x50) | (false, 0x50) => KeyCode::Down,
                        (true, 0x4B) | (false, 0x4B) => KeyCode::Left,
                        (true, 0x4D) | (false, 0x4D) => KeyCode::Right,

                        // Number keys
                        (false, 0x02) => KeyCode::Char('1'),
                        (false, 0x03) => KeyCode::Char('2'),
                        (false, 0x04) => KeyCode::Char('3'),
                        (false, 0x05) => KeyCode::Char('4'),
                        (false, 0x06) => KeyCode::Char('5'),
                        (false, 0x07) => KeyCode::Char('6'),
                        (false, 0x08) => KeyCode::Char('7'),
                        (false, 0x09) => KeyCode::Char('8'),
                        (false, 0x0A) => KeyCode::Char('9'),
                        (false, 0x0B) => KeyCode::Char('0'),

                        // Letter keys
                        (false, 0x10) => KeyCode::Char('q'),
                        (false, 0x11) => KeyCode::Char('w'),
                        (false, 0x12) => KeyCode::Char('e'),
                        (false, 0x13) => KeyCode::Char('r'),
                        (false, 0x14) => KeyCode::Char('t'),
                        (false, 0x15) => KeyCode::Char('y'),
                        (false, 0x16) => KeyCode::Char('u'),
                        (false, 0x17) => KeyCode::Char('i'),
                        (false, 0x18) => KeyCode::Char('o'),
                        (false, 0x19) => KeyCode::Char('p'),
                        (false, 0x1E) => KeyCode::Char('a'),
                        (false, 0x1F) => KeyCode::Char('s'),
                        (false, 0x20) => KeyCode::Char('d'),
                        (false, 0x21) => KeyCode::Char('f'),
                        (false, 0x22) => KeyCode::Char('g'),
                        (false, 0x23) => KeyCode::Char('h'),
                        (false, 0x24) => KeyCode::Char('j'),
                        (false, 0x25) => KeyCode::Char('k'),
                        (false, 0x26) => KeyCode::Char('l'),
                        (false, 0x2C) => KeyCode::Char('z'),
                        (false, 0x2D) => KeyCode::Char('x'),
                        (false, 0x2E) => KeyCode::Char('c'),
                        (false, 0x2F) => KeyCode::Char('v'),
                        (false, 0x30) => KeyCode::Char('b'),
                        (false, 0x31) => KeyCode::Char('n'),
                        (false, 0x32) => KeyCode::Char('m'),

                        // Symbols
                        (false, 0x0C) => KeyCode::Char('-'),
                        (false, 0x0D) => KeyCode::Char('='),
                        (false, 0x33) => KeyCode::Char(','),
                        (false, 0x34) => KeyCode::Char('.'),
                        (false, 0x35) => KeyCode::Char('/'),
                        (false, 0x27) => KeyCode::Char(';'),
                        (false, 0x28) => KeyCode::Char('\''),
                        (false, 0x1A) => KeyCode::Char('['),
                        (false, 0x1B) => KeyCode::Char(']'),
                        (false, 0x2B) => KeyCode::Char('\\'),

                        _ => KeyCode::Unknown,
                    };
                    self.last_key = Some(key);
                }
            }
        }

        (
            MouseState {
                x: self.mouse_x,
                y: self.mouse_y,
                left_down: self.mouse_left_down,
                left_clicked,
                right_down: self.mouse_right_down,
                right_clicked,
            },
            self.last_key,
        )
    }
}

impl InputDevice for Ps2Input {
    fn name(&self) -> &'static str {
        "ps2-input"
    }

    fn poll(&mut self, screen_width: i32, screen_height: i32) -> (MouseState, Option<KeyCode>) {
        self.poll_state(screen_width, screen_height)
    }
}

#![no_std]

use nc_hal::FramebufferInfo;

pub mod blur;
pub mod compose;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const EMPTY: Rect = Rect { x: 0, y: 0, width: 0, height: 0 };

    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect { x, y, width, height }
    }
}

pub const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    ((red as u32) << 16) | ((green as u32) << 8) | (blue as u32)
}

pub const fn invert(color: u32) -> u32 {
    color ^ 0x00FF_FFFF
}

fn channel(color: u32, shift: u32) -> u32 {
    (color >> shift) & 0xFF
}

fn mix(from: u32, to: u32, t: u32) -> u32 {
    (from * (255 - t) + to * t + 127) / 255
}

pub fn lerp_color(from: u32, to: u32, t: u32) -> u32 {
    let t = t.min(255);
    (mix(channel(from, 16), channel(to, 16), t) << 16)
        | (mix(channel(from, 8), channel(to, 8), t) << 8)
        | mix(channel(from, 0), channel(to, 0), t)
}

const DITHER: [u32; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

fn dithered_mix(from: u32, to: u32, amount: u32, threshold: u32) -> u32 {
    let mix = |shift: u32| -> u32 {
        ((channel(from, shift) * (256 - amount) + channel(to, shift) * amount + threshold) >> 8).min(255)
    };
    (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

fn add_scaled(destination: u32, color: u32, amount: u32) -> u32 {
    let add = |shift: u32| -> u32 {
        (channel(destination, shift) + channel(color, shift) * amount / 256).min(255)
    };
    (add(16) << 16) | (add(8) << 8) | add(0)
}

pub struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: usize,
    height: usize,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u32], width: usize, height: usize) -> Option<Canvas<'a>> {
        if width == 0 || height == 0 || pixels.len() < width * height {
            return None;
        }
        Some(Canvas { pixels, width, height })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels[..self.width * self.height]
    }

    pub fn pixels_mut(&mut self) -> &mut [u32] {
        let count = self.width * self.height;
        &mut self.pixels[..count]
    }

    pub fn fill(&mut self, color: u32) {
        for pixel in self.pixels_mut().iter_mut() {
            *pixel = color;
        }
    }

    pub fn vertical_gradient(&mut self, top: u32, bottom: u32) {
        let width = self.width;
        let span = (self.height.max(2) - 1) as u32;
        for (row_index, row) in self.pixels_mut().chunks_exact_mut(width).enumerate() {
            let weight = row_index as u32 * 65536 / span;
            let fixed = |shift: u32| -> u32 {
                (channel(top, shift) * (65536 - weight) + channel(bottom, shift) * weight) >> 8
            };
            let (red, green, blue) = (fixed(16), fixed(8), fixed(0));
            for (column, pixel) in row.iter_mut().enumerate() {
                let threshold = DITHER[(row_index & 3) * 4 + (column & 3)] * 16;
                *pixel = (((red + threshold) >> 8).min(255) << 16)
                    | (((green + threshold) >> 8).min(255) << 8)
                    | ((blue + threshold) >> 8).min(255);
            }
        }
    }

    pub fn add_glow(&mut self, center_x: i32, center_y: i32, radius: i32, color: u32, strength: u32) {
        if radius <= 0 {
            return;
        }
        let radius_squared = (radius as i64) * (radius as i64);
        let x_start = (center_x - radius).max(0);
        let x_end = (center_x + radius).min(self.width as i32 - 1);
        let y_start = (center_y - radius).max(0);
        let y_end = (center_y + radius).min(self.height as i32 - 1);
        let strength = strength.min(256);
        for y in y_start..=y_end {
            for x in x_start..=x_end {
                let dx = (x - center_x) as i64;
                let dy = (y - center_y) as i64;
                let distance_squared = dx * dx + dy * dy;
                if distance_squared >= radius_squared {
                    continue;
                }
                let linear = ((radius_squared - distance_squared) * 256 / radius_squared) as u32;
                let falloff = (linear * linear) >> 8;
                let amount = (falloff * strength) >> 8;
                let index = y as usize * self.width + x as usize;
                self.pixels[index] = add_scaled(self.pixels[index], color, amount);
            }
        }
    }

    pub fn blend_glow(&mut self, center_x: i32, center_y: i32, radius: i32, color: u32, strength: u32) {
        if radius <= 0 {
            return;
        }
        let radius_squared = (radius as i64) * (radius as i64);
        let x_start = (center_x - radius).max(0);
        let x_end = (center_x + radius).min(self.width as i32 - 1);
        let y_start = (center_y - radius).max(0);
        let y_end = (center_y + radius).min(self.height as i32 - 1);
        let strength = strength.min(256);
        for y in y_start..=y_end {
            for x in x_start..=x_end {
                let dx = (x - center_x) as i64;
                let dy = (y - center_y) as i64;
                let distance_squared = dx * dx + dy * dy;
                if distance_squared >= radius_squared {
                    continue;
                }
                let linear = ((radius_squared - distance_squared) * 256 / radius_squared) as u32;
                let falloff = (linear * linear) >> 8;
                let amount = (falloff * strength) >> 8;
                let index = y as usize * self.width + x as usize;
                let threshold = DITHER[(y as usize & 3) * 4 + (x as usize & 3)] * 16;
                self.pixels[index] = dithered_mix(self.pixels[index], color, amount.min(256), threshold);
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        let x_start = x.max(0);
        let y_start = y.max(0);
        let x_end = (x + width).min(self.width as i32);
        let y_end = (y + height).min(self.height as i32);
        for row in y_start..y_end {
            for column in x_start..x_end {
                self.pixels[row as usize * self.width + column as usize] = color;
            }
        }
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, color: u32, alpha: u8) {
        if alpha == 0 || x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = y as usize * self.width + x as usize;
        self.pixels[index] = lerp_color(self.pixels[index], color, alpha as u32);
    }

    pub fn draw_rgba(&mut self, x: i32, y: i32, width: u32, height: u32, data: &[u8]) {
        let width = width as usize;
        let height = height as usize;
        if data.len() < width * height * 4 {
            return;
        }
        for row in 0..height {
            for column in 0..width {
                let index = (row * width + column) * 4;
                let alpha = data[index + 3];
                if alpha == 0 {
                    continue;
                }
                let color = rgb(data[index], data[index + 1], data[index + 2]);
                self.blend_pixel(x + column as i32, y + row as i32, color, alpha);
            }
        }
    }

    pub fn copy_rect_from(
        &mut self,
        source: &Canvas<'_>,
        source_x: i32,
        source_y: i32,
        destination_x: i32,
        destination_y: i32,
        width: i32,
        height: i32,
    ) {
        let first = 0.max(-source_x).max(-destination_x);
        let last = width.min(source.width as i32 - source_x).min(self.width as i32 - destination_x);
        if last <= first {
            return;
        }
        let length = (last - first) as usize;
        for row in 0..height {
            let from_y = source_y + row;
            let to_y = destination_y + row;
            if from_y < 0 || to_y < 0 || from_y >= source.height as i32 || to_y >= self.height as i32 {
                continue;
            }
            let from = from_y as usize * source.width + (source_x + first) as usize;
            let to = to_y as usize * self.width + (destination_x + first) as usize;
            self.pixels[to..to + length].copy_from_slice(&source.pixels[from..from + length]);
        }
    }
}

fn pack(color: u32, framebuffer: &FramebufferInfo) -> u32 {
    let place = |value: u32, size: u8, position: u8| -> u32 {
        if size == 0 || size > 8 || position >= 32 {
            0
        } else {
            (value >> (8 - size as u32)) << (position as u32)
        }
    };
    place(channel(color, 16), framebuffer.red_size, framebuffer.red_position)
        | place(channel(color, 8), framebuffer.green_size, framebuffer.green_position)
        | place(channel(color, 0), framebuffer.blue_size, framebuffer.blue_position)
}

pub unsafe fn present_region(canvas: &Canvas, framebuffer: &FramebufferInfo, area: Rect) {
    let limit_x = canvas.width().min(framebuffer.width as usize);
    let limit_y = canvas.height().min(framebuffer.height as usize);
    let x_start = area.x.max(0) as usize;
    let y_start = area.y.max(0) as usize;
    let x_end = ((area.x + area.width).max(0) as usize).min(limit_x);
    let y_end = ((area.y + area.height).max(0) as usize).min(limit_y);
    if x_end <= x_start || y_end <= y_start {
        return;
    }
    let base = framebuffer.address as usize as *mut u8;
    let pitch = framebuffer.pitch as usize;
    let native = framebuffer.bpp == 32
        && framebuffer.red_position == 16
        && framebuffer.red_size == 8
        && framebuffer.green_position == 8
        && framebuffer.green_size == 8
        && framebuffer.blue_position == 0
        && framebuffer.blue_size == 8;
    let source = canvas.pixels();
    let stride = canvas.width();
    for y in y_start..y_end {
        let row = base.add(y * pitch) as *mut u32;
        let source_row = &source[y * stride + x_start..y * stride + x_end];
        for (offset, value) in source_row.iter().enumerate() {
            let output = if native { *value } else { pack(*value, framebuffer) };
            core::ptr::write_volatile(row.add(x_start + offset), output);
        }
    }
}

pub unsafe fn present(canvas: &Canvas, framebuffer: &FramebufferInfo) {
    present_region(
        canvas,
        framebuffer,
        Rect::new(0, 0, canvas.width() as i32, canvas.height() as i32),
    );
}

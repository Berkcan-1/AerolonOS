#![no_std]

use nc_hal::FramebufferInfo;

pub const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    ((red as u32) << 16) | ((green as u32) << 8) | (blue as u32)
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

    pub fn fill(&mut self, color: u32) {
        let count = self.width * self.height;
        for pixel in self.pixels[..count].iter_mut() {
            *pixel = color;
        }
    }

    pub fn vertical_gradient(&mut self, top: u32, bottom: u32) {
        let width = self.width;
        let height = self.height;
        let span = (height.max(2) - 1) as u32;
        for (row_index, row) in self.pixels[..width * height].chunks_exact_mut(width).enumerate() {
            let t = (row_index as u32 * 255) / span;
            let color = lerp_color(top, bottom, t);
            for pixel in row.iter_mut() {
                *pixel = color;
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

pub unsafe fn present(canvas: &Canvas, framebuffer: &FramebufferInfo) {
    let base = framebuffer.address as usize as *mut u8;
    let pitch = framebuffer.pitch as usize;
    let width = canvas.width().min(framebuffer.width as usize);
    let height = canvas.height().min(framebuffer.height as usize);
    let native = framebuffer.bpp == 32
        && framebuffer.red_position == 16
        && framebuffer.red_size == 8
        && framebuffer.green_position == 8
        && framebuffer.green_size == 8
        && framebuffer.blue_position == 0
        && framebuffer.blue_size == 8;
    let source = canvas.pixels();
    for y in 0..height {
        let row = base.add(y * pitch) as *mut u32;
        let source_row = &source[y * canvas.width()..y * canvas.width() + width];
        for x in 0..width {
            let value = if native {
                source_row[x]
            } else {
                pack(source_row[x], framebuffer)
            };
            core::ptr::write_volatile(row.add(x), value);
        }
    }
}

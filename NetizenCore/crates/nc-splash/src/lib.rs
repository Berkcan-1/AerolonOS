#![no_std]

use nc_font::{draw_text, text_width, Style};
use nc_gfx::{lerp_color, rgb, Canvas};

const TITLE: &str = "NetizenCore";
static LOGO_BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/logo.raw"));

struct Logo {
    width: u32,
    height: u32,
    pixels: &'static [u8],
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

fn parse_logo() -> Option<Logo> {
    let blob = LOGO_BLOB;
    if blob.len() < 12 || blob[0] != b'N' || blob[1] != b'C' || blob[2] != b'L' || blob[3] != b'G' {
        return None;
    }
    let width = read_u32(blob, 4);
    let height = read_u32(blob, 8);
    let needed = width as usize * height as usize * 4;
    if width == 0 || height == 0 || blob.len() < 12 + needed {
        return None;
    }
    Some(Logo { width, height, pixels: &blob[12..12 + needed] })
}

pub fn draw(canvas: &mut Canvas<'_>) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;

    canvas.vertical_gradient(rgb(5, 6, 10), rgb(14, 16, 24));

    let logo = parse_logo();
    let (logo_width, logo_height) = match &logo {
        Some(logo) => (logo.width as i32, logo.height as i32),
        None => (0, 0),
    };

    let cap = (height / 11).max(20);
    let style = Style {
        cap_height: cap,
        stroke_sixteenths: (cap * 16 / 22).max(24),
        tracking: cap * 7 / 10,
    };
    let title_width = text_width(TITLE, &style);
    let gap = if logo_height > 0 { cap * 9 / 10 } else { 0 };
    let block_height = logo_height + gap + cap;
    let top = ((height - block_height) / 2 - height / 24).max(8);
    let center_x = width / 2;
    let text_top = top + logo_height + gap;

    canvas.add_glow(center_x, top + block_height / 2, height * 55 / 100, rgb(70, 52, 20), 180);

    if let Some(logo) = &logo {
        canvas.draw_rgba(center_x - logo_width / 2, top, logo.width, logo.height, logo.pixels);
    }

    let top_color = rgb(244, 222, 164);
    let bottom_color = rgb(176, 132, 58);
    {
        let mut plot = |x: i32, y: i32, coverage: u8| {
            let t = ((y - text_top) * 255 / cap).clamp(0, 255) as u32;
            canvas.blend_pixel(x, y, lerp_color(top_color, bottom_color, t), coverage);
        };
        draw_text(TITLE, center_x - title_width / 2, text_top, &style, &mut plot);
    }

    let line_y = text_top + cap + cap * 8 / 10;
    let line_x = center_x - title_width / 2;
    let line_color = rgb(212, 175, 98);
    let half = (title_width / 2).max(1);
    for offset in 0..title_width {
        let distance = (offset - title_width / 2).abs();
        let alpha = (255 - distance * 255 / half).clamp(0, 255) as u8;
        canvas.blend_pixel(line_x + offset, line_y, line_color, alpha);
        canvas.blend_pixel(line_x + offset, line_y + 1, line_color, alpha / 3);
    }
}

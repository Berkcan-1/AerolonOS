#![no_std]

mod dock;
mod topbar;
mod views;

use nc_font::text_width;
use nc_gfx::Canvas;
use nc_hal::{KeyCode, MouseState};
use nc_ui::{draw_backdrop, draw_text_color, text_style, TEXT_DIM};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DesktopAction {
    None,
    Shutdown,
    Reboot,
}

pub struct DesktopState {
    pub active_tab: usize,
}

impl DesktopState {
    pub fn new() -> DesktopState {
        DesktopState { active_tab: 0 }
    }

    pub fn update(&mut self, mouse: &MouseState, key: Option<KeyCode>, width: i32, height: i32) -> DesktopAction {
        if let Some(k) = key {
            match k {
                KeyCode::Escape => return DesktopAction::Shutdown,
                KeyCode::Left => {
                    if self.active_tab > 0 {
                        self.active_tab -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.active_tab < 2 {
                        self.active_tab += 1;
                    }
                }
                _ => {}
            }
        }

        match topbar::update(mouse, width, height) {
            topbar::TopBarAction::Shutdown => return DesktopAction::Shutdown,
            topbar::TopBarAction::Reboot => return DesktopAction::Reboot,
            topbar::TopBarAction::None => {}
        }

        self.active_tab = dock::update(mouse, width, height, self.active_tab);
        DesktopAction::None
    }
}

pub fn draw(canvas: &mut Canvas<'_>, platform: &str, state: &DesktopState, mouse: &MouseState, time_seconds: u32) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let unit = (height / 36).max(10);

    draw_backdrop(canvas);

    topbar::draw(canvas, width, height, mouse, time_seconds);
    views::draw_active_view(canvas, width, height, platform, state.active_tab);
    dock::draw(canvas, width, height, mouse, state.active_tab);

    let footer_y = height - unit * 2;
    let footer_style = text_style(unit * 5 / 10, 40);
    draw_text_color(canvas, "NETIZENCORE OPERATING SYSTEM", unit * 3, footer_y, &footer_style, TEXT_DIM);

    let mut plat_buf = [0u8; 32];
    let mut plen = 0;
    for b in b"PLATFORM: " {
        plat_buf[plen] = *b;
        plen += 1;
    }
    for b in platform.bytes() {
        if plen < 31 {
            plat_buf[plen] = b.to_ascii_uppercase();
            plen += 1;
        }
    }
    if let Ok(st) = core::str::from_utf8(&plat_buf[..plen]) {
        let pw = text_width(st, &footer_style);
        draw_text_color(canvas, st, width - unit * 3 - pw, footer_y, &footer_style, TEXT_DIM);
    }
}

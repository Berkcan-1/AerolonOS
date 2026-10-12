use nc_font::text_width;
use nc_gfx::{Canvas, Rect};
use nc_hal::MouseState;
use nc_ui::{
    draw_badge, draw_button, draw_panel, draw_text_color, text_style, PANEL_BORDER, PANEL_FILL,
    SUCCESS, SUCCESS_BG, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    None,
    Shutdown,
    Reboot,
}

fn point_in_rect(x: i32, y: i32, r: Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

pub fn update(mouse: &MouseState, width: i32, height: i32) -> TopBarAction {
    let unit = (height / 36).max(10);
    let topbar_h = unit * 3;
    let shutdown_btn = Rect::new(width - unit * 11, (topbar_h - unit * 2) / 2, unit * 9, unit * 2);
    let reboot_btn = Rect::new(shutdown_btn.x - unit * 10, (topbar_h - unit * 2) / 2, unit * 9, unit * 2);

    if mouse.left_clicked {
        if point_in_rect(mouse.x, mouse.y, shutdown_btn) {
            return TopBarAction::Shutdown;
        }
        if point_in_rect(mouse.x, mouse.y, reboot_btn) {
            return TopBarAction::Reboot;
        }
    }
    TopBarAction::None
}

pub fn draw(canvas: &mut Canvas<'_>, width: i32, height: i32, mouse: &MouseState, time_seconds: u32) {
    let unit = (height / 36).max(10);
    let topbar_h = unit * 3;
    let topbar = Rect::new(0, 0, width, topbar_h);
    draw_panel(canvas, topbar, 0, PANEL_FILL, 250, PANEL_BORDER, 255);

    let brand_style = text_style(unit, 30);
    draw_text_color(canvas, "NETIZENCORE OS", unit * 2, (topbar_h - brand_style.cap_height) / 2, &brand_style, TEXT_PRIMARY);

    let status_badge = Rect::new(unit * 18, (topbar_h - unit * 3 / 2) / 2, unit * 6, unit * 3 / 2);
    draw_badge(canvas, status_badge, "RUNNING", SUCCESS_BG, SUCCESS);

    let info_style = text_style(unit * 6 / 10, 30);
    let mut time_buf = [0u8; 16];
    let mins = (time_seconds / 60) % 60;
    let secs = time_seconds % 60;
    time_buf[0] = b'0' + (mins / 10) as u8;
    time_buf[1] = b'0' + (mins % 10) as u8;
    time_buf[2] = b':';
    time_buf[3] = b'0' + (secs / 10) as u8;
    time_buf[4] = b'0' + (secs % 10) as u8;
    if let Ok(t_str) = core::str::from_utf8(&time_buf[..5]) {
        let tw = text_width(t_str, &info_style);
        draw_text_color(canvas, t_str, width / 2 - tw / 2, (topbar_h - info_style.cap_height) / 2, &info_style, TEXT_MUTED);
    }

    let shutdown_btn = Rect::new(width - unit * 11, (topbar_h - unit * 2) / 2, unit * 9, unit * 2);
    let reboot_btn = Rect::new(shutdown_btn.x - unit * 10, (topbar_h - unit * 2) / 2, unit * 9, unit * 2);

    let reb_hov = point_in_rect(mouse.x, mouse.y, reboot_btn);
    draw_button(canvas, reboot_btn, "RESTART", reb_hov, false);

    let shut_hov = point_in_rect(mouse.x, mouse.y, shutdown_btn);
    draw_button(canvas, shutdown_btn, "SHUTDOWN", shut_hov, true);
}

use nc_font::text_width;
use nc_gfx::{Canvas, Rect};
use nc_hal::MouseState;
use nc_ui::{
    draw_panel, draw_text_color, text_style, ACCENT_LINE, PANEL_BORDER, PANEL_FILL, TEXT_MUTED,
    WHITE,
};

#[allow(dead_code)]
pub const TAB_STORAGE: usize = 0;
#[allow(dead_code)]
pub const TAB_SYSTEM: usize = 1;
#[allow(dead_code)]
pub const TAB_VOLUMES: usize = 2;

const TABS: [&str; 3] = ["STORAGE", "SYSTEM", "VOLUMES"];

fn point_in_rect(x: i32, y: i32, r: Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

pub fn update(mouse: &MouseState, width: i32, height: i32, current_tab: usize) -> usize {
    let unit = (height / 36).max(10);
    let dock_w = unit * 24;
    let dock_h = unit * 5 / 2;
    let dock_x = (width - dock_w) / 2;
    let dock_y = height - unit * 7;
    let tab_w = dock_w / 3;

    if mouse.left_clicked {
        for i in 0..3 {
            let tab_rect = Rect::new(dock_x + i as i32 * tab_w, dock_y, tab_w, dock_h);
            if point_in_rect(mouse.x, mouse.y, tab_rect) {
                return i;
            }
        }
    }
    current_tab
}

pub fn draw(canvas: &mut Canvas<'_>, width: i32, height: i32, mouse: &MouseState, active_tab: usize) {
    let unit = (height / 36).max(10);
    let dock_w = unit * 24;
    let dock_h = unit * 5 / 2;
    let dock_x = (width - dock_w) / 2;
    let dock_y = height - unit * 7;
    let dock_rect = Rect::new(dock_x, dock_y, dock_w, dock_h);

    draw_panel(canvas, dock_rect, unit / 2, PANEL_FILL, 255, PANEL_BORDER, 255);

    let tab_w = dock_w / 3;
    let label_style = text_style(unit * 5 / 10, 30);

    for i in 0..3 {
        let tab_rect = Rect::new(dock_x + i as i32 * tab_w + 2, dock_y + 2, tab_w - 4, dock_h - 4);
        let is_active = i == active_tab;
        let is_hov = point_in_rect(mouse.x, mouse.y, tab_rect);

        if is_active {
            draw_panel(canvas, tab_rect, unit / 3, ACCENT_LINE, 255, ACCENT_LINE, 0);
            let tw = text_width(TABS[i], &label_style);
            let tx = tab_rect.x + (tab_rect.width - tw) / 2;
            let ty = tab_rect.y + (tab_rect.height - label_style.cap_height) / 2;
            draw_text_color(canvas, TABS[i], tx, ty, &label_style, WHITE);
        } else {
            if is_hov {
                draw_panel(canvas, tab_rect, unit / 3, PANEL_BORDER, 150, PANEL_BORDER, 0);
            }
            let tw = text_width(TABS[i], &label_style);
            let tx = tab_rect.x + (tab_rect.width - tw) / 2;
            let ty = tab_rect.y + (tab_rect.height - label_style.cap_height) / 2;
            draw_text_color(canvas, TABS[i], tx, ty, &label_style, TEXT_MUTED);
        }
    }
}

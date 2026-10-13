use nc_app::theme::SIDEBAR_HOVER;
use nc_app::{draw_icon, AppInfo};
use nc_app::icon::{draw_cross, draw_dash};
use nc_gfx::{Canvas, Rect};
use nc_ui::{draw_panel, draw_shadow, draw_text_color, text_style, PANEL_BORDER, PANEL_FILL, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY, WHITE};

use crate::layout::{close_button, max_button, min_button, title_height};
use crate::theme::{BLACK, CLOSE_HOVER, TITLE_ACTIVE, TITLE_INACTIVE};
use crate::Target;

pub fn draw_frame(
    canvas: &mut Canvas<'_>,
    info: &AppInfo,
    index: usize,
    hover: Target,
    accent: u32,
    win: Rect,
    focused: bool,
    maximized: bool,
    unit: i32,
) {
    let radius = if maximized { 0 } else { unit * 4 / 5 };
    let shadow_strength = if focused && !maximized { 96 } else if !maximized { 44 } else { 0 };

    if shadow_strength > 0 {
        draw_shadow(canvas, win, radius, unit / 2, unit * 2, BLACK, shadow_strength);
    }

    let border_color = if focused { accent } else { PANEL_BORDER };
    draw_panel(canvas, win, radius, PANEL_FILL, 255, border_color, 255);

    let title = title_height(unit);
    let title_color = if focused { TITLE_ACTIVE } else { TITLE_INACTIVE };
    draw_panel(canvas, Rect::new(win.x, win.y, win.width, title + radius), radius, title_color, 255, border_color, 255);
    canvas.fill_rect(win.x + 1, win.y + title, win.width - 2, radius, PANEL_FILL);
    canvas.fill_rect(win.x + 1, win.y + title, win.width - 2, 1, border_color);

    let ink = if focused { TEXT_PRIMARY } else { TEXT_DIM };
    draw_icon(canvas, info.icon, win.x + unit * 2, win.y + title / 2, unit * 3 / 2, if focused { accent } else { TEXT_DIM });
    let label_style = text_style(unit * 6 / 10, 40);
    draw_text_color(canvas, &info.name, win.x + unit * 7 / 2, win.y + (title - label_style.cap_height) / 2, &label_style, ink);

    let minimize = min_button(win, unit);
    if hover == Target::WinMin(index) {
        draw_panel(canvas, minimize, unit / 3, SIDEBAR_HOVER, 255, SIDEBAR_HOVER, 0);
    }
    draw_dash(canvas, minimize.x + minimize.width / 2, minimize.y + minimize.height / 2, unit / 2, TEXT_MUTED);

    let maximize_btn = max_button(win, unit);
    if hover == Target::WinMax(index) {
        draw_panel(canvas, maximize_btn, unit / 3, SIDEBAR_HOVER, 255, SIDEBAR_HOVER, 0);
    }
    let max_box = Rect::new(
        maximize_btn.x + maximize_btn.width / 2 - unit / 3,
        maximize_btn.y + maximize_btn.height / 2 - unit / 3,
        unit * 2 / 3,
        unit * 2 / 3,
    );
    draw_panel(canvas, max_box, 1, WHITE, 0, TEXT_MUTED, 255);

    let close = close_button(win, unit);
    let close_hover = hover == Target::WinClose(index);
    if close_hover {
        draw_panel(canvas, close, unit / 3, CLOSE_HOVER, 255, CLOSE_HOVER, 0);
    }
    let cross_color = if close_hover { PANEL_FILL } else { TEXT_MUTED };
    draw_cross(canvas, close.x + close.width / 2, close.y + close.height / 2, unit / 3, cross_color);
}

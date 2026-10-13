use nc_app::icon::draw_grid;
use nc_app::text::{horizontal_line, shadowed, Line};
use nc_app::theme::accent_hover;
use nc_app::draw_icon;
use nc_font::text_width;
use nc_gfx::{Canvas, Rect};
use nc_ui::{draw_badge, draw_dot, draw_panel, draw_shadow, draw_text_color, text_style, PANEL_BORDER, PANEL_FILL, WHITE};

use crate::layout::{
    context_item_rect, context_menu_rect, desktop_icon, icon_tile, separator_y, start_button, start_item, start_menu, task_button, taskbar_rect,
    visible_start_apps,
};
use crate::theme::{BLACK, DANGER, ICON_SLATE, ON_DARK, ON_DARK_DIM, TASKBAR, TASKBAR_ALPHA, WARN};
use crate::{DesktopState, Target};

pub fn draw_icons(canvas: &mut Canvas<'_>, state: &DesktopState, height: i32, unit: i32) {
    let label_style = text_style(unit * 6 / 10, 30);
    let accent = state.settings.accent_color();
    for (index, slot) in state.apps.iter().enumerate() {
        let info = slot.app.info();
        let cell = desktop_icon(index, height, unit);
        if state.hover == Target::Icon(index) {
            draw_panel(canvas, cell, unit / 2, WHITE, 44, WHITE, 90);
        }
        let tile = icon_tile(cell, unit);
        let color = if index % 2 == 0 { accent } else { ICON_SLATE };
        draw_shadow(canvas, tile, unit, unit / 3, unit, BLACK, 60);
        draw_panel(canvas, tile, unit, color, 255, WHITE, 110);
        draw_icon(canvas, info.icon, tile.x + tile.width / 2, tile.y + tile.height / 2, unit * 2, WHITE);
        let width = text_width(&info.name, &label_style);
        let label_y = tile.y + tile.height + unit / 2;
        shadowed(canvas, &info.name, cell.x + (cell.width - width) / 2, label_y, &label_style, WHITE, BLACK);
    }
}

pub fn draw_taskbar(canvas: &mut Canvas<'_>, state: &DesktopState, width: i32, height: i32, unit: i32) {
    let bar = taskbar_rect(width, height, unit);
    draw_panel(canvas, bar, 0, TASKBAR, TASKBAR_ALPHA, WHITE, 26);

    let accent = state.settings.accent_color();
    let start = start_button(height, unit);
    let start_active = state.start_open || state.hover == Target::StartButton;
    let start_color = if start_active { accent_hover(accent) } else { accent };
    draw_panel(canvas, start, unit / 2, start_color, 255, WHITE, 40);
    draw_grid(canvas, start.x + unit * 3 / 2 + unit / 4, start.y + start.height / 2, unit * 6 / 5, WHITE);
    let label_style = text_style(unit * 6 / 10, 40);
    draw_text_color(canvas, "START", start.x + unit * 3, start.y + (start.height - label_style.cap_height) / 2, &label_style, WHITE);

    let focused = state.focused();
    let mut slot_index = 0;
    for (index, slot) in state.apps.iter().enumerate() {
        if !slot.window.open {
            continue;
        }
        let button = task_button(slot_index, height, unit);
        slot_index += 1;
        let minimized = slot.window.minimized;
        if focused == Some(index) {
            draw_panel(canvas, button, unit / 2, WHITE, 38, WHITE, 40);
        } else if state.hover == Target::TaskButton(index) {
            draw_panel(canvas, button, unit / 2, WHITE, 22, WHITE, 0);
        }
        let ink = if minimized { ON_DARK_DIM } else { ON_DARK };
        let info = slot.app.info();
        draw_icon(canvas, info.icon, button.x + unit * 3 / 2 + unit / 4, button.y + button.height / 2, unit * 3 / 2, ink);
        draw_text_color(canvas, &info.name, button.x + unit * 3, button.y + (button.height - label_style.cap_height) / 2, &label_style, ink);
        let marker_color = if focused == Some(index) { accent } else { ON_DARK_DIM };
        let marker = Rect::new(button.x + button.width / 2 - unit, button.y + button.height - 3, unit * 2, 3);
        draw_panel(canvas, marker, 1, marker_color, 255, marker_color, 0);
    }

    let seconds = state.uptime_ms / 1000;
    let mut tray = Line::new();
    tray.text("NET: 10.0.2.15 (E1000)  |  RAM 32MB / ")
        .number(state.total_memory_mb, 1)
        .text("MB  |  UP ")
        .number(seconds / 3600 % 100, 2)
        .text(":")
        .number(seconds / 60 % 60, 2)
        .text(":")
        .number(seconds % 60, 2);
    let tray_width = text_width(tray.as_str(), &label_style);
    draw_text_color(canvas, tray.as_str(), width - unit * 2 - tray_width, bar.y + (bar.height - label_style.cap_height) / 2, &label_style, ON_DARK_DIM);
}

pub fn draw_start_menu(canvas: &mut Canvas<'_>, state: &DesktopState, height: i32, unit: i32) {
    let visible = visible_start_apps(state.apps.len(), height, unit);
    let menu = start_menu(visible, height, unit);
    draw_shadow(canvas, menu, unit, unit / 2, unit * 2, BLACK, 90);
    draw_panel(canvas, menu, unit, TASKBAR, 255, WHITE, 36);

    let accent = state.settings.accent_color();
    let caption_style = text_style(unit * 55 / 100, 40);
    draw_dot(canvas, menu.x + unit * 2, menu.y + unit * 3 / 2 + caption_style.cap_height / 2, unit / 4, accent, 256);
    draw_text_color(canvas, "NETIZENCORE OS", menu.x + unit * 3, menu.y + unit * 3 / 2, &caption_style, ON_DARK_DIM);

    let label_style = text_style(unit * 65 / 100, 30);
    for index in 0..visible + 2 {
        let item = start_item(index, visible, height, unit);
        if state.hover == Target::StartItem(index) {
            draw_panel(canvas, item, unit / 2, WHITE, 30, WHITE, 0);
        }
        let center_y = item.y + item.height / 2;
        let center_x = item.x + unit * 3 / 2 + unit / 4;
        let label = if index < visible {
            let info = state.apps[index].app.info();
            draw_icon(canvas, info.icon, center_x, center_y, unit * 3 / 2, ON_DARK);
            &info.name[..]
        } else if index == visible {
            draw_dot(canvas, center_x, center_y, unit / 3, WARN, 256);
            "RESTART"
        } else {
            draw_dot(canvas, center_x, center_y, unit / 3, DANGER, 256);
            "SHUT DOWN"
        };
        draw_text_color(canvas, label, item.x + unit * 3 + unit / 2, center_y - label_style.cap_height / 2, &label_style, ON_DARK);
    }

    let dash_rect = Rect::new(menu.x + unit * 13, menu.y + unit * 3, unit * 14, menu.height - unit * 6);
    draw_panel(canvas, dash_rect, unit / 2, PANEL_FILL, 20, PANEL_BORDER, 100);

    draw_text_color(canvas, "SYSTEM DASHBOARD", dash_rect.x + unit, dash_rect.y + unit, &caption_style, ON_DARK_DIM);

    let mut plat_line = Line::new();
    plat_line.text("Platform: ").text(state.platform);
    draw_text_color(canvas, plat_line.as_str(), dash_rect.x + unit, dash_rect.y + unit * 5 / 2, &label_style, ON_DARK);

    let mut ram_line = Line::new();
    ram_line.text("Memory: ").number(state.total_memory_mb, 1).text(" MB RAM");
    draw_text_color(canvas, ram_line.as_str(), dash_rect.x + unit, dash_rect.y + unit * 4, &label_style, ON_DARK);

    draw_badge(canvas, Rect::new(dash_rect.x + unit / 2, dash_rect.y + unit * 6, unit * 12, unit * 3 / 2), "VFS : /RAM READY", accent, WHITE);

    horizontal_line(canvas, menu.x + unit, separator_y(visible, height, unit), menu.width - unit * 2, WHITE, 40);
}

pub fn draw_context_menu(canvas: &mut Canvas<'_>, state: &DesktopState, pos: (i32, i32), width: i32, height: i32, unit: i32) {
    let menu = context_menu_rect(pos.0, pos.1, width, height, unit);
    draw_shadow(canvas, menu, unit / 2, unit / 4, unit, BLACK, 80);
    draw_panel(canvas, menu, unit / 2, TASKBAR, 240, PANEL_BORDER, 180);

    let label_style = text_style(unit * 6 / 10, 30);
    let items = [
        "CHANGE WALLPAPER",
        "OPEN FILE MANAGER",
        "SYSTEM INFORMATION",
        "REFRESH DESKTOP",
    ];

    for (idx, label) in items.iter().enumerate() {
        let item_r = context_item_rect(menu, idx, unit);
        if state.hover == Target::ContextItem(idx) {
            draw_panel(canvas, item_r, unit / 4, WHITE, 40, WHITE, 0);
        }
        let ty = item_r.y + (item_r.height - label_style.cap_height) / 2;
        draw_text_color(canvas, label, item_r.x + unit, ty, &label_style, ON_DARK);
    }
}

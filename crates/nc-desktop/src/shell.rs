use nc_app::draw_icon;
use nc_app::icon::{draw_cross, draw_grid};
use nc_app::text::{shadowed, Line};
use nc_font::text_width;
use nc_gfx::{lerp_color, rgb, Canvas, Rect};
use nc_math::{cos_milli, sin_milli};
use nc_ui::{draw_dot, draw_panel, draw_shadow, draw_text_color, text_style, WHITE};

use crate::layout::{
    context_item_rect, context_menu_rect, desktop_modal_rect, dock_app_pill, dock_rect,
    dock_sensor_strip, dock_start_orb, icon_tile, launch_toast_rect, snake_head_pod_rect,
    snake_node_rect, snake_power_buttons, task_manager_action_btn, task_manager_close_rect,
    task_manager_rect, task_manager_row_rect, title_height,
};
use crate::theme::{
    AQUA_CORE, AQUA_DEEP, AQUA_SOFT, AZURE_CORE, AZURE_DEEP, BLACK, CLOSE_HOVER, DANGER_CORAL,
    GLASS_BORDER, GLASS_PORCELAIN_BOT, GLASS_PORCELAIN_TOP, INK_BODY, INK_DARK, INK_MUTED,
    SHADOW_DEEP, WARN_AMBER,
};
use crate::{ContextMenuKind, DesktopIconId, DesktopModal, DesktopState, Target};

const MAX_BLUR_SPAN: usize = 2048;

pub fn blur_rect_in_place(canvas: &mut Canvas<'_>, rect: Rect, radius: usize) {
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let x0 = rect.x.clamp(0, cw);
    let y0 = rect.y.clamp(0, ch);
    let x1 = (rect.x + rect.width).clamp(0, cw);
    let y1 = (rect.y + rect.height).clamp(0, ch);
    let rw = (x1 - x0) as usize;
    let rh = (y1 - y0) as usize;
    if rw == 0 || rh == 0 || radius == 0 || rw > MAX_BLUR_SPAN || rh > MAX_BLUR_SPAN {
        return;
    }

    let stride = cw as usize;
    let pixels = canvas.pixels_mut();
    let mut scratch = [0u32; MAX_BLUR_SPAN];
    let r = radius.min(16);
    let win = (r * 2 + 1) as u32;

    for _ in 0..2 {
        for y in (y0 as usize)..(y1 as usize) {
            let base = y * stride + (x0 as usize);
            for i in 0..rw {
                scratch[i] = pixels[base + i];
            }
            let first = scratch[0];
            let mut sr = ((first >> 16) & 0xFF) * (r as u32 + 1);
            let mut sg = ((first >> 8) & 0xFF) * (r as u32 + 1);
            let mut sb = (first & 0xFF) * (r as u32 + 1);
            for i in 1..=r {
                let p = scratch[i.min(rw - 1)];
                sr += (p >> 16) & 0xFF;
                sg += (p >> 8) & 0xFF;
                sb += p & 0xFF;
            }
            for i in 0..rw {
                pixels[base + i] = ((sr / win) << 16) | ((sg / win) << 8) | (sb / win);
                let out_p = scratch[i.saturating_sub(r)];
                let in_p = scratch[(i + r + 1).min(rw - 1)];
                sr = sr + ((in_p >> 16) & 0xFF) - ((out_p >> 16) & 0xFF);
                sg = sg + ((in_p >> 8) & 0xFF) - ((out_p >> 8) & 0xFF);
                sb = sb + (in_p & 0xFF) - (out_p & 0xFF);
            }
        }

        for x in (x0 as usize)..(x1 as usize) {
            for j in 0..rh {
                scratch[j] = pixels[(y0 as usize + j) * stride + x];
            }
            let first = scratch[0];
            let mut sr = ((first >> 16) & 0xFF) * (r as u32 + 1);
            let mut sg = ((first >> 8) & 0xFF) * (r as u32 + 1);
            let mut sb = (first & 0xFF) * (r as u32 + 1);
            for j in 1..=r {
                let p = scratch[j.min(rh - 1)];
                sr += (p >> 16) & 0xFF;
                sg += (p >> 8) & 0xFF;
                sb += p & 0xFF;
            }
            for j in 0..rh {
                pixels[(y0 as usize + j) * stride + x] =
                    ((sr / win) << 16) | ((sg / win) << 8) | (sb / win);
                let out_p = scratch[j.saturating_sub(r)];
                let in_p = scratch[(j + r + 1).min(rh - 1)];
                sr = sr + ((in_p >> 16) & 0xFF) - ((out_p >> 16) & 0xFF);
                sg = sg + ((in_p >> 8) & 0xFF) - ((out_p >> 8) & 0xFF);
                sb = sb + (in_p & 0xFF) - (out_p & 0xFF);
            }
        }
    }
}

pub fn draw_aero_glass_surface(
    canvas: &mut Canvas<'_>,
    rect: Rect,
    radius: i32,
    blur_radius: usize,
    fill_alpha: u32,
    shadow_strength: u32,
) {
    if rect.width <= 2 || rect.height <= 2 {
        return;
    }
    if shadow_strength > 0 {
        draw_shadow(canvas, rect, radius, 4, 14, SHADOW_DEEP, shadow_strength);
    }
    if blur_radius > 0 {
        blur_rect_in_place(canvas, rect, blur_radius);
    }
    draw_panel(
        canvas,
        rect,
        radius,
        GLASS_PORCELAIN_TOP,
        fill_alpha,
        GLASS_BORDER,
        235,
    );
    let inner_bottom = Rect::new(
        rect.x + 2,
        rect.y + rect.height / 2,
        rect.width - 4,
        rect.height / 2 - 1,
    );
    if inner_bottom.width > 4 && inner_bottom.height > 2 {
        draw_panel(
            canvas,
            inner_bottom,
            (radius - 2).max(1),
            GLASS_PORCELAIN_BOT,
            fill_alpha / 3,
            GLASS_PORCELAIN_BOT,
            0,
        );
    }
    let glint_w = (rect.width * 60 / 100).max(12);
    let glint_x = rect.x + (rect.width - glint_w) / 2;
    if rect.y >= 0 && rect.y < canvas.height() as i32 {
        for i in 0..glint_w {
            let t = (i * 255 / glint_w.max(1)) as u32;
            let c = lerp_color(AZURE_CORE, AQUA_CORE, t);
            canvas.blend_pixel(glint_x + i, rect.y + 1, c, 150);
        }
    }
}

fn draw_floating_orb(
    canvas: &mut Canvas<'_>,
    cx: i32,
    cy: i32,
    radius: i32,
    c1: u32,
    c2: u32,
    alpha: u32,
) {
    if radius <= 2 {
        return;
    }
    let r2 = (radius as i64) * (radius as i64);
    let x0 = (cx - radius).max(0);
    let x1 = (cx + radius).min(canvas.width() as i32 - 1);
    let y0 = (cy - radius).max(0);
    let y1 = (cy + radius).min(canvas.height() as i32 - 1);
    for y in y0..=y1 {
        let dy = (y - cy) as i64;
        for x in x0..=x1 {
            let dx = (x - cx) as i64;
            let d2 = dx * dx + dy * dy;
            if d2 >= r2 {
                continue;
            }
            let t = (d2 * 255 / r2) as u32;
            let col = lerp_color(c1, c2, t);
            let a = ((255 - t * 180 / 255) * alpha / 256).min(255) as u8;
            canvas.blend_pixel(x, y, col, a);
            if t > 200 {
                canvas.blend_pixel(x, y, WHITE, (alpha / 3).min(180) as u8);
            }
        }
    }
}

pub fn draw_animated_wallpaper_effects(canvas: &mut Canvas<'_>, state: &DesktopState) {
    if !state.settings.wallpaper_animated {
        return;
    }
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let speed = state.settings.wallpaper_speed.max(1) as u32;
    let deg1 = ((state.uptime_ms / (22 / speed.max(1)).max(4)) % 360) as i32;
    let deg2 = ((state.uptime_ms / (28 / speed.max(1)).max(5) + 120) % 360) as i32;
    let deg3 = ((state.uptime_ms / (34 / speed.max(1)).max(6) + 240) % 360) as i32;

    let ox1 = w * 24 / 100 + (cos_milli(deg1) * (w / 9) / 1000);
    let oy1 = h * 30 / 100 + (sin_milli(deg1) * (h / 10) / 1000);
    draw_floating_orb(canvas, ox1, oy1, h / 6, AQUA_SOFT, AZURE_CORE, 56);

    let ox2 = w * 74 / 100 + (sin_milli(deg2) * (w / 8) / 1000);
    let oy2 = h * 66 / 100 + (cos_milli(deg2) * (h / 9) / 1000);
    draw_floating_orb(canvas, ox2, oy2, h / 5, rgb(180, 228, 255), AQUA_CORE, 52);

    let ox3 = w * 52 / 100 + (cos_milli(deg3) * (w / 10) / 1000);
    let oy3 = h * 22 / 100 + (sin_milli(deg3) * (h / 12) / 1000);
    draw_floating_orb(canvas, ox3, oy3, h / 9, WHITE, AQUA_SOFT, 44);
}

fn draw_fs_tile_art(canvas: &mut Canvas<'_>, tile: Rect, is_dir: bool, unit: i32) {
    if is_dir {
        let tab = Rect::new(
            tile.x + unit / 2,
            tile.y + unit / 2,
            unit * 5 / 4,
            unit / 3 + 2,
        );
        draw_panel(canvas, tab, 2, AQUA_CORE, 255, AQUA_CORE, 0);
        let folder = Rect::new(
            tile.x + unit / 3,
            tile.y + unit * 3 / 4,
            tile.width - unit * 2 / 3,
            tile.height - unit * 4 / 3,
        );
        draw_panel(canvas, folder, unit / 3, AZURE_CORE, 245, WHITE, 190);
        let front = Rect::new(
            folder.x,
            folder.y + unit / 3,
            folder.width,
            folder.height - unit / 3,
        );
        draw_panel(canvas, front, unit / 3, AQUA_SOFT, 210, WHITE, 140);
    } else {
        let doc = Rect::new(
            tile.x + unit * 2 / 3,
            tile.y + unit / 2,
            tile.width - unit * 4 / 3,
            tile.height - unit,
        );
        draw_panel(canvas, doc, unit / 4, WHITE, 250, AZURE_CORE, 220);
        let lx = doc.x + 5;
        let lw = (doc.width - 10).max(4);
        canvas.fill_rect(lx, doc.y + 6, lw, 2, AZURE_CORE);
        canvas.fill_rect(lx, doc.y + 11, lw, 2, AQUA_CORE);
        canvas.fill_rect(lx, doc.y + 16, lw * 2 / 3, 2, INK_MUTED);
    }
}

pub fn draw_icons(canvas: &mut Canvas<'_>, state: &DesktopState, height: i32, unit: i32) {
    let label_style = text_style((unit * 54 / 100).max(10), 15);
    let accent = state.settings.accent_color();
    let blur_r = state.blur_radius(6);

    for (index, slot) in state.apps.iter().enumerate() {
        let info = slot.app.info();
        let cell = state.app_icon_rect(index, height, unit);
        let is_selected = state.selected_icon == Some(DesktopIconId::App(index));
        let is_hovered = state.hover == Target::AppIcon(index);

        if is_selected {
            draw_aero_glass_surface(canvas, cell, unit / 2, blur_r, 195, 32);
            draw_panel(canvas, cell, unit / 2, AQUA_SOFT, 75, AZURE_CORE, 245);
        } else if is_hovered {
            draw_aero_glass_surface(canvas, cell, unit / 2, blur_r, 125, 16);
        }

        let tile = icon_tile(cell, unit);
        let top_c = if index % 2 == 0 { accent } else { AZURE_DEEP };
        let bot_c = if index % 2 == 0 { AQUA_CORE } else { AZURE_CORE };
        draw_shadow(canvas, tile, unit * 3 / 4, unit / 4, unit * 3 / 4, SHADOW_DEEP, 52);
        draw_panel(canvas, tile, unit * 3 / 4, top_c, 245, WHITE, 175);
        let lower_half = Rect::new(
            tile.x + 2,
            tile.y + tile.height / 2,
            tile.width - 4,
            tile.height / 2 - 2,
        );
        draw_panel(canvas, lower_half, unit / 2, bot_c, 110, bot_c, 0);
        draw_icon(
            canvas,
            info.icon,
            tile.x + tile.width / 2,
            tile.y + tile.height / 2,
            unit * 7 / 4,
            WHITE,
        );
        if slot.pinned_dock {
            draw_dot(canvas, tile.x + tile.width - 5, tile.y + 5, 3, AQUA_SOFT, 255);
        }
        let width = text_width(&info.name, &label_style);
        let label_y = tile.y + tile.height + unit / 3;
        if is_selected {
            draw_text_color(
                canvas,
                &info.name,
                cell.x + (cell.width - width) / 2,
                label_y,
                &label_style,
                INK_DARK,
            );
        } else {
            shadowed(
                canvas,
                &info.name,
                cell.x + (cell.width - width) / 2,
                label_y,
                &label_style,
                WHITE,
                BLACK,
            );
        }
    }

    for (f_idx, item) in state.fs_items.iter().enumerate() {
        let cell = state.fs_icon_rect(f_idx, height, unit);
        let is_selected = state.selected_icon == Some(DesktopIconId::Fs(f_idx));
        let is_hovered = state.hover == Target::FsIcon(f_idx);

        if is_selected {
            draw_aero_glass_surface(canvas, cell, unit / 2, blur_r, 195, 32);
            draw_panel(canvas, cell, unit / 2, AQUA_SOFT, 75, AZURE_CORE, 245);
        } else if is_hovered {
            draw_aero_glass_surface(canvas, cell, unit / 2, blur_r, 125, 16);
        }

        let tile = icon_tile(cell, unit);
        draw_shadow(canvas, tile, unit * 3 / 4, unit / 4, unit * 3 / 4, SHADOW_DEEP, 44);
        draw_panel(
            canvas,
            tile,
            unit * 3 / 4,
            GLASS_PORCELAIN_TOP,
            210,
            GLASS_BORDER,
            220,
        );
        draw_fs_tile_art(canvas, tile, item.is_dir, unit);

        let width = text_width(&item.name, &label_style);
        let label_y = tile.y + tile.height + unit / 3;
        if is_selected {
            draw_text_color(
                canvas,
                &item.name,
                cell.x + (cell.width - width) / 2,
                label_y,
                &label_style,
                INK_DARK,
            );
        } else {
            shadowed(
                canvas,
                &item.name,
                cell.x + (cell.width - width) / 2,
                label_y,
                &label_style,
                WHITE,
                BLACK,
            );
        }
    }
}

pub fn draw_launch_toast(
    canvas: &mut Canvas<'_>,
    state: &DesktopState,
    width: i32,
    height: i32,
    unit: i32,
) {
    let Some(launching_idx) = state.launching_app() else {
        return;
    };
    let slot = &state.apps[launching_idx];
    let toast = launch_toast_rect(width, height, unit);
    draw_aero_glass_surface(canvas, toast, toast.height / 2, state.blur_radius(8), 236, 48);

    let orb_cx = toast.x + unit * 3 / 2;
    let orb_cy = toast.y + toast.height / 2;
    draw_dot(canvas, orb_cx, orb_cy, unit * 4 / 5, AZURE_CORE, 255);
    draw_icon(
        canvas,
        slot.app.info().icon,
        orb_cx,
        orb_cy,
        unit,
        WHITE,
    );

    let label_st = text_style((unit * 52 / 100).max(9), 15);
    let mut msg = Line::new();
    msg.text(&slot.app.info().name)
        .text(" arka planda hazirlaniyor...");
    draw_text_color(
        canvas,
        msg.as_str(),
        toast.x + unit * 3,
        toast.y + unit / 2,
        &label_st,
        INK_DARK,
    );

    let bar = Rect::new(
        toast.x + unit * 3,
        toast.y + toast.height - unit,
        toast.width - unit * 4,
        unit / 3 + 2,
    );
    draw_panel(canvas, bar, 3, GLASS_PORCELAIN_BOT, 255, GLASS_BORDER, 255);
    let total = slot.window.launch_total.max(1);
    let done = total.saturating_sub(slot.window.launch_delay);
    let fill_w = (bar.width * done as i32 / total as i32).max(6);
    draw_panel(
        canvas,
        Rect::new(bar.x, bar.y, fill_w, bar.height),
        3,
        AQUA_CORE,
        255,
        AQUA_CORE,
        0,
    );
}

pub fn draw_task_manager(
    canvas: &mut Canvas<'_>,
    state: &DesktopState,
    unit: i32,
) {
    if !state.task_mgr.open {
        return;
    }
    let tm = task_manager_rect(state.task_mgr.x, state.task_mgr.y, unit);
    draw_aero_glass_surface(canvas, tm, unit * 3 / 4, state.blur_radius(10), 238, 68);

    let title_h = title_height(unit);
    let title_st = text_style((unit * 58 / 100).max(10), 16);
    let label_st = text_style((unit * 50 / 100).max(9), 15);
    let cap_st = text_style((unit * 44 / 100).max(8), 16);

    draw_dot(canvas, tm.x + unit * 3 / 2, tm.y + title_h / 2, 5, AQUA_CORE, 255);
    draw_text_color(
        canvas,
        "Gorev Yoneticisi ve Arka Plan Islemleri",
        tm.x + unit * 5 / 2,
        tm.y + (title_h - title_st.cap_height) / 2,
        &title_st,
        INK_DARK,
    );

    let close_r = task_manager_close_rect(tm, unit);
    let close_hov = state.hover == Target::TaskMgrClose;
    if close_hov {
        draw_panel(canvas, close_r, unit / 3, CLOSE_HOVER, 255, CLOSE_HOVER, 0);
    }
    draw_cross(
        canvas,
        close_r.x + close_r.width / 2,
        close_r.y + close_r.height / 2,
        unit / 4,
        if close_hov { WHITE } else { INK_BODY },
    );

    let graph_card = Rect::new(
        tm.x + unit,
        tm.y + title_h + unit / 4,
        tm.width - unit * 2,
        unit * 5 + unit / 2,
    );
    draw_panel(
        canvas,
        graph_card,
        unit / 3,
        WHITE,
        215,
        GLASS_BORDER,
        240,
    );

    let cpu_pct = state.total_cpu_percent();
    let used_ram = state.used_ram_mb();
    let mut m_line = Line::new();
    m_line
        .text("Toplam CPU Yuku: %")
        .number(cpu_pct, 1)
        .text("    |    Bellek Kullanimi: ")
        .number(used_ram, 1)
        .text(" / ")
        .number(state.total_memory_mb, 1)
        .text(" MB");
    draw_text_color(
        canvas,
        m_line.as_str(),
        graph_card.x + unit * 3 / 4,
        graph_card.y + unit / 2,
        &label_st,
        AZURE_DEEP,
    );

    let bars_x = graph_card.x + unit * 3 / 4;
    let bars_y = graph_card.y + unit * 2;
    let bars_w = graph_card.width - unit * 3 / 2;
    let bars_h = unit * 3;
    canvas.fill_rect(bars_x, bars_y, bars_w, bars_h, GLASS_PORCELAIN_BOT);

    let step_w = (bars_w / 32).max(2);
    for i in 0..32usize {
        let hist_idx = (state.task_mgr.hist_cursor + i) % 32;
        let val = state.task_mgr.cpu_history[hist_idx].min(100) as i32;
        let bh = (bars_h * val / 100).max(2);
        let bx = bars_x + (i as i32) * step_w;
        let by = bars_y + bars_h - bh;
        let col = if val > 70 {
            WARN_AMBER
        } else if i % 2 == 0 {
            AZURE_CORE
        } else {
            AQUA_CORE
        };
        canvas.fill_rect(bx, by, (step_w - 1).max(1), bh, col);
    }

    let hdr_y = tm.y + unit * 8 + unit / 5;
    draw_text_color(canvas, "Islem Adi", tm.x + unit * 3 / 2, hdr_y, &cap_st, INK_MUTED);
    draw_text_color(canvas, "PID", tm.x + unit * 15, hdr_y, &cap_st, INK_MUTED);
    draw_text_color(canvas, "Durum", tm.x + unit * 19, hdr_y, &cap_st, INK_MUTED);
    draw_text_color(canvas, "CPU", tm.x + unit * 26, hdr_y, &cap_st, INK_MUTED);
    draw_text_color(canvas, "Bellek", tm.x + unit * 29, hdr_y, &cap_st, INK_MUTED);

    let rows = state.collect_task_rows();
    for (idx, row_data) in rows.iter().take(7).enumerate() {
        let rr = task_manager_row_rect(tm, idx, unit);
        let sel = state.task_mgr.selected_row == idx;
        let hov = state.hover == Target::TaskMgrRow(idx);
        let fill = if sel {
            AQUA_SOFT
        } else if hov {
            GLASS_PORCELAIN_BOT
        } else {
            WHITE
        };
        let brd = if sel { AZURE_CORE } else { GLASS_BORDER };
        draw_panel(canvas, rr, unit / 4, fill, 220, brd, 220);

        let dot_c = if row_data.is_app {
            AZURE_CORE
        } else {
            AQUA_CORE
        };
        draw_dot(canvas, rr.x + unit / 2, rr.y + rr.height / 2, 3, dot_c, 255);

        let ty = rr.y + (rr.height - cap_st.cap_height) / 2;
        draw_text_color(canvas, row_data.name, rr.x + unit, ty, &cap_st, INK_DARK);

        let mut pid_l = Line::new();
        pid_l.number(row_data.pid, 1);
        draw_text_color(canvas, pid_l.as_str(), tm.x + unit * 15, ty, &cap_st, INK_BODY);

        draw_text_color(
            canvas,
            row_data.status,
            tm.x + unit * 19,
            ty,
            &cap_st,
            if row_data.is_app { AZURE_DEEP } else { AQUA_DEEP },
        );

        let mut cpu_l = Line::new();
        cpu_l.text("%").number(row_data.cpu_pct as u32, 1);
        draw_text_color(canvas, cpu_l.as_str(), tm.x + unit * 26, ty, &cap_st, INK_DARK);

        let mut mem_l = Line::new();
        mem_l.number(row_data.mem_mb as u32, 1).text(" MB");
        draw_text_color(canvas, mem_l.as_str(), tm.x + unit * 29, ty, &cap_st, INK_BODY);
    }

    let opt_btn = task_manager_action_btn(tm, false, unit);
    let kill_btn = task_manager_action_btn(tm, true, unit);

    let opt_hov = state.hover == Target::TaskMgrOptimize;
    draw_panel(
        canvas,
        opt_btn,
        opt_btn.height / 2,
        if opt_hov { AQUA_SOFT } else { WHITE },
        245,
        AQUA_CORE,
        220,
    );
    let ow = text_width("Arka Plani Dengele", &cap_st);
    draw_text_color(
        canvas,
        "Arka Plani Dengele",
        opt_btn.x + (opt_btn.width - ow) / 2,
        opt_btn.y + (opt_btn.height - cap_st.cap_height) / 2,
        &cap_st,
        INK_DARK,
    );

    let kill_hov = state.hover == Target::TaskMgrKill;
    draw_panel(
        canvas,
        kill_btn,
        kill_btn.height / 2,
        if kill_hov { CLOSE_HOVER } else { DANGER_CORAL },
        250,
        CLOSE_HOVER,
        255,
    );
    let kw = text_width("Gorevi Sonlandir", &cap_st);
    draw_text_color(
        canvas,
        "Gorevi Sonlandir",
        kill_btn.x + (kill_btn.width - kw) / 2,
        kill_btn.y + (kill_btn.height - cap_st.cap_height) / 2,
        &cap_st,
        WHITE,
    );
}

pub fn draw_floating_dock(
    canvas: &mut Canvas<'_>,
    state: &DesktopState,
    width: i32,
    height: i32,
    unit: i32,
) {
    let dock_items = state.dock_indices();
    if state.dock_reveal < 16 {
        let strip = dock_sensor_strip(width, height, unit);
        draw_panel(canvas, strip, 2, AQUA_SOFT, 165, WHITE, 190);
        return;
    }

    let dock = dock_rect(width, height, unit, dock_items.len(), state.dock_reveal);
    let radius = dock.height / 2;
    draw_aero_glass_surface(canvas, dock, radius, state.blur_radius(10), 224, 54);

    let start_orb = dock_start_orb(dock, unit);
    let start_active = state.start_open || state.hover == Target::StartButton;
    let orb_fill = if start_active { AQUA_CORE } else { AZURE_CORE };
    draw_panel(
        canvas,
        start_orb,
        start_orb.height / 2,
        orb_fill,
        250,
        WHITE,
        180,
    );
    draw_grid(
        canvas,
        start_orb.x + unit + unit / 4,
        start_orb.y + start_orb.height / 2,
        unit,
        WHITE,
    );

    let label_style = text_style((unit * 56 / 100).max(10), 16);
    let small_style = text_style((unit * 46 / 100).max(9), 15);
    draw_text_color(
        canvas,
        "Baslat",
        start_orb.x + unit * 2 + unit / 5,
        start_orb.y + (start_orb.height - label_style.cap_height) / 2,
        &label_style,
        WHITE,
    );

    let focused = state.focused();
    for (slot_pos, &app_idx) in dock_items.iter().enumerate() {
        let pill = dock_app_pill(dock, slot_pos, unit);
        let slot = &state.apps[app_idx];
        let is_focus = focused == Some(app_idx);
        let is_hov = state.hover == Target::DockApp(app_idx);

        if is_focus {
            draw_panel(
                canvas,
                pill,
                pill.height / 2,
                AZURE_CORE,
                56,
                AZURE_CORE,
                210,
            );
        } else if is_hov {
            draw_panel(
                canvas,
                pill,
                pill.height / 2,
                AQUA_SOFT,
                95,
                AQUA_CORE,
                190,
            );
        } else {
            draw_panel(
                canvas,
                pill,
                pill.height / 2,
                WHITE,
                150,
                GLASS_BORDER,
                180,
            );
        }

        let icon_col = if is_focus {
            AZURE_DEEP
        } else if slot.window.open {
            INK_DARK
        } else {
            INK_BODY
        };
        draw_icon(
            canvas,
            slot.app.info().icon,
            pill.x + pill.width / 2,
            pill.y + pill.height / 2 - 1,
            unit * 4 / 3,
            icon_col,
        );

        if slot.window.open || slot.window.launch_delay > 0 {
            let dot_col = if slot.window.launch_delay > 0 {
                WARN_AMBER
            } else if is_focus {
                AQUA_CORE
            } else {
                AZURE_CORE
            };
            draw_dot(
                canvas,
                pill.x + pill.width / 2,
                pill.y + pill.height - 3,
                3,
                dot_col,
                255,
            );
        }
    }

    let seconds = state.uptime_ms / 1000;
    let mut tray = Line::new();
    let net_tag = if state.settings.wifi_enabled {
        "Wi-Fi"
    } else {
        "ETH"
    };
    tray.text(net_tag)
        .text("  |  CPU %")
        .number(state.total_cpu_percent(), 1)
        .text("  |  ")
        .number(seconds / 60 % 60, 2)
        .text(":")
        .number(seconds % 60, 2);
    let tray_w = text_width(tray.as_str(), &small_style);
    let tray_x = dock.x + dock.width - tray_w - unit;
    let tray_y = dock.y + (dock.height - small_style.cap_height) / 2;
    draw_text_color(canvas, tray.as_str(), tray_x, tray_y, &small_style, INK_BODY);
}

fn draw_energy_tube(canvas: &mut Canvas<'_>, a: Rect, b: Rect) {
    let ax = a.x + a.width / 2;
    let ay = a.y + a.height / 2;
    let bx = b.x + b.width / 2;
    let by = b.y + b.height / 2;
    let steps = ((bx - ax).abs().max((by - ay).abs()) / 3).max(1);
    for s in 0..=steps {
        let px = ax + (bx - ax) * s / steps;
        let py = ay + (by - ay) * s / steps;
        let t = (s * 255 / steps) as u32;
        let col = lerp_color(AZURE_CORE, AQUA_CORE, t);
        draw_dot(canvas, px, py, 4, col, 175);
        draw_dot(canvas, px, py, 2, WHITE, 210);
    }
}

fn draw_power_symbol(canvas: &mut Canvas<'_>, cx: i32, cy: i32, r: i32, col: u32) {
    draw_dot(canvas, cx, cy, r, col, 255);
    draw_dot(canvas, cx, cy, (r - 2).max(1), WHITE, 255);
    canvas.fill_rect(cx - 1, cy - r - 1, 3, r + 1, col);
}

fn draw_reboot_symbol(canvas: &mut Canvas<'_>, cx: i32, cy: i32, r: i32, col: u32) {
    draw_dot(canvas, cx, cy, r, col, 255);
    draw_dot(canvas, cx, cy, (r - 2).max(1), WHITE, 255);
    canvas.fill_rect(cx + 1, cy - r, r, 3, col);
}

pub fn draw_serpentine_start_menu(
    canvas: &mut Canvas<'_>,
    state: &DesktopState,
    width: i32,
    height: i32,
    unit: i32,
) {
    if state.start_anim == 0 {
        return;
    }
    let dock_items = state.dock_indices();
    let dock = dock_rect(
        width,
        height,
        unit,
        dock_items.len(),
        state.dock_reveal.max(220),
    );
    let snake_apps = state.start_indices();
    let count = snake_apps.len();
    let blur_r = state.blur_radius(8);

    let start_orb = dock_start_orb(dock, unit);
    if count > 0 {
        let first_node = snake_node_rect(0, dock, unit, state.start_anim);
        draw_energy_tube(canvas, start_orb, first_node);
    }

    for seq in 0..count {
        if seq + 1 < count {
            let cur_r = snake_node_rect(seq, dock, unit, state.start_anim);
            let nxt_r = snake_node_rect(seq + 1, dock, unit, state.start_anim);
            draw_energy_tube(canvas, cur_r, nxt_r);
        }
    }

    let head_pod = snake_head_pod_rect(count, dock, unit, state.start_anim);
    if count > 0 {
        let last_node = snake_node_rect(count - 1, dock, unit, state.start_anim);
        draw_energy_tube(canvas, last_node, head_pod);
    } else {
        draw_energy_tube(canvas, start_orb, head_pod);
    }

    let label_style = text_style((unit * 54 / 100).max(10), 15);
    let sub_style = text_style((unit * 44 / 100).max(8), 15);

    for (seq, &app_idx) in snake_apps.iter().enumerate() {
        let node = snake_node_rect(seq, dock, unit, state.start_anim);
        let is_hov = state.hover == Target::StartNode(app_idx);
        let info = state.apps[app_idx].app.info();

        draw_aero_glass_surface(
            canvas,
            node,
            node.height / 2,
            blur_r,
            if is_hov { 248 } else { 222 },
            36,
        );
        if is_hov {
            draw_panel(
                canvas,
                node,
                node.height / 2,
                AQUA_SOFT,
                85,
                AZURE_CORE,
                240,
            );
        }

        let badge_r = (node.height - unit * 2 / 3) / 2;
        let badge_cx = node.x + badge_r + unit / 3;
        let badge_cy = node.y + node.height / 2;
        let orb_col = if seq % 2 == 0 { AZURE_CORE } else { AQUA_CORE };
        draw_dot(canvas, badge_cx, badge_cy, badge_r, orb_col, 255);
        draw_icon(canvas, info.icon, badge_cx, badge_cy, unit, WHITE);

        let text_x = badge_cx + badge_r + unit / 3;
        let text_y = node.y + (node.height - label_style.cap_height) / 2;
        draw_text_color(canvas, &info.name, text_x, text_y, &label_style, INK_DARK);
    }

    draw_aero_glass_surface(canvas, head_pod, head_pod.height / 2, blur_r, 232, 44);
    draw_dot(
        canvas,
        head_pod.x + unit * 3 / 4,
        head_pod.y + head_pod.height / 2,
        4,
        AQUA_CORE,
        255,
    );
    draw_text_color(
        canvas,
        "NetizenCore",
        head_pod.x + unit * 3 / 2,
        head_pod.y + (head_pod.height - sub_style.cap_height) / 2,
        &sub_style,
        INK_DARK,
    );

    let (reb_btn, shut_btn) = snake_power_buttons(head_pod, unit);
    let reb_hov = state.hover == Target::PowerReboot;
    let shut_hov = state.hover == Target::PowerShutdown;

    draw_panel(
        canvas,
        reb_btn,
        reb_btn.height / 2,
        if reb_hov { WARN_AMBER } else { WHITE },
        245,
        WARN_AMBER,
        220,
    );
    draw_reboot_symbol(
        canvas,
        reb_btn.x + reb_btn.width / 2,
        reb_btn.y + reb_btn.height / 2,
        (unit / 3).max(4),
        if reb_hov { INK_DARK } else { WARN_AMBER },
    );

    draw_panel(
        canvas,
        shut_btn,
        shut_btn.height / 2,
        if shut_hov { DANGER_CORAL } else { WHITE },
        245,
        DANGER_CORAL,
        220,
    );
    draw_power_symbol(
        canvas,
        shut_btn.x + shut_btn.width / 2,
        shut_btn.y + shut_btn.height / 2,
        (unit / 3).max(4),
        if shut_hov { INK_DARK } else { DANGER_CORAL },
    );
}

pub fn draw_context_menu(
    canvas: &mut Canvas<'_>,
    state: &DesktopState,
    pos: (i32, i32),
    kind: ContextMenuKind,
    width: i32,
    height: i32,
    unit: i32,
) {
    let item_count = state.context_menu_item_count(kind);
    let menu = context_menu_rect(pos.0, pos.1, item_count, width, height, unit);
    draw_aero_glass_surface(canvas, menu, unit * 2 / 3, state.blur_radius(12), 236, 56);

    let cap_style = text_style((unit * 46 / 100).max(9), 18);
    let item_style = text_style((unit * 54 / 100).max(10), 15);

    let header_title = match kind {
        ContextMenuKind::DesktopSurface => "MASAUSTU VE SISTEM MENUSU",
        ContextMenuKind::AppItem(idx) => {
            if idx < state.apps.len() {
                &state.apps[idx].app.info().name
            } else {
                "UYGULAMA MENUSU"
            }
        }
        ContextMenuKind::FsItem(idx) => {
            if idx < state.fs_items.len() {
                &state.fs_items[idx].name
            } else {
                "DOSYA MENUSU"
            }
        }
    };

    draw_dot(canvas, menu.x + unit, menu.y + unit, 4, AQUA_CORE, 255);
    draw_text_color(
        canvas,
        header_title,
        menu.x + unit * 7 / 4,
        menu.y + unit * 2 / 3,
        &cap_style,
        AZURE_DEEP,
    );
    canvas.fill_rect(
        menu.x + unit / 2,
        menu.y + unit * 7 / 4,
        menu.width - unit,
        1,
        GLASS_BORDER,
    );

    for idx in 0..item_count {
        let item_r = context_item_rect(menu, idx, unit);
        let is_hov = state.hover == Target::ContextItem(idx);
        let (label, badge, dot_color) = state.context_menu_entry(kind, idx);

        if is_hov {
            draw_panel(
                canvas,
                item_r,
                unit / 3,
                AQUA_SOFT,
                125,
                AZURE_CORE,
                210,
            );
        }

        draw_dot(
            canvas,
            item_r.x + unit * 2 / 3,
            item_r.y + item_r.height / 2,
            4,
            dot_color,
            255,
        );
        let ty = item_r.y + (item_r.height - item_style.cap_height) / 2;
        draw_text_color(
            canvas,
            label,
            item_r.x + unit * 3 / 2,
            ty,
            &item_style,
            INK_DARK,
        );

        if !badge.is_empty() {
            let bw = text_width(badge, &cap_style);
            draw_text_color(
                canvas,
                badge,
                item_r.x + item_r.width - bw - unit / 2,
                item_r.y + (item_r.height - cap_style.cap_height) / 2,
                &cap_style,
                INK_MUTED,
            );
        }
    }
}

pub fn draw_desktop_modal(
    canvas: &mut Canvas<'_>,
    state: &DesktopState,
    width: i32,
    height: i32,
    unit: i32,
) {
    let (title, subtitle, text_val, tall) = match state.modal {
        DesktopModal::None => return,
        DesktopModal::CreateFolder { ref name_buf } => (
            "Masaustunde Yeni Klasor Olustur",
            "Konum: /data/desktop (Klavye ile yazip Enter veya Kaydet'e basin)",
            name_buf.as_str(),
            false,
        ),
        DesktopModal::CreateTextFile { ref name_buf } => (
            "Masaustunde Yeni Metin Belgesi",
            "Konum: /data/desktop (Ornek: notlarim.txt)",
            name_buf.as_str(),
            false,
        ),
        DesktopModal::RenameItem {
            ref old_name,
            ref new_buf,
            ..
        } => (
            "Masaustu Ogesini Yeniden Adlandir",
            old_name.as_str(),
            new_buf.as_str(),
            false,
        ),
        DesktopModal::EditTextFile {
            ref name,
            ref content,
        } => (
            "Masaustu Hizli Not Duzenleyici",
            name.as_str(),
            content.as_str(),
            true,
        ),
    };

    let m_rect = desktop_modal_rect(width, height, unit, tall);
    draw_aero_glass_surface(canvas, m_rect, unit * 3 / 4, state.blur_radius(12), 240, 64);

    let title_st = text_style((unit * 72 / 100).max(11), 16);
    let sub_st = text_style((unit * 48 / 100).max(9), 15);
    let body_st = text_style((unit * 56 / 100).max(10), 15);

    draw_dot(
        canvas,
        m_rect.x + unit * 5 / 4,
        m_rect.y + unit * 3 / 2,
        5,
        AQUA_CORE,
        255,
    );
    draw_text_color(
        canvas,
        title,
        m_rect.x + unit * 2,
        m_rect.y + unit,
        &title_st,
        INK_DARK,
    );
    draw_text_color(
        canvas,
        subtitle,
        m_rect.x + unit * 2,
        m_rect.y + unit * 5 / 2,
        &sub_st,
        AZURE_DEEP,
    );

    let box_h = if tall {
        m_rect.height - unit * 8
    } else {
        unit * 5 / 2
    };
    let input_r = Rect::new(
        m_rect.x + unit * 3 / 2,
        m_rect.y + unit * 4,
        m_rect.width - unit * 3,
        box_h,
    );
    draw_panel(canvas, input_r, unit / 3, WHITE, 252, AZURE_CORE, 240);

    if tall {
        let step_y = (body_st.cap_height + 6).max(14);
        let max_lines = ((input_r.height - 12) / step_y).max(1) as usize;
        let total = text_val.lines().count().max(1);
        let skip = total.saturating_sub(max_lines);
        for (i, line) in text_val.lines().skip(skip).take(max_lines).enumerate() {
            let ly = input_r.y + 8 + (i as i32) * step_y;
            draw_text_color(canvas, line, input_r.x + 10, ly, &body_st, INK_DARK);
            if skip + i + 1 == total {
                let lw = text_width(line, &body_st);
                canvas.fill_rect(
                    input_r.x + 12 + lw,
                    ly - 1,
                    2,
                    body_st.cap_height + 2,
                    AQUA_CORE,
                );
            }
        }
        if text_val.is_empty() {
            canvas.fill_rect(
                input_r.x + 10,
                input_r.y + 8,
                2,
                body_st.cap_height + 2,
                AQUA_CORE,
            );
        }
    } else {
        let ty = input_r.y + (input_r.height - body_st.cap_height) / 2;
        draw_text_color(canvas, text_val, input_r.x + 12, ty, &body_st, INK_DARK);
        let tw = text_width(text_val, &body_st);
        canvas.fill_rect(
            input_r.x + 14 + tw,
            ty - 1,
            2,
            body_st.cap_height + 2,
            AQUA_CORE,
        );
    }

    let btn_w = unit * 7;
    let btn_h = unit * 2;
    let btn_y = m_rect.y + m_rect.height - btn_h - unit;
    let ok_r = Rect::new(
        m_rect.x + m_rect.width - btn_w - unit * 3 / 2,
        btn_y,
        btn_w,
        btn_h,
    );
    let cancel_r = Rect::new(ok_r.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

    let cancel_hov = state.hover == Target::ModalCancel;
    let ok_hov = state.hover == Target::ModalConfirm;

    draw_panel(
        canvas,
        cancel_r,
        cancel_r.height / 2,
        if cancel_hov { AQUA_SOFT } else { WHITE },
        245,
        GLASS_BORDER,
        220,
    );
    let c_tw = text_width("Vazgec", &body_st);
    draw_text_color(
        canvas,
        "Vazgec",
        cancel_r.x + (cancel_r.width - c_tw) / 2,
        cancel_r.y + (cancel_r.height - body_st.cap_height) / 2,
        &body_st,
        INK_DARK,
    );

    draw_panel(
        canvas,
        ok_r,
        ok_r.height / 2,
        if ok_hov { AQUA_DEEP } else { AZURE_CORE },
        255,
        AZURE_DEEP,
        255,
    );
    let ok_tw = text_width("Kaydet", &body_st);
    draw_text_color(
        canvas,
        "Kaydet",
        ok_r.x + (ok_r.width - ok_tw) / 2,
        ok_r.y + (ok_r.height - body_st.cap_height) / 2,
        &body_st,
        WHITE,
    );
}

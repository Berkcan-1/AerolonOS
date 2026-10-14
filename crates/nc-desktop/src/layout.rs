use nc_gfx::Rect;

pub const SNAKE_COLS: usize = 3;

pub fn unit_for(height: i32) -> i32 {
    (height / 36).max(10)
}

pub fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

pub fn dock_height(unit: i32) -> i32 {
    unit * 3
}

pub fn dock_rect(width: i32, height: i32, unit: i32, item_count: usize, reveal256: u32) -> Rect {
    let h = dock_height(unit);
    let apps_w = (item_count.max(1) as i32) * (unit * 3 + unit / 2);
    let tray_w = unit * 15;
    let start_w = unit * 6;
    let total_w = (start_w + apps_w + tray_w + unit * 3)
        .min(width - unit * 2)
        .max(unit * 26);
    let x = (width - total_w) / 2;
    let full_y = height - h - unit / 2;
    let hidden_y = height + 4;
    let y = hidden_y - ((hidden_y - full_y) * (reveal256.min(256) as i32) / 256);
    Rect::new(x, y, total_w, h)
}

pub fn dock_sensor_strip(width: i32, height: i32, unit: i32) -> Rect {
    let w = (width * 36 / 100).max(unit * 16);
    Rect::new((width - w) / 2, height - 5, w, 5)
}

pub fn dock_start_orb(dock: Rect, unit: i32) -> Rect {
    Rect::new(
        dock.x + unit / 2,
        dock.y + unit / 3,
        unit * 11 / 2,
        dock.height - unit * 2 / 3,
    )
}

pub fn dock_app_pill(dock: Rect, slot_idx: usize, unit: i32) -> Rect {
    let start = dock_start_orb(dock, unit);
    let step = unit * 3 + unit / 2;
    let x = start.x + start.width + unit + (slot_idx as i32) * step;
    Rect::new(x, dock.y + unit / 3, unit * 3, dock.height - unit * 2 / 3)
}

pub fn snake_node_rect(seq_index: usize, dock: Rect, unit: i32, anim256: u32) -> Rect {
    let row = seq_index / SNAKE_COLS;
    let pos_in_row = seq_index % SNAKE_COLS;
    let col = if row % 2 == 0 {
        pos_in_row
    } else {
        (SNAKE_COLS - 1) - pos_in_row
    };
    let node_w = unit * 8 + unit / 2;
    let node_h = unit * 2 + unit * 2 / 3;
    let gap_x = unit * 3 / 4;
    let gap_y = unit * 3 / 4;

    let base_x = dock.x;
    let target_x = base_x + (col as i32) * (node_w + gap_x);
    let target_y = dock.y - unit - ((row as i32 + 1) * (node_h + gap_y));
    let drop = (256 - anim256.min(256)) as i32 * unit / 128;

    Rect::new(target_x, target_y + drop, node_w, node_h)
}

pub fn snake_head_pod_rect(total_nodes: usize, dock: Rect, unit: i32, anim256: u32) -> Rect {
    let last_idx = total_nodes.saturating_sub(1);
    let top_row = if total_nodes == 0 {
        0
    } else {
        last_idx / SNAKE_COLS
    };
    let node_w = unit * 8 + unit / 2;
    let node_h = unit * 2 + unit * 2 / 3;
    let gap_x = unit * 3 / 4;
    let gap_y = unit * 3 / 4;

    let pod_w = node_w * 2 + gap_x;
    let pod_h = unit * 2 + unit / 2;
    let x = if top_row % 2 == 0 {
        dock.x + (SNAKE_COLS as i32) * (node_w + gap_x) - gap_x - pod_w
    } else {
        dock.x
    };
    let top_node_y = dock.y - unit - ((top_row as i32 + 1) * (node_h + gap_y));
    let drop = (256 - anim256.min(256)) as i32 * unit / 128;
    Rect::new(x, top_node_y - pod_h - gap_y + drop, pod_w, pod_h)
}

pub fn snake_power_buttons(pod: Rect, unit: i32) -> (Rect, Rect) {
    let btn_size = pod.height - unit / 2;
    let shut_r = Rect::new(
        pod.x + pod.width - btn_size - unit / 3,
        pod.y + unit / 4,
        btn_size,
        btn_size,
    );
    let reb_r = Rect::new(
        shut_r.x - btn_size - unit / 3,
        pod.y + unit / 4,
        btn_size,
        btn_size,
    );
    (reb_r, shut_r)
}

pub fn desktop_icon(index: usize, height: i32, unit: i32) -> Rect {
    let room = height - unit * 5;
    let per_column = (room / (unit * 6 + unit / 2)).max(1) as usize;
    let column = (index / per_column) as i32;
    let row = (index % per_column) as i32;
    Rect::new(
        unit * 3 / 2 + column * (unit * 7 + unit / 2),
        unit * 3 / 2 + row * (unit * 6 + unit / 2),
        unit * 7,
        unit * 6,
    )
}

pub fn icon_tile(cell: Rect, unit: i32) -> Rect {
    let size = unit * 7 / 2;
    Rect::new(cell.x + (cell.width - size) / 2, cell.y + unit / 3, size, size)
}

pub fn animated_window_rect(target: Rect, anim256: u32, unit: i32) -> Rect {
    let a = anim256.min(256) as i32;
    if a >= 256 {
        return target;
    }
    let scale256 = 188 + (a * 68 / 256);
    let w = (target.width * scale256 / 256).max(unit * 10);
    let h = (target.height * scale256 / 256).max(unit * 6);
    let cx = target.x + target.width / 2;
    let cy = target.y + target.height / 2 + ((256 - a) * unit / 128);
    Rect::new(cx - w / 2, cy - h / 2, w, h)
}

pub fn launch_toast_rect(width: i32, height: i32, unit: i32) -> Rect {
    let w = unit * 18;
    let h = unit * 3;
    Rect::new((width - w) / 2, height - unit * 8, w, h)
}

pub fn task_manager_rect(x: i32, y: i32, unit: i32) -> Rect {
    Rect::new(x, y, unit * 34, unit * 23)
}

pub fn task_manager_close_rect(tm: Rect, unit: i32) -> Rect {
    Rect::new(
        tm.x + tm.width - unit * 3 - unit / 2,
        tm.y + unit / 2,
        unit * 3,
        title_height(unit) - unit,
    )
}

pub fn task_manager_row_rect(tm: Rect, idx: usize, unit: i32) -> Rect {
    let top = tm.y + unit * 9 + (idx as i32) * (unit * 3 / 2);
    Rect::new(tm.x + unit, top, tm.width - unit * 2, unit * 4 / 3)
}

pub fn task_manager_action_btn(tm: Rect, primary: bool, unit: i32) -> Rect {
    let w = unit * 10;
    let h = unit * 2;
    let y = tm.y + tm.height - h - unit * 3 / 4;
    if primary {
        Rect::new(tm.x + tm.width - w - unit, y, w, h)
    } else {
        Rect::new(tm.x + unit, y, w, h)
    }
}

pub fn title_height(unit: i32) -> i32 {
    unit * 5 / 2
}

pub fn title_bar(win: Rect, unit: i32) -> Rect {
    Rect::new(win.x, win.y, win.width, title_height(unit))
}

pub fn content_area(win: Rect, unit: i32) -> Rect {
    let title = title_height(unit);
    Rect::new(win.x, win.y + title, win.width, win.height - title)
}

pub fn close_button(win: Rect, unit: i32) -> Rect {
    Rect::new(
        win.x + win.width - unit * 3 - unit / 2,
        win.y + unit / 2,
        unit * 3,
        title_height(unit) - unit,
    )
}

pub fn max_button(win: Rect, unit: i32) -> Rect {
    let close = close_button(win, unit);
    Rect::new(close.x - close.width - unit / 4, close.y, close.width, close.height)
}

pub fn min_button(win: Rect, unit: i32) -> Rect {
    let max_b = max_button(win, unit);
    Rect::new(max_b.x - max_b.width - unit / 4, max_b.y, max_b.width, max_b.height)
}

pub fn context_menu_rect(
    x: i32,
    y: i32,
    item_count: usize,
    width: i32,
    height: i32,
    unit: i32,
) -> Rect {
    let menu_w = unit * 19;
    let header_h = unit * 2;
    let row_step = unit * 2 + unit / 5;
    let menu_h = header_h + (item_count as i32) * row_step + unit * 3 / 4;
    let clamped_x = x.min(width - menu_w - unit / 2).max(unit / 2);
    let clamped_y = y.min(height - menu_h - unit / 2).max(unit / 2);
    Rect::new(clamped_x, clamped_y, menu_w, menu_h)
}

pub fn context_item_rect(menu: Rect, index: usize, unit: i32) -> Rect {
    let header_h = unit * 2;
    let row_step = unit * 2 + unit / 5;
    Rect::new(
        menu.x + unit / 2,
        menu.y + header_h + (index as i32) * row_step,
        menu.width - unit,
        unit * 2,
    )
}

pub fn desktop_modal_rect(width: i32, height: i32, unit: i32, tall: bool) -> Rect {
    let w = (unit * 28).min(width - unit * 2);
    let h = if tall {
        (unit * 18).min(height - unit * 2)
    } else {
        (unit * 11).min(height - unit * 2)
    };
    Rect::new((width - w) / 2, (height - h) / 2, w, h)
}

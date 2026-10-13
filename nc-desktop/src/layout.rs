use nc_gfx::Rect;

pub fn unit_for(height: i32) -> i32 {
    (height / 36).max(10)
}

pub fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

pub fn taskbar_height(unit: i32) -> i32 {
    unit * 5 / 2
}

pub fn taskbar_rect(width: i32, height: i32, unit: i32) -> Rect {
    let bar = taskbar_height(unit);
    Rect::new(0, height - bar, width, bar)
}

pub fn start_button(height: i32, unit: i32) -> Rect {
    let bar = taskbar_height(unit);
    Rect::new(unit / 2, height - bar + unit / 2, unit * 7, bar - unit)
}

pub fn task_button(slot: i32, height: i32, unit: i32) -> Rect {
    let start = start_button(height, unit);
    Rect::new(start.x + start.width + unit + slot * (unit * 9 + unit / 2), start.y, unit * 9, start.height)
}

pub fn visible_start_apps(app_count: usize, height: i32, unit: i32) -> usize {
    let room = height - taskbar_height(unit) - unit * 2 - unit * 11;
    let fit = (room / (unit * 3)).max(1) as usize;
    app_count.min(fit)
}

pub fn start_menu(visible: usize, height: i32, unit: i32) -> Rect {
    let menu_height = unit * 14 + visible as i32 * unit * 3;
    Rect::new(unit / 2, height - taskbar_height(unit) - menu_height - unit / 2, unit * 28, menu_height)
}

pub fn start_item(index: usize, visible: usize, height: i32, unit: i32) -> Rect {
    let menu = start_menu(visible, height, unit);
    let item_height = unit * 3;
    let first = menu.y + unit * 3;
    let slot = index as i32;
    let y = if index < visible {
        first + slot * item_height
    } else {
        first + visible as i32 * item_height + unit + (slot - visible as i32) * item_height
    };
    Rect::new(menu.x + unit / 2, y, unit * 12, item_height)
}

pub fn separator_y(visible: usize, height: i32, unit: i32) -> i32 {
    start_menu(visible, height, unit).y + unit * 3 + visible as i32 * unit * 3 + unit / 2
}

pub fn desktop_icon(index: usize, height: i32, unit: i32) -> Rect {
    let room = height - taskbar_height(unit) - unit * 4;
    let per_column = (room / (unit * 7)).max(1) as usize;
    let column = (index / per_column) as i32;
    let row = (index % per_column) as i32;
    Rect::new(unit * 2 + column * unit * 8, unit * 2 + row * unit * 7, unit * 7, unit * 6)
}

pub fn icon_tile(cell: Rect, unit: i32) -> Rect {
    let size = unit * 7 / 2;
    Rect::new(cell.x + (cell.width - size) / 2, cell.y + unit / 2, size, size)
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
    Rect::new(win.x + win.width - unit * 3 - unit / 2, win.y + unit / 2, unit * 3, title_height(unit) - unit)
}

pub fn max_button(win: Rect, unit: i32) -> Rect {
    let close = close_button(win, unit);
    Rect::new(close.x - close.width - unit / 4, close.y, close.width, close.height)
}

pub fn min_button(win: Rect, unit: i32) -> Rect {
    let max_b = max_button(win, unit);
    Rect::new(max_b.x - max_b.width - unit / 4, max_b.y, max_b.width, max_b.height)
}

pub fn context_menu_rect(x: i32, y: i32, width: i32, height: i32, unit: i32) -> Rect {
    let menu_w = unit * 16;
    let menu_h = unit * 10;
    let clamped_x = x.min(width - menu_w - unit / 2).max(unit / 2);
    let clamped_y = y.min(height - taskbar_height(unit) - menu_h - unit / 2).max(unit / 2);
    Rect::new(clamped_x, clamped_y, menu_w, menu_h)
}

pub fn context_item_rect(menu: Rect, index: usize, unit: i32) -> Rect {
    Rect::new(menu.x + unit / 2, menu.y + unit / 2 + index as i32 * (unit * 2 + unit / 4), menu.width - unit, unit * 2)
}

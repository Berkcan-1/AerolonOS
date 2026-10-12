use nc_gfx::{Canvas, Rect};
use nc_ui::{
    draw_badge, draw_card, draw_dot, draw_panel, draw_text_color, text_style, ACCENT_BG,
    ACCENT_LINE, PANEL_BORDER, SUCCESS, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY, WHITE,
};

pub fn draw(canvas: &mut Canvas<'_>, width: i32, height: i32) {
    let unit = (height / 36).max(10);
    let card_w = unit * 36;
    let card_h = unit * 21;
    let card = Rect::new((width - card_w) / 2, (height - card_h) / 2 - unit, card_w, card_h);
    draw_card(canvas, card, unit);

    let padding = unit * 2;
    let title_style = text_style(unit * 11 / 10, 20);
    draw_text_color(canvas, "STORAGE MOUNT POINTS", card.x + padding, card.y + padding, &title_style, TEXT_PRIMARY);

    let sub_style = text_style(unit * 6 / 10, 30);
    draw_text_color(canvas, "EROFS AND READ-WRITE PARTITIONS", card.x + padding, card.y + padding + unit * 2, &sub_style, TEXT_MUTED);

    let rows = [
        ("/SYSTEM", "EROFS", "READ ONLY", "CORE OS SYSTEM FILES"),
        ("/SECURITY", "EROFS", "READ ONLY", "SECURITY KEYS AND POLICIES"),
        ("/DATA", "RW", "READ WRITE", "USER APPS AND SETTINGS"),
        ("/KERNEL", "EROFS", "BOOT", "KERNEL IMAGE ARCHIVE"),
    ];

    let row_h = unit * 3;
    let start_y = card.y + padding + unit * 4;
    for i in 0..rows.len() {
        let py = start_y + i as i32 * (row_h + unit / 2);
        let row_box = Rect::new(card.x + padding, py, card_w - padding * 2, row_h);
        let (mount, fs, mode, desc) = rows[i];

        draw_panel(canvas, row_box, unit / 2, ACCENT_BG, 255, PANEL_BORDER, 255);
        draw_dot(canvas, row_box.x + unit * 3 / 2, row_box.y + row_h / 2, unit * 3 / 10, ACCENT_LINE, 255);

        let row_style = text_style(unit * 6 / 10, 30);
        let ty = row_box.y + (row_h - row_style.cap_height) / 2;
        draw_text_color(canvas, mount, row_box.x + unit * 3, ty, &row_style, TEXT_PRIMARY);

        let badge_rect = Rect::new(row_box.x + unit * 12, row_box.y + (row_h - unit * 3 / 2) / 2, unit * 4, unit * 3 / 2);
        let badge_bg = if i == 2 { SUCCESS } else { ACCENT_LINE };
        draw_badge(canvas, badge_rect, fs, badge_bg, WHITE);

        draw_text_color(canvas, mode, row_box.x + unit * 17, ty, &row_style, TEXT_MUTED);
        draw_text_color(canvas, desc, row_box.x + unit * 24, ty, &row_style, TEXT_DIM);
    }
}
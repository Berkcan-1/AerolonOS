#![no_std]

use nc_font::text_width;
use nc_gfx::{Canvas, Rect};
use nc_hal::{BlockDeviceInfo, DeviceType, KeyCode, MouseState};
use nc_ui::{
    draw_backdrop, draw_badge, draw_button, draw_card, draw_dot, draw_panel, draw_radio,
    draw_text_color, text_style, ACCENT_BG, ACCENT_LINE, PANEL_BORDER, PANEL_FILL, RAIL,
    RAIL_DOT, SUCCESS, SUCCESS_BG, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY, WHITE,
};

const STEPS: [&str; 4] = ["WELCOME", "DISKS", "INSTALL", "FINISH"];
const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAX_SETUP_DISKS: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetupAction {
    None,
    Shutdown,
    Reboot,
}

pub struct SetupState {
    pub step: usize,
    pub selected_disk: usize,
    pub disks: [BlockDeviceInfo; MAX_SETUP_DISKS],
    pub disk_count: usize,
    pub progress: u32,
    pub changed: bool,
}

struct CardLayout {
    card: Rect,
    padding: i32,
    content_y: i32,
    footer_y: i32,
    status_badge: Rect,
    plat_x: i32,
    plat_y: i32,
    next_btn: Rect,
    back_btn: Rect,
}

fn compute_card_layout(width: i32, height: i32, unit: i32) -> CardLayout {
    let margin = unit * 4;
    let card_x = margin + unit * 16;
    let card_y = unit * 5;
    let card_w = width - card_x - margin;
    let card_h = height - unit * 10;
    let card = Rect::new(card_x, card_y, card_w, card_h);

    let padding = unit * 2;
    let content_y = card.y + padding + unit * 4;

    let footer_h = unit * 4;
    let footer_y = card.y + card.height - footer_h;

    let btn_h = unit * 2;
    let btn_y = footer_y + (footer_h - btn_h) / 2;
    let next_btn = Rect::new(card.x + card.width - padding - unit * 8, btn_y, unit * 8, btn_h);
    let back_btn = Rect::new(next_btn.x - unit * 6 - unit / 2, btn_y, unit * 6, btn_h);

    let badge_h = unit * 3 / 2;
    let badge_w = unit * 5;
    let badge_y = footer_y + (footer_h - badge_h) / 2;
    let status_badge = Rect::new(card.x + padding, badge_y, badge_w, badge_h);

    let plat_x = status_badge.x + status_badge.width + unit;
    let plat_y = footer_y + (footer_h - (unit * 6 / 10)) / 2;

    CardLayout {
        card,
        padding,
        content_y,
        footer_y,
        status_badge,
        plat_x,
        plat_y,
        next_btn,
        back_btn,
    }
}

impl SetupState {
    pub fn new(disks_slice: &[BlockDeviceInfo]) -> SetupState {
        let mut disks = [BlockDeviceInfo::EMPTY; MAX_SETUP_DISKS];
        let count = disks_slice.len().min(MAX_SETUP_DISKS);
        for i in 0..count {
            disks[i] = disks_slice[i];
        }
        SetupState {
            step: 0,
            selected_disk: 0,
            disks,
            disk_count: count,
            progress: 0,
            changed: true,
        }
    }

    pub fn update(&mut self, mouse: &MouseState, key: Option<KeyCode>, width: i32, height: i32) -> SetupAction {
        let unit = (height / 36).max(10);
        let layout = compute_card_layout(width, height, unit);

        if let Some(k) = key {
            match k {
                KeyCode::Enter | KeyCode::Space => {
                    if self.step < 3 {
                        self.step += 1;
                        self.changed = true;
                    } else {
                        return SetupAction::Reboot;
                    }
                }
                KeyCode::Escape => {
                    if self.step > 0 && self.step < 3 {
                        self.step -= 1;
                        self.changed = true;
                    }
                }
                KeyCode::Up => {
                    if self.step == 1 && self.selected_disk > 0 {
                        self.selected_disk -= 1;
                        self.changed = true;
                    }
                }
                KeyCode::Down => {
                    if self.step == 1 && self.selected_disk + 1 < self.disk_count {
                        self.selected_disk += 1;
                        self.changed = true;
                    }
                }
                _ => {}
            }
        }

        if mouse.left_clicked {
            if point_in_rect(mouse.x, mouse.y, layout.next_btn) {
                if self.step < 3 {
                    self.step += 1;
                    self.changed = true;
                } else {
                    return SetupAction::Reboot;
                }
            } else if self.step > 0 && self.step < 3 && point_in_rect(mouse.x, mouse.y, layout.back_btn) {
                self.step -= 1;
                self.changed = true;
            } else if self.step == 1 {
                let item_h = unit * 3;
                let item_gap = unit / 2;
                for i in 0..self.disk_count {
                    let item_y = layout.content_y + i as i32 * (item_h + item_gap);
                    let item_area = Rect::new(layout.card.x + layout.padding, item_y, layout.card.width - layout.padding * 2, item_h);
                    if point_in_rect(mouse.x, mouse.y, item_area) {
                        self.selected_disk = i;
                        self.changed = true;
                    }
                }
            }
        }

        if self.step == 2 && self.progress < 256 {
            self.progress = (self.progress + 4).min(256);
            self.changed = true;
            if self.progress == 256 {
                self.step = 3;
            }
        }

        SetupAction::None
    }
}

fn point_in_rect(x: i32, y: i32, r: Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

fn format_size(mb: u64, buf: &mut [u8]) -> usize {
    let (val, unit) = if mb >= 1024 { (mb / 1024, b" GB") } else { (mb, b" MB") };
    let mut temp = [0u8; 16];
    let mut tlen = 0;
    let mut n = val;
    if n == 0 {
        temp[0] = b'0';
        tlen = 1;
    } else {
        while n > 0 {
            temp[tlen] = b'0' + (n % 10) as u8;
            n /= 10;
            tlen += 1;
        }
    }
    let mut len = 0;
    for i in 0..tlen {
        buf[len] = temp[tlen - 1 - i];
        len += 1;
    }
    for b in unit {
        buf[len] = *b;
        len += 1;
    }
    buf[len] = 0;
    len
}

pub fn draw(canvas: &mut Canvas<'_>, platform: &str, state: &SetupState, mouse: &MouseState) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let unit = (height / 36).max(10);
    let margin = unit * 4;

    draw_backdrop(canvas);

    let brand_style = text_style(unit * 11 / 10, 30);
    draw_text_color(canvas, "NETIZENCORE", margin, unit * 2, &brand_style, TEXT_PRIMARY);

    let tag_style = text_style(unit * 7 / 10, 40);
    let tag_width = text_width("SETUP WIZARD", &tag_style);
    draw_text_color(
        canvas,
        "SETUP WIZARD",
        width - margin - tag_width,
        unit * 2 + (brand_style.cap_height - tag_style.cap_height),
        &tag_style,
        TEXT_DIM,
    );

    let step_style = text_style(unit * 7 / 10, 30);
    let rail_x = margin + unit;
    let first_center = unit * 8 + step_style.cap_height / 2;
    let last_center = first_center + (STEPS.len() as i32 - 1) * unit * 4;
    canvas.fill_rect(rail_x - 1, first_center, 2, last_center - first_center, RAIL);

    for index in 0..STEPS.len() {
        let y = unit * 8 + index as i32 * unit * 4;
        let center_y = y + step_style.cap_height / 2;
        let label_x = margin + unit * 3;
        if index == state.step {
            draw_dot(canvas, rail_x, center_y, unit * 2 / 5, ACCENT_LINE, 255);
            draw_text_color(canvas, STEPS[index], label_x, y, &step_style, TEXT_PRIMARY);
        } else if index < state.step {
            draw_dot(canvas, rail_x, center_y, unit * 3 / 10, SUCCESS, 255);
            draw_text_color(canvas, STEPS[index], label_x, y, &step_style, TEXT_MUTED);
        } else {
            draw_dot(canvas, rail_x, center_y, unit * 1 / 4, RAIL_DOT, 255);
            draw_text_color(canvas, STEPS[index], label_x, y, &step_style, TEXT_DIM);
        }
    }

    let layout = compute_card_layout(width, height, unit);
    draw_card(canvas, layout.card, unit);

    let caption_style = text_style(unit * 6 / 10, 40);
    let title_style = text_style(unit * 12 / 10, 20);
    let body_style = text_style(unit * 7 / 10, 25);

    let step_label = match state.step {
        0 => "STEP 1 OF 4",
        1 => "STEP 2 OF 4",
        2 => "STEP 3 OF 4",
        _ => "STEP 4 OF 4",
    };
    draw_text_color(canvas, step_label, layout.card.x + layout.padding, layout.card.y + layout.padding, &caption_style, TEXT_MUTED);

    let title_y = layout.card.y + layout.padding + unit * 2;

    match state.step {
        0 => {
            draw_text_color(canvas, "WELCOME TO NETIZENCORE", layout.card.x + layout.padding, title_y, &title_style, TEXT_PRIMARY);
            draw_text_color(canvas, "A CLEAN, CUSTOM BUILT OPERATING SYSTEM.", layout.card.x + layout.padding, layout.content_y, &body_style, TEXT_MUTED);
            draw_text_color(canvas, "CLICK NEXT TO SCAN AND SELECT INSTALLATION STORAGE.", layout.card.x + layout.padding, layout.content_y + unit * 3 / 2, &body_style, TEXT_MUTED);

            let sys_box_y = layout.content_y + unit * 4;
            let sys_box_h = unit * 3;
            let sys_box = Rect::new(layout.card.x + layout.padding, sys_box_y, layout.card.width - layout.padding * 2, sys_box_h);
            draw_panel(canvas, sys_box, unit / 2, ACCENT_BG, 255, PANEL_BORDER, 255);
            draw_text_color(canvas, "SYSTEM READY FOR SETUP", sys_box.x + unit * 2, sys_box_y + unit * 3 / 4, &caption_style, ACCENT_LINE);
            draw_text_color(canvas, "KERNEL AND LOW LEVEL DRIVERS INITIALIZED.", sys_box.x + unit * 2, sys_box_y + unit * 7 / 4, &caption_style, TEXT_MUTED);
        }
        1 => {
            draw_text_color(canvas, "SELECT TARGET STORAGE", layout.card.x + layout.padding, title_y, &title_style, TEXT_PRIMARY);

            if state.disk_count == 0 {
                draw_text_color(canvas, "NO STORAGE DRIVES DETECTED.", layout.card.x + layout.padding, layout.content_y, &body_style, TEXT_MUTED);
            } else {
                let item_h = unit * 3;
                let item_gap = unit / 2;
                for i in 0..state.disk_count {
                    let d = &state.disks[i];
                    let item_y = layout.content_y + i as i32 * (item_h + item_gap);
                    let item_area = Rect::new(layout.card.x + layout.padding, item_y, layout.card.width - layout.padding * 2, item_h);
                    let is_sel = i == state.selected_disk;
                    let is_hov = point_in_rect(mouse.x, mouse.y, item_area);

                    let (bg, border) = if is_sel {
                        (ACCENT_BG, ACCENT_LINE)
                    } else if is_hov {
                        (PANEL_FILL, ACCENT_LINE)
                    } else {
                        (PANEL_FILL, PANEL_BORDER)
                    };

                    draw_panel(canvas, item_area, unit / 2, bg, 255, border, 255);
                    draw_radio(canvas, item_area.x + unit * 2, item_area.y + item_area.height / 2, unit * 3 / 10, is_sel);

                    let badge_area = Rect::new(item_area.x + unit * 4, item_area.y + (item_h - unit * 3 / 2) / 2, unit * 4, unit * 3 / 2);
                    let (badge_bg, badge_fg) = if d.device_type == DeviceType::OpticalDisc {
                        (PANEL_BORDER, TEXT_MUTED)
                    } else {
                        (ACCENT_LINE, WHITE)
                    };
                    draw_badge(canvas, badge_area, d.type_label(), badge_bg, badge_fg);

                    let text_y = item_area.y + (item_area.height - caption_style.cap_height) / 2;
                    draw_text_color(canvas, d.model_str(), item_area.x + unit * 9, text_y, &caption_style, TEXT_PRIMARY);

                    if d.device_type == DeviceType::OpticalDisc {
                        let note_w = text_width("INSTALL MEDIA", &caption_style);
                        draw_text_color(canvas, "INSTALL MEDIA", item_area.x + item_area.width - unit * 2 - note_w, text_y, &caption_style, TEXT_DIM);
                    } else {
                        let mut sbuf = [0u8; 24];
                        let slen = format_size(d.size_mb(), &mut sbuf);
                        if let Ok(st) = core::str::from_utf8(&sbuf[..slen]) {
                            let sz_w = text_width(st, &caption_style);
                            draw_text_color(canvas, st, item_area.x + item_area.width - unit * 2 - sz_w, text_y, &caption_style, TEXT_MUTED);
                        }
                    }
                }
            }
        }
        2 => {
            draw_text_color(canvas, "INSTALLING SYSTEM", layout.card.x + layout.padding, title_y, &title_style, TEXT_PRIMARY);
            draw_text_color(canvas, "WRITING MBR BOOTLOADER AND KERNEL TO DISK...", layout.card.x + layout.padding, layout.content_y, &body_style, TEXT_MUTED);

            let bar_bg = Rect::new(layout.card.x + layout.padding, layout.content_y + unit * 3, layout.card.width - layout.padding * 2, unit * 3 / 2);
            draw_panel(canvas, bar_bg, bar_bg.height / 2, PANEL_BORDER, 255, PANEL_BORDER, 255);
            let fill_w = (bar_bg.width as u64 * state.progress as u64 / 256) as i32;
            if fill_w > bar_bg.height {
                let bar_fill = Rect::new(bar_bg.x, bar_bg.y, fill_w, bar_bg.height);
                draw_panel(canvas, bar_fill, bar_bg.height / 2, ACCENT_LINE, 255, ACCENT_LINE, 0);
            }
        }
        3 => {
            draw_text_color(canvas, "INSTALLATION COMPLETE", layout.card.x + layout.padding, title_y, &title_style, TEXT_PRIMARY);
            draw_text_color(canvas, "NETIZENCORE HAS BEEN INSTALLED TO PARTITIONS.", layout.card.x + layout.padding, layout.content_y, &body_style, TEXT_MUTED);
            draw_text_color(canvas, "CLICK RESTART TO BOOT FROM HARD DISK.", layout.card.x + layout.padding, layout.content_y + unit * 2, &body_style, TEXT_MUTED);
        }
        _ => {}
    }

    canvas.fill_rect(layout.card.x, layout.footer_y, layout.card.width, 1, PANEL_BORDER);
    draw_badge(canvas, layout.status_badge, "READY", SUCCESS_BG, SUCCESS);

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
    if let Ok(plat_str) = core::str::from_utf8(&plat_buf[..plen]) {
        draw_text_color(canvas, plat_str, layout.plat_x, layout.plat_y, &caption_style, TEXT_DIM);
    }

    if state.step > 0 && state.step < 3 {
        let back_hov = point_in_rect(mouse.x, mouse.y, layout.back_btn);
        draw_button(canvas, layout.back_btn, "BACK", back_hov, false);
    }

    let next_text = if state.step == 3 { "RESTART" } else { "NEXT" };
    let next_hov = point_in_rect(mouse.x, mouse.y, layout.next_btn);
    draw_button(canvas, layout.next_btn, next_text, next_hov, true);

    let footer_style = text_style(unit * 6 / 10, 40);
    let bottom_y = height - unit * 3;
    draw_text_color(canvas, "NETIZENCORE", margin, bottom_y, &footer_style, TEXT_DIM);
    let brand_w = text_width("NETIZENCORE", &footer_style);
    draw_text_color(canvas, VERSION, margin + brand_w + unit, bottom_y, &footer_style, TEXT_DIM);
}

#![no_std]

use nc_app::text::{centered, Line};
use nc_app::theme::{
    accent_tint, ACCENTS, ACCENT_COUNT, SIDEBAR, SIDEBAR_HOVER, WALLPAPERS, WALLPAPER_COUNT,
};
use nc_app::{App, AppInfo, Context, Icon, Input, Update};
use nc_font::Style;
use nc_gfx::{lerp_color, Canvas, Rect};
use nc_ui::{
    draw_dot, draw_panel, draw_text_color, text_style, PANEL_BORDER, RAIL_DOT, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY, WHITE,
};

const PAGES: [&str; 2] = ["PERSONALIZE", "DISPLAY"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hover {
    Nothing,
    Page(usize),
    Swatch(usize),
    Accent(usize),
}

pub struct SettingsApp {
    info: AppInfo,
    page: usize,
    hover: Hover,
}

impl SettingsApp {
    pub fn new() -> SettingsApp {
        SettingsApp {
            info: AppInfo::new("nc.settings", "SETTINGS", env!("CARGO_PKG_VERSION"), Icon::Sliders, 38, 24),
            page: 0,
            hover: Hover::Nothing,
        }
    }

    fn hit(&self, area: Rect, unit: i32, x: i32, y: i32) -> Hover {
        for index in 0..PAGES.len() {
            if inside(sidebar_item(area, index, unit), x, y) {
                return Hover::Page(index);
            }
        }
        if self.page == 0 {
            for index in 0..WALLPAPER_COUNT {
                if inside(swatch(area, index, unit), x, y) {
                    return Hover::Swatch(index);
                }
            }
            for index in 0..ACCENT_COUNT {
                if inside(accent_rect(area, index, unit), x, y) {
                    return Hover::Accent(index);
                }
            }
        }
        Hover::Nothing
    }
}

impl App for SettingsApp {
    fn info(&self) -> &AppInfo {
        &self.info
    }

    fn update(&mut self, ctx: &mut Context, input: &Input) -> Update {
        let hover = match input.pointer {
            Some(pointer) => self.hit(input.area, ctx.unit, pointer.x, pointer.y),
            None => Hover::Nothing,
        };
        let mut redraw = hover != self.hover;
        self.hover = hover;
        if let Some(pointer) = input.pointer {
            if pointer.clicked {
                match hover {
                    Hover::Page(index) => self.page = index,
                    Hover::Swatch(index) => ctx.settings.wallpaper = index,
                    Hover::Accent(index) => ctx.settings.accent = index,
                    Hover::Nothing => {}
                }
                redraw = true;
            }
        }
        Update { redraw, close: false, install_app: None }
    }

    fn draw(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context) {
        let unit = ctx.unit;
        let accent = ctx.settings.accent_color();

        draw_panel(canvas, sidebar(area, unit), unit / 2, SIDEBAR, 255, PANEL_BORDER, 255);

        let label_style = text_style(unit * 6 / 10, 30);
        for index in 0..PAGES.len() {
            let item = sidebar_item(area, index, unit);
            let selected = self.page == index;
            if selected {
                let tint = accent_tint(accent);
                draw_panel(canvas, item, unit / 3, tint, 255, tint, 0);
            } else if self.hover == Hover::Page(index) {
                draw_panel(canvas, item, unit / 3, SIDEBAR_HOVER, 255, SIDEBAR_HOVER, 0);
            }
            let ink = if selected { accent } else { TEXT_MUTED };
            draw_text_color(canvas, PAGES[index], item.x + unit, item.y + (item.height - label_style.cap_height) / 2, &label_style, ink);
        }

        let x = main_x(area, unit);
        let title_style = text_style(unit * 11 / 10, 20);
        draw_text_color(canvas, PAGES[self.page.min(PAGES.len() - 1)], x, main_y(area, unit), &title_style, TEXT_PRIMARY);

        let caption_style = text_style(unit * 55 / 100, 40);
        if self.page == 0 {
            self.draw_personalize(canvas, area, ctx, x, &caption_style);
        } else {
            draw_display(canvas, area, ctx, x, &caption_style, &label_style);
        }
    }
}

impl SettingsApp {
    fn draw_personalize(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context, x: i32, caption_style: &Style) {
        let unit = ctx.unit;
        let accent = ctx.settings.accent_color();
        let top = content_y(area, unit);

        draw_text_color(canvas, "WALLPAPER", x, top, caption_style, TEXT_MUTED);

        for index in 0..WALLPAPER_COUNT {
            let cell = swatch(area, index, unit);
            let wallpaper = &WALLPAPERS[index];
            for row in 0..cell.height {
                let t = (row * 255 / (cell.height - 1).max(1)) as u32;
                canvas.fill_rect(cell.x, cell.y + row, cell.width, 1, lerp_color(wallpaper.top, wallpaper.bottom, t));
            }
            draw_panel(canvas, cell, 2, WHITE, 0, PANEL_BORDER, 255);
            let selected = ctx.settings.wallpaper == index;
            if selected {
                draw_panel(canvas, Rect::new(cell.x - 3, cell.y - 3, cell.width + 6, cell.height + 6), 4, WHITE, 0, accent, 255);
                draw_panel(canvas, Rect::new(cell.x - 4, cell.y - 4, cell.width + 8, cell.height + 8), 5, WHITE, 0, accent, 255);
            } else if self.hover == Hover::Swatch(index) {
                draw_panel(canvas, Rect::new(cell.x - 3, cell.y - 3, cell.width + 6, cell.height + 6), 4, WHITE, 0, RAIL_DOT, 255);
            }
            let ink = if selected { TEXT_PRIMARY } else { TEXT_MUTED };
            centered(canvas, wallpaper.name, cell.x + cell.width / 2, cell.y + cell.height + unit * 3 / 4, caption_style, ink);
        }

        draw_text_color(canvas, "ACCENT COLOR", x, top + unit * 9, caption_style, TEXT_MUTED);

        for index in 0..ACCENT_COUNT {
            let cell = accent_rect(area, index, unit);
            let center_x = cell.x + cell.width / 2;
            let center_y = cell.y + cell.height / 2;
            let radius = unit * 4 / 5;
            if ctx.settings.accent == index {
                draw_dot(canvas, center_x, center_y, radius + 4, ACCENTS[index], 256);
                draw_dot(canvas, center_x, center_y, radius + 2, WHITE, 256);
            } else if self.hover == Hover::Accent(index) {
                draw_dot(canvas, center_x, center_y, radius + 2, RAIL_DOT, 256);
            }
            draw_dot(canvas, center_x, center_y, radius, ACCENTS[index], 256);
        }

        draw_text_color(canvas, "SETTINGS ARE SAVED FOR THIS SESSION.", x, area.y + area.height - unit * 2, caption_style, TEXT_DIM);
    }
}

fn draw_display(canvas: &mut Canvas<'_>, area: Rect, ctx: &Context, x: i32, caption_style: &Style, label_style: &Style) {
    let unit = ctx.unit;
    let top = content_y(area, unit);
    let row = Rect::new(x, top, area.x + area.width - unit * 2 - x, unit * 3);
    draw_panel(canvas, row, unit / 2, SIDEBAR, 255, PANEL_BORDER, 255);

    let text_y = row.y + (row.height - label_style.cap_height) / 2;
    draw_text_color(canvas, "RESOLUTION", row.x + unit * 3 / 2, text_y, label_style, TEXT_MUTED);

    let mut value = Line::new();
    value.number(ctx.screen_width as u32, 1).text(" X ").number(ctx.screen_height as u32, 1);
    draw_text_color(canvas, value.as_str(), row.x + unit * 11, text_y, label_style, TEXT_PRIMARY);

    draw_text_color(canvas, "DISPLAY RESOLUTION IS CONFIGURED BY THE BOOTLOADER.", x, top + unit * 4, caption_style, TEXT_DIM);
}

fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

fn sidebar(area: Rect, unit: i32) -> Rect {
    Rect::new(area.x + unit / 2, area.y + unit / 2, unit * 8, area.height - unit)
}

fn sidebar_item(area: Rect, index: usize, unit: i32) -> Rect {
    let side = sidebar(area, unit);
    Rect::new(side.x + unit / 2, side.y + unit / 2 + index as i32 * (unit * 3 + unit / 4), side.width - unit, unit * 3)
}

fn main_x(area: Rect, unit: i32) -> i32 {
    let side = sidebar(area, unit);
    side.x + side.width + unit * 2
}

fn main_y(area: Rect, unit: i32) -> i32 {
    area.y + unit * 3 / 2
}

fn content_y(area: Rect, unit: i32) -> i32 {
    main_y(area, unit) + unit * 3
}

fn swatch(area: Rect, index: usize, unit: i32) -> Rect {
    Rect::new(main_x(area, unit) + index as i32 * (unit * 5 + unit * 3 / 4), content_y(area, unit) + unit * 2, unit * 5, unit * 4)
}

fn accent_rect(area: Rect, index: usize, unit: i32) -> Rect {
    Rect::new(main_x(area, unit) + index as i32 * unit * 3, content_y(area, unit) + unit * 21 / 2, unit * 2, unit * 2)
}

#![no_std]

use nc_assets::{logo, Logo};
use nc_font::{text_width, Style};
use nc_gfx::blur::box_blur;
use nc_gfx::compose::compose_ink;
use nc_gfx::{invert, lerp_color, rgb, Canvas, Rect};
use nc_math::{ratio256, sin_milli, smoothstep256};
use nc_mem::Arena;
use nc_ui::{draw_dot, draw_text_color, text_style, ACCENT_DARK, ACCENT_LIGHT, ACCENT_LINE, AQUA, TEXT_MUTED, TEXT_PRIMARY};

const TITLE: &str = "NetizenCore";
const INTRO_MS: u32 = 1600;
const OUTRO_MS: u32 = 800;
const FADE_IN_MS: u32 = 1000;
const LINE_GROW_START_MS: u32 = 400;
const LINE_GROW_MS: u32 = 1000;
const BREATH_PERIOD_MS: u32 = 3200;
const SHIMMER_PERIOD_MS: u32 = 2000;
const DOT_PERIOD_MS: u32 = 1400;
const LEVEL_COUNT: usize = 4;
const BLUR_RADII: [usize; LEVEL_COUNT] = [0, 4, 10, 22];
const STAGE_MARGIN: i32 = 56;
const SILHOUETTE_FLOOR: u8 = 200;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Intro,
    Idle,
    Outro,
    Done,
}

pub struct SplashOptions<'a> {
    pub subtitle: Option<&'a str>,
}

pub struct Update {
    pub rects: [Rect; 2],
    pub count: usize,
}

impl Update {
    fn none() -> Update {
        Update { rects: [Rect::EMPTY; 2], count: 0 }
    }

    fn one(first: Rect) -> Update {
        Update { rects: [first, Rect::EMPTY], count: 1 }
    }

    fn two(first: Rect, second: Rect) -> Update {
        Update { rects: [first, second], count: 2 }
    }
}

struct Layout {
    stage: Rect,
    center_x: i32,
    logo_x: i32,
    logo_y: i32,
    title_style: Style,
    title_x: i32,
    title_y: i32,
    title_width: i32,
    line_y: i32,
    has_subtitle: bool,
    subtitle_style: Style,
    subtitle_x: i32,
    subtitle_y: i32,
    dots_x: i32,
    dots_y: i32,
    dot_gap: i32,
    dot_radius: i32,
}

fn compute_layout(width: i32, height: i32, logo_width: i32, logo_height: i32, subtitle: Option<&str>) -> Layout {
    let cap = (height / 15).max(20);
    let title_style = text_style(cap, 36);
    let title_width = text_width(TITLE, &title_style);
    let subtitle_cap = (cap * 36 / 100).max(12);
    let subtitle_style = text_style(subtitle_cap, 70);
    let has_subtitle = subtitle.is_some();
    let subtitle_width = match subtitle {
        Some(text) => text_width(text, &subtitle_style),
        None => 0,
    };
    let dot_radius = (subtitle_cap / 3).max(3);
    let dot_gap = subtitle_cap * 12 / 10;
    let spacer = subtitle_cap * 12 / 10;
    let subtitle_total = if has_subtitle { subtitle_width + spacer + dot_gap * 2 } else { 0 };

    let gap_logo = if logo_height > 0 { cap * 9 / 10 } else { 0 };
    let title_offset = logo_height + gap_logo;
    let line_offset = title_offset + cap + cap * 7 / 10;
    let subtitle_offset = line_offset + cap * 7 / 10;
    let block_height = if has_subtitle { subtitle_offset + subtitle_cap } else { line_offset + 2 };
    let top = ((height - block_height) / 2 - height / 24).max(8);
    let center_x = width / 2;
    let subtitle_x = center_x - subtitle_total / 2;

    let widest = logo_width.max(title_width).max(subtitle_total);
    let stage_left = (center_x - widest / 2 - STAGE_MARGIN).max(0);
    let stage_right = (center_x + widest / 2 + STAGE_MARGIN).min(width);
    let stage_top = (top - STAGE_MARGIN).max(0);
    let stage_bottom = (top + block_height + STAGE_MARGIN).min(height);

    Layout {
        stage: Rect::new(stage_left, stage_top, stage_right - stage_left, stage_bottom - stage_top),
        center_x,
        logo_x: center_x - logo_width / 2,
        logo_y: top,
        title_style,
        title_x: center_x - title_width / 2,
        title_y: top + title_offset,
        title_width,
        line_y: top + line_offset,
        has_subtitle,
        subtitle_style,
        subtitle_x,
        subtitle_y: top + subtitle_offset,
        dots_x: subtitle_x + subtitle_width + spacer,
        dots_y: top + subtitle_offset + subtitle_cap * 9 / 10,
        dot_gap,
        dot_radius,
    }
}

fn new_layer(arena: &mut Arena, width: usize, height: usize) -> Option<Canvas<'static>> {
    let pixels = arena.alloc_pixels(width * height)?;
    Canvas::new(pixels, width, height)
}

fn is_silhouette(logo: &Logo) -> bool {
    logo.pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[3] > 0)
        .all(|pixel| pixel[0] >= SILHOUETTE_FLOOR && pixel[1] >= SILHOUETTE_FLOOR && pixel[2] >= SILHOUETTE_FLOOR)
}

fn draw_logo_ink(layer: &mut Canvas<'_>, x: i32, y: i32, logo: &Logo) {
    let width = logo.width as usize;
    let height = logo.height as usize;
    let tinted = is_silhouette(logo);
    for row in 0..height {
        let tint = lerp_color(ACCENT_LIGHT, ACCENT_DARK, (row * 255 / height.max(1)) as u32);
        for column in 0..width {
            let index = (row * width + column) * 4;
            let alpha = logo.pixels[index + 3];
            if alpha == 0 {
                continue;
            }
            let color = if tinted {
                tint
            } else {
                rgb(logo.pixels[index], logo.pixels[index + 1], logo.pixels[index + 2])
            };
            layer.blend_pixel(x + column as i32, y + row as i32, invert(color), alpha);
        }
    }
}

fn breathing(elapsed: u32) -> u32 {
    let cycle = (elapsed % BREATH_PERIOD_MS) as i32 * 360 / BREATH_PERIOD_MS as i32;
    let unit = (1000 + sin_milli(cycle - 90)) as u32 / 2;
    90 + 166 * unit / 1000
}

pub struct Splash {
    layout: Layout,
    base: Canvas<'static>,
    levels: [Canvas<'static>; LEVEL_COUNT],
    started_at: Option<u32>,
    finish_requested: bool,
    finish_at: u32,
    phase: Phase,
}

impl Splash {
    pub fn new(canvas: &Canvas<'_>, arena: &mut Arena, options: SplashOptions<'_>) -> Option<Splash> {
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        let logo = logo();
        let (logo_width, logo_height) = match &logo {
            Some(logo) => (logo.width as i32, logo.height as i32),
            None => (0, 0),
        };
        let layout = compute_layout(width, height, logo_width, logo_height, options.subtitle);
        let stage = layout.stage;
        let stage_width = stage.width as usize;
        let stage_height = stage.height as usize;

        let mut base = new_layer(arena, stage_width, stage_height)?;
        base.copy_rect_from(canvas, stage.x, stage.y, 0, 0, stage.width, stage.height);

        let mut sharp = new_layer(arena, stage_width, stage_height)?;
        sharp.fill(0);
        if let Some(logo) = &logo {
            draw_logo_ink(&mut sharp, layout.logo_x - stage.x, layout.logo_y - stage.y, logo);
        }
        draw_text_color(
            &mut sharp,
            TITLE,
            layout.title_x - stage.x,
            layout.title_y - stage.y,
            &layout.title_style,
            invert(TEXT_PRIMARY),
        );
        if let Some(text) = options.subtitle {
            draw_text_color(
                &mut sharp,
                text,
                layout.subtitle_x - stage.x,
                layout.subtitle_y - stage.y,
                &layout.subtitle_style,
                invert(TEXT_MUTED),
            );
        }

        let mut level1 = new_layer(arena, stage_width, stage_height)?;
        let mut level2 = new_layer(arena, stage_width, stage_height)?;
        let mut level3 = new_layer(arena, stage_width, stage_height)?;
        let mut temp = new_layer(arena, stage_width, stage_height)?;
        box_blur(&sharp, &mut level1, &mut temp, BLUR_RADII[1]);
        box_blur(&sharp, &mut level2, &mut temp, BLUR_RADII[2]);
        box_blur(&sharp, &mut level3, &mut temp, BLUR_RADII[3]);

        Some(Splash {
            layout,
            base,
            levels: [sharp, level1, level2, level3],
            started_at: None,
            finish_requested: false,
            finish_at: 0,
            phase: Phase::Intro,
        })
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn request_finish(&mut self) {
        self.finish_requested = true;
    }

    pub fn frame(&mut self, canvas: &mut Canvas<'_>, now: u32) -> Update {
        let start = match self.started_at {
            Some(value) => value,
            None => {
                self.started_at = Some(now);
                now
            }
        };
        let elapsed = now.wrapping_sub(start);
        if self.phase == Phase::Idle && self.finish_requested {
            self.phase = Phase::Outro;
            self.finish_at = elapsed;
        }
        match self.phase {
            Phase::Intro => self.frame_intro(canvas, elapsed),
            Phase::Idle => self.frame_idle(canvas, elapsed),
            Phase::Outro => self.frame_outro(canvas, elapsed),
            Phase::Done => Update::none(),
        }
    }

    fn full_area(&self) -> Rect {
        Rect::new(0, 0, self.layout.stage.width, self.layout.stage.height)
    }

    fn compose(&self, canvas: &mut Canvas<'_>, area: Rect, blur_position: u32, opacity: u32) {
        let lower = ((blur_position >> 8) as usize).min(LEVEL_COUNT - 1);
        let upper = (lower + 1).min(LEVEL_COUNT - 1);
        let mix = if lower == upper { 0 } else { blur_position & 255 };
        compose_ink(
            canvas,
            self.layout.stage.x,
            self.layout.stage.y,
            &self.base,
            &self.levels[lower],
            &self.levels[upper],
            mix,
            opacity,
            area,
        );
    }

    fn restore(&self, canvas: &mut Canvas<'_>, screen_area: Rect) {
        let local = Rect::new(
            screen_area.x - self.layout.stage.x,
            screen_area.y - self.layout.stage.y,
            screen_area.width,
            screen_area.height,
        );
        self.compose(canvas, local, 0, 256);
    }

    fn line_rect(&self) -> Rect {
        let half = self.layout.title_width / 2;
        Rect::new(self.layout.center_x - half - 8, self.layout.line_y - 3, half * 2 + 16, 7)
    }

    fn dots_rect(&self) -> Rect {
        let layout = &self.layout;
        let radius = layout.dot_radius * 2;
        Rect::new(
            layout.dots_x - radius - 2,
            layout.dots_y - radius - 2,
            layout.dot_gap * 2 + radius * 2 + 4,
            radius * 2 + 4,
        )
    }

    fn draw_line(&self, canvas: &mut Canvas<'_>, elapsed: u32, scale: u32, opacity: u32) {
        let layout = &self.layout;
        let half_full = layout.title_width / 2;
        let half = (half_full as i64 * scale as i64 / 256) as i32;
        if half < 2 || opacity == 0 {
            return;
        }
        let sweep = (half_full / 5).max(12);
        let travel = half_full * 2 + sweep * 2;
        let phase = (elapsed % SHIMMER_PERIOD_MS) as i64;
        let position = -half_full - sweep + (travel as i64 * phase / SHIMMER_PERIOD_MS as i64) as i32;
        for offset in -half..=half {
            let fade = 255 - offset.abs() * 255 / half;
            let soft = (fade * fade / 255) as u32;
            let alpha = (soft * opacity / 256).min(255) as u8;
            let boost = (255 - (offset - position).abs() * 255 / sweep).clamp(0, 255) as u32;
            let color = lerp_color(ACCENT_LINE, AQUA, boost);
            let x = layout.center_x + offset;
            canvas.blend_pixel(x, layout.line_y, color, alpha);
            canvas.blend_pixel(x, layout.line_y - 1, color, alpha / 3);
            canvas.blend_pixel(x, layout.line_y + 1, color, alpha / 3);
        }
    }

    fn draw_dots(&self, canvas: &mut Canvas<'_>, elapsed: u32, opacity: u32) {
        let layout = &self.layout;
        let cycle = (elapsed % DOT_PERIOD_MS) as i32 * 360 / DOT_PERIOD_MS as i32;
        for index in 0..3i32 {
            let wave = (1000 + sin_milli(cycle - index * 110)) as u32 / 2;
            let strength = ((70 + 186 * wave / 1000) * opacity / 256).min(256);
            draw_dot(
                canvas,
                layout.dots_x + index * layout.dot_gap,
                layout.dots_y,
                layout.dot_radius,
                ACCENT_LINE,
                strength,
            );
        }
    }

    fn draw_dynamic(&self, canvas: &mut Canvas<'_>, elapsed: u32, line_scale: u32, opacity: u32) {
        self.draw_line(canvas, elapsed, line_scale, opacity);
        if self.layout.has_subtitle {
            self.draw_dots(canvas, elapsed, opacity);
        }
    }

    fn frame_intro(&mut self, canvas: &mut Canvas<'_>, elapsed: u32) -> Update {
        if elapsed >= INTRO_MS {
            self.phase = Phase::Idle;
            let area = self.full_area();
            self.compose(canvas, area, 0, 256);
            self.draw_dynamic(canvas, elapsed, breathing(elapsed), 256);
            return Update::one(self.layout.stage);
        }
        let progress = smoothstep256(ratio256(elapsed, INTRO_MS));
        let blur_position = (256 - progress) * 3;
        let opacity = smoothstep256(ratio256(elapsed, FADE_IN_MS));
        let grow = smoothstep256(ratio256(elapsed.saturating_sub(LINE_GROW_START_MS), LINE_GROW_MS));
        let line_scale = breathing(elapsed) * grow / 256;
        let area = self.full_area();
        self.compose(canvas, area, blur_position, opacity);
        self.draw_dynamic(canvas, elapsed, line_scale, opacity);
        Update::one(self.layout.stage)
    }

    fn frame_idle(&mut self, canvas: &mut Canvas<'_>, elapsed: u32) -> Update {
        let line_rect = self.line_rect();
        self.restore(canvas, line_rect);
        self.draw_line(canvas, elapsed, breathing(elapsed), 256);
        if self.layout.has_subtitle {
            let dots_rect = self.dots_rect();
            self.restore(canvas, dots_rect);
            self.draw_dots(canvas, elapsed, 256);
            Update::two(line_rect, dots_rect)
        } else {
            Update::one(line_rect)
        }
    }

    fn frame_outro(&mut self, canvas: &mut Canvas<'_>, elapsed: u32) -> Update {
        let since = elapsed.wrapping_sub(self.finish_at);
        let area = self.full_area();
        if since >= OUTRO_MS {
            self.phase = Phase::Done;
            self.compose(canvas, area, 512, 0);
            return Update::one(self.layout.stage);
        }
        let progress = smoothstep256(ratio256(since, OUTRO_MS));
        let blur_position = progress * 2;
        let opacity = 256 - progress;
        let line_scale = breathing(elapsed) * opacity / 256;
        self.compose(canvas, area, blur_position, opacity);
        self.draw_dynamic(canvas, elapsed, line_scale, opacity);
        Update::one(self.layout.stage)
    }
}

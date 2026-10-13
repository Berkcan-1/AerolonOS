#![no_std]

use nc_math::{cos_milli, integer_sqrt, sin_milli};

const MAX_SEGMENTS: usize = 64;
const SPACE_WIDTH: i32 = 400;

#[derive(Clone, Copy)]
pub struct Style {
    pub cap_height: i32,
    pub stroke_sixteenths: i32,
    pub tracking: i32,
}

#[derive(Clone, Copy)]
struct Segment {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

const EMPTY: Segment = Segment { x0: 0, y0: 0, x1: 0, y1: 0 };

struct Path {
    segments: [Segment; MAX_SEGMENTS],
    count: usize,
    width: i32,
}

fn arc_point(center_x: i32, center_y: i32, radius_x: i32, radius_y: i32, degrees: i32) -> (i32, i32) {
    (
        center_x + radius_x * cos_milli(degrees) / 1000,
        center_y - radius_y * sin_milli(degrees) / 1000,
    )
}

impl Path {
    fn new(width: i32) -> Path {
        Path { segments: [EMPTY; MAX_SEGMENTS], count: 0, width }
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        if self.count < MAX_SEGMENTS {
            self.segments[self.count] = Segment { x0, y0, x1, y1 };
            self.count += 1;
        }
    }

    fn arc(&mut self, center_x: i32, center_y: i32, radius_x: i32, radius_y: i32, start: i32, end: i32, step: i32) {
        let steps = (end - start) / step;
        let mut previous = arc_point(center_x, center_y, radius_x, radius_y, start);
        for index in 1..=steps {
            let current = arc_point(center_x, center_y, radius_x, radius_y, start + index * step);
            self.line(previous.0, previous.1, current.0, current.1);
            previous = current;
        }
    }
}

fn glyph(character: char) -> Path {
    match character.to_ascii_uppercase() {
        'A' => {
            let mut path = Path::new(700);
            path.line(0, 1000, 350, 0);
            path.line(350, 0, 700, 1000);
            path.line(126, 640, 574, 640);
            path
        }
        'B' => {
            let mut path = Path::new(580);
            path.line(0, 0, 0, 1000);
            path.line(0, 0, 300, 0);
            path.arc(300, 250, 250, 250, 90, -90, -10);
            path.line(300, 500, 0, 500);
            path.line(0, 500, 330, 500);
            path.arc(330, 750, 250, 250, 90, -90, -10);
            path.line(330, 1000, 0, 1000);
            path
        }
        'C' => {
            let mut path = Path::new(800);
            path.arc(450, 500, 450, 500, 40, 320, 10);
            path
        }
        'D' => {
            let mut path = Path::new(700);
            path.line(0, 0, 0, 1000);
            path.line(0, 0, 330, 0);
            path.arc(330, 500, 370, 500, 90, -90, -10);
            path.line(330, 1000, 0, 1000);
            path
        }
        'E' => {
            let mut path = Path::new(540);
            path.line(0, 0, 0, 1000);
            path.line(0, 0, 540, 0);
            path.line(0, 500, 460, 500);
            path.line(0, 1000, 540, 1000);
            path
        }
        'F' => {
            let mut path = Path::new(520);
            path.line(0, 0, 0, 1000);
            path.line(0, 0, 520, 0);
            path.line(0, 480, 440, 480);
            path
        }
        'G' => {
            let mut path = Path::new(900);
            path.arc(450, 500, 450, 500, 40, 360, 10);
            path.line(900, 500, 500, 500);
            path
        }
        'H' => {
            let mut path = Path::new(700);
            path.line(0, 0, 0, 1000);
            path.line(700, 0, 700, 1000);
            path.line(0, 500, 700, 500);
            path
        }
        'I' => {
            let mut path = Path::new(0);
            path.line(0, 0, 0, 1000);
            path
        }
        'J' => {
            let mut path = Path::new(420);
            path.line(420, 0, 420, 650);
            path.arc(210, 650, 210, 350, 0, -180, -10);
            path.line(0, 650, 0, 560);
            path
        }
        'K' => {
            let mut path = Path::new(640);
            path.line(0, 0, 0, 1000);
            path.line(0, 560, 620, 0);
            path.line(200, 380, 640, 1000);
            path
        }
        'L' => {
            let mut path = Path::new(520);
            path.line(0, 0, 0, 1000);
            path.line(0, 1000, 520, 1000);
            path
        }
        'M' => {
            let mut path = Path::new(860);
            path.line(0, 1000, 0, 0);
            path.line(0, 0, 430, 620);
            path.line(430, 620, 860, 0);
            path.line(860, 0, 860, 1000);
            path
        }
        'N' => {
            let mut path = Path::new(720);
            path.line(0, 1000, 0, 0);
            path.line(0, 0, 720, 1000);
            path.line(720, 1000, 720, 0);
            path
        }
        'O' => {
            let mut path = Path::new(900);
            path.arc(450, 500, 450, 500, 90, 450, 10);
            path
        }
        'P' => {
            let mut path = Path::new(595);
            path.line(0, 0, 0, 1000);
            path.line(0, 0, 330, 0);
            path.arc(330, 265, 265, 265, 90, -90, -10);
            path.line(330, 530, 0, 530);
            path
        }
        'Q' => {
            let mut path = Path::new(900);
            path.arc(450, 500, 450, 500, 90, 450, 10);
            path.line(560, 720, 880, 1000);
            path
        }
        'R' => {
            let mut path = Path::new(620);
            path.line(0, 0, 0, 1000);
            path.line(0, 0, 320, 0);
            path.arc(320, 265, 265, 265, 90, -90, -10);
            path.line(320, 530, 0, 530);
            path.line(300, 530, 620, 1000);
            path
        }
        'S' => {
            let mut path = Path::new(600);
            path.arc(300, 250, 280, 250, 30, 270, 10);
            path.arc(300, 750, 280, 250, 90, -150, -10);
            path
        }
        'T' => {
            let mut path = Path::new(620);
            path.line(0, 0, 620, 0);
            path.line(310, 0, 310, 1000);
            path
        }
        'U' => {
            let mut path = Path::new(700);
            path.line(0, 0, 0, 650);
            path.arc(350, 650, 350, 350, 180, 360, 10);
            path.line(700, 650, 700, 0);
            path
        }
        'V' => {
            let mut path = Path::new(720);
            path.line(0, 0, 360, 1000);
            path.line(360, 1000, 720, 0);
            path
        }
        'W' => {
            let mut path = Path::new(1060);
            path.line(0, 0, 265, 1000);
            path.line(265, 1000, 530, 0);
            path.line(530, 0, 795, 1000);
            path.line(795, 1000, 1060, 0);
            path
        }
        'X' => {
            let mut path = Path::new(700);
            path.line(0, 0, 700, 1000);
            path.line(700, 0, 0, 1000);
            path
        }
        'Y' => {
            let mut path = Path::new(700);
            path.line(0, 0, 350, 520);
            path.line(700, 0, 350, 520);
            path.line(350, 520, 350, 1000);
            path
        }
        'Z' => {
            let mut path = Path::new(600);
            path.line(0, 0, 600, 0);
            path.line(600, 0, 0, 1000);
            path.line(0, 1000, 600, 1000);
            path
        }
        '0' => {
            let mut path = Path::new(620);
            path.arc(310, 500, 310, 500, 90, 450, 10);
            path
        }
        '1' => {
            let mut path = Path::new(200);
            path.line(0, 200, 200, 0);
            path.line(200, 0, 200, 1000);
            path
        }
        '2' => {
            let mut path = Path::new(600);
            path.arc(300, 300, 290, 300, 160, -30, -10);
            path.line(522, 450, 0, 1000);
            path.line(0, 1000, 600, 1000);
            path
        }
        '3' => {
            let mut path = Path::new(570);
            path.arc(280, 250, 260, 250, 150, -90, -10);
            path.arc(280, 750, 290, 250, 90, -150, -10);
            path
        }
        '4' => {
            let mut path = Path::new(620);
            path.line(430, 1000, 430, 0);
            path.line(430, 0, 0, 680);
            path.line(0, 680, 620, 680);
            path
        }
        '5' => {
            let mut path = Path::new(580);
            path.line(540, 0, 60, 0);
            path.line(60, 0, 20, 460);
            path.line(20, 460, 104, 506);
            path.arc(290, 720, 290, 280, 130, -140, -10);
            path
        }
        '6' => {
            let mut path = Path::new(600);
            path.arc(300, 700, 300, 300, 90, 450, 10);
            path.arc(620, 700, 620, 700, 180, 105, -5);
            path
        }
        '7' => {
            let mut path = Path::new(600);
            path.line(0, 0, 600, 0);
            path.line(600, 0, 200, 1000);
            path
        }
        '8' => {
            let mut path = Path::new(600);
            path.arc(300, 255, 245, 255, 90, 450, 15);
            path.arc(300, 745, 300, 255, 90, 450, 15);
            path
        }
        '9' => {
            let mut path = Path::new(600);
            path.arc(300, 300, 300, 300, 90, 450, 10);
            path.arc(-20, 300, 620, 700, 0, -75, -5);
            path
        }
        '.' => {
            let mut path = Path::new(0);
            path.line(0, 1000, 0, 1000);
            path
        }
        ',' => {
            let mut path = Path::new(0);
            path.line(0, 1000, 0, 1000);
            path.line(0, 1000, -50, 1130);
            path
        }
        ':' => {
            let mut path = Path::new(0);
            path.line(0, 300, 0, 300);
            path.line(0, 1000, 0, 1000);
            path
        }
        '-' => {
            let mut path = Path::new(400);
            path.line(0, 500, 400, 500);
            path
        }
        '/' => {
            let mut path = Path::new(400);
            path.line(0, 1000, 400, 0);
            path
        }
        '!' => {
            let mut path = Path::new(0);
            path.line(0, 0, 0, 680);
            path.line(0, 1000, 0, 1000);
            path
        }
        _ => Path::new(SPACE_WIDTH),
    }
}

fn scale(unit: i32, cap_height: i32) -> i32 {
    unit * cap_height * 256 / 1000
}

fn glyph_width_pixels(path: &Path, cap_height: i32) -> i32 {
    path.width * cap_height / 1000
}

fn distance_squared(px: i64, py: i64, segment: &Segment) -> i64 {
    let ax = segment.x0 as i64;
    let ay = segment.y0 as i64;
    let dx = segment.x1 as i64 - ax;
    let dy = segment.y1 as i64 - ay;
    let length_squared = dx * dx + dy * dy;
    let (qx, qy) = if length_squared == 0 {
        (ax, ay)
    } else {
        let t = (px - ax) * dx + (py - ay) * dy;
        if t <= 0 {
            (ax, ay)
        } else if t >= length_squared {
            (segment.x1 as i64, segment.y1 as i64)
        } else {
            (ax + dx * t / length_squared, ay + dy * t / length_squared)
        }
    };
    let ex = px - qx;
    let ey = py - qy;
    ex * ex + ey * ey
}

fn draw_path(path: &Path, origin_x: i32, origin_y: i32, style: &Style, plot: &mut dyn FnMut(i32, i32, u8)) {
    if path.count == 0 {
        return;
    }
    let cap = style.cap_height;
    let half = style.stroke_sixteenths * 8;
    let mut scaled = [EMPTY; MAX_SEGMENTS];
    let mut left = i32::MAX;
    let mut right = i32::MIN;
    let mut top = i32::MAX;
    let mut bottom = i32::MIN;
    for index in 0..path.count {
        let segment = path.segments[index];
        let moved = Segment {
            x0: origin_x * 256 + scale(segment.x0, cap),
            y0: origin_y * 256 + scale(segment.y0, cap),
            x1: origin_x * 256 + scale(segment.x1, cap),
            y1: origin_y * 256 + scale(segment.y1, cap),
        };
        left = left.min(moved.x0).min(moved.x1);
        right = right.max(moved.x0).max(moved.x1);
        top = top.min(moved.y0).min(moved.y1);
        bottom = bottom.max(moved.y0).max(moved.y1);
        scaled[index] = moved;
    }
    let min_x = (left - half).div_euclid(256) - 2;
    let max_x = (right + half).div_euclid(256) + 2;
    let min_y = (top - half).div_euclid(256) - 2;
    let max_y = (bottom + half).div_euclid(256) + 2;
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let sample_x = px as i64 * 256 + 128;
            let sample_y = py as i64 * 256 + 128;
            let mut best = i64::MAX;
            for index in 0..path.count {
                let distance = distance_squared(sample_x, sample_y, &scaled[index]);
                if distance < best {
                    best = distance;
                }
            }
            let distance = integer_sqrt(best as u64) as i32;
            let coverage = half + 128 - distance;
            if coverage > 0 {
                plot(px, py, coverage.min(255) as u8);
            }
        }
    }
}

pub fn text_width(text: &str, style: &Style) -> i32 {
    let mut total = 0;
    let mut first = true;
    for character in text.chars() {
        if !first {
            total += style.tracking;
        }
        first = false;
        total += glyph_width_pixels(&glyph(character), style.cap_height);
    }
    total
}

pub fn draw_text(text: &str, x: i32, y: i32, style: &Style, plot: &mut dyn FnMut(i32, i32, u8)) {
    let mut cursor = x;
    for character in text.chars() {
        let path = glyph(character);
        draw_path(&path, cursor, y, style, plot);
        cursor += glyph_width_pixels(&path, style.cap_height) + style.tracking;
    }
}

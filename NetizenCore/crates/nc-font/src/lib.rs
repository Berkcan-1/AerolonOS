#![no_std]

const SIN_TABLE: [i32; 19] = [
    0, 87, 174, 259, 342, 423, 500, 574, 643, 707, 766, 819, 866, 906, 940, 966, 985, 996, 1000,
];
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

fn sin_milli(degrees: i32) -> i32 {
    let d = degrees.rem_euclid(360);
    let (index, sign) = if d <= 90 {
        (d / 5, 1)
    } else if d <= 180 {
        ((180 - d) / 5, 1)
    } else if d <= 270 {
        ((d - 180) / 5, -1)
    } else {
        ((360 - d) / 5, -1)
    };
    sign * SIN_TABLE[index as usize]
}

fn cos_milli(degrees: i32) -> i32 {
    sin_milli(degrees + 90)
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
        'N' => {
            let mut path = Path::new(720);
            path.line(0, 1000, 0, 0);
            path.line(0, 0, 720, 1000);
            path.line(720, 1000, 720, 0);
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
        'T' => {
            let mut path = Path::new(620);
            path.line(0, 0, 620, 0);
            path.line(310, 0, 310, 1000);
            path
        }
        'I' => {
            let mut path = Path::new(0);
            path.line(0, 0, 0, 1000);
            path
        }
        'Z' => {
            let mut path = Path::new(600);
            path.line(0, 0, 600, 0);
            path.line(600, 0, 0, 1000);
            path.line(0, 1000, 600, 1000);
            path
        }
        'C' => {
            let mut path = Path::new(800);
            path.arc(450, 500, 450, 500, 40, 320, 10);
            path
        }
        'O' => {
            let mut path = Path::new(900);
            path.arc(450, 500, 450, 500, 90, 450, 10);
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
        _ => Path::new(SPACE_WIDTH),
    }
}

fn scale(unit: i32, cap_height: i32) -> i32 {
    unit * cap_height * 256 / 1000
}

fn glyph_width_pixels(path: &Path, cap_height: i32) -> i32 {
    path.width * cap_height / 1000
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut x = 1u64 << ((64 - value.leading_zeros() + 1) / 2);
    loop {
        let y = (x + value / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
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
    for index in 0..path.count {
        let segment = path.segments[index];
        scaled[index] = Segment {
            x0: origin_x * 256 + scale(segment.x0, cap),
            y0: origin_y * 256 + scale(segment.y0, cap),
            x1: origin_x * 256 + scale(segment.x1, cap),
            y1: origin_y * 256 + scale(segment.y1, cap),
        };
    }
    let margin = (half + 255) / 256 + 2;
    let min_x = origin_x - margin;
    let max_x = origin_x + glyph_width_pixels(path, cap) + margin;
    let min_y = origin_y - margin;
    let max_y = origin_y + cap + margin;
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

use nc_gfx::{lerp_color, rgb};
use nc_ui::WHITE;

pub struct Glow {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    pub color: u32,
    pub strength: u32,
}

pub struct Wallpaper {
    pub name: &'static str,
    pub top: u32,
    pub bottom: u32,
    pub glows: [Glow; 2],
}

pub const WALLPAPER_COUNT: usize = 4;

pub const WALLPAPERS: [Wallpaper; WALLPAPER_COUNT] = [
    Wallpaper {
        name: "AURORA",
        top: rgb(30, 112, 214),
        bottom: rgb(14, 150, 160),
        glows: [
            Glow { x: 75, y: 0, radius: 90, color: rgb(186, 230, 253), strength: 120 },
            Glow { x: 12, y: 100, radius: 80, color: rgb(110, 231, 214), strength: 110 },
        ],
    },
    Wallpaper {
        name: "DUSK",
        top: rgb(49, 46, 129),
        bottom: rgb(176, 64, 120),
        glows: [
            Glow { x: 10, y: 0, radius: 70, color: rgb(167, 139, 250), strength: 90 },
            Glow { x: 82, y: 100, radius: 85, color: rgb(251, 191, 126), strength: 120 },
        ],
    },
    Wallpaper {
        name: "FOREST",
        top: rgb(8, 74, 64),
        bottom: rgb(22, 128, 100),
        glows: [
            Glow { x: 20, y: 5, radius: 80, color: rgb(167, 243, 208), strength: 90 },
            Glow { x: 90, y: 95, radius: 60, color: rgb(253, 230, 138), strength: 60 },
        ],
    },
    Wallpaper {
        name: "GRAPHITE",
        top: rgb(36, 48, 68),
        bottom: rgb(15, 23, 42),
        glows: [
            Glow { x: 70, y: 0, radius: 90, color: rgb(100, 116, 139), strength: 80 },
            Glow { x: 10, y: 100, radius: 70, color: rgb(51, 65, 85), strength: 80 },
        ],
    },
];

pub const ACCENT_COUNT: usize = 4;
pub const ACCENTS: [u32; ACCENT_COUNT] = [rgb(37, 99, 235), rgb(13, 148, 136), rgb(124, 58, 237), rgb(234, 88, 12)];

pub const SIDEBAR: u32 = rgb(244, 247, 250);
pub const SIDEBAR_HOVER: u32 = rgb(232, 238, 245);
const BLACK: u32 = rgb(0, 0, 0);

pub fn accent_tint(accent: u32) -> u32 {
    lerp_color(WHITE, accent, 30)
}

pub fn accent_hover(accent: u32) -> u32 {
    lerp_color(accent, BLACK, 40)
}

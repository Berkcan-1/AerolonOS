pub mod storage;
pub mod system;
pub mod volumes;

use crate::dock::{TAB_STORAGE, TAB_SYSTEM, TAB_VOLUMES};
use nc_gfx::Canvas;

pub fn draw_active_view(canvas: &mut Canvas<'_>, width: i32, height: i32, platform: &str, active_tab: usize) {
    match active_tab {
        TAB_STORAGE => storage::draw(canvas, width, height),
        TAB_SYSTEM => system::draw(canvas, width, height, platform),
        TAB_VOLUMES => volumes::draw(canvas, width, height),
        _ => storage::draw(canvas, width, height),
    }
}

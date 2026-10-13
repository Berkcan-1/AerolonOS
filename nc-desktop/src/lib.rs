#![no_std]
extern crate alloc;

mod layout;
mod shell;
mod theme;
mod window;

use alloc::boxed::Box;
use alloc::vec::Vec;
use nc_app::theme::{WALLPAPERS, WALLPAPER_COUNT};
use nc_app::{App, Context, Input, Pointer, Settings};
use nc_gfx::{Canvas, Rect};
use nc_hal::{KeyCode, MouseState};
use layout::{
    close_button, content_area, context_item_rect, context_menu_rect, desktop_icon, inside, max_button, min_button, start_button,
    start_item, start_menu, task_button, taskbar_height, taskbar_rect, title_bar, title_height, unit_for, visible_start_apps,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DesktopAction {
    None,
    Shutdown,
    Reboot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Nothing,
    Desktop,
    Icon(usize),
    Taskbar,
    StartButton,
    TaskButton(usize),
    StartBody,
    StartItem(usize),
    WinBody(usize),
    WinTitle(usize),
    WinClose(usize),
    WinMax(usize),
    WinMin(usize),
    ContextMenu,
    ContextItem(usize),
}

#[derive(Clone, Copy)]
pub(crate) struct WindowState {
    pub open: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub placed: bool,
    pub x: i32,
    pub y: i32,
}

pub(crate) struct Slot {
    pub app: Box<dyn App>,
    pub window: WindowState,
}

#[derive(Clone, Copy)]
struct Drag {
    app: usize,
    offset_x: i32,
    offset_y: i32,
}

pub struct DesktopState {
    platform: &'static str,
    total_memory_mb: u32,
    pub(crate) apps: Vec<Slot>,
    pub(crate) settings: Settings,
    pub(crate) start_open: bool,
    pub(crate) context_menu_pos: Option<(i32, i32)>,
    pub(crate) hover: Target,
    pub(crate) uptime_ms: u32,
    order: Vec<usize>,
    drag: Option<Drag>,
    dirty: bool,
    wallpaper_dirty: bool,
}

impl DesktopState {
    pub fn new(platform: &'static str, total_memory_mb: u32, apps: Vec<Box<dyn App>>) -> DesktopState {
        let mut state = DesktopState {
            platform,
            total_memory_mb,
            apps: Vec::new(),
            settings: Settings::new(),
            start_open: false,
            context_menu_pos: None,
            hover: Target::Nothing,
            uptime_ms: 0,
            order: Vec::new(),
            drag: None,
            dirty: true,
            wallpaper_dirty: true,
        };
        for app in apps {
            state.install_app(app);
        }
        state
    }

    pub fn install_app(&mut self, app: Box<dyn App>) {
        let index = self.apps.len();
        self.apps.push(Slot {
            app,
            window: WindowState {
                open: false,
                minimized: false,
                maximized: false,
                placed: false,
                x: 0,
                y: 0,
            },
        });
        self.order.push(index);
        self.dirty = true;
    }

    pub fn app_count(&self) -> usize {
        self.apps.len()
    }

    pub fn wallpaper_index(&self) -> usize {
        self.settings.wallpaper
    }

    pub fn take_dirty(&mut self) -> bool {
        core::mem::replace(&mut self.dirty, false)
    }

    pub fn take_wallpaper_dirty(&mut self) -> bool {
        core::mem::replace(&mut self.wallpaper_dirty, false)
    }

    pub(crate) fn focused(&self) -> Option<usize> {
        for position in (0..self.order.len()).rev() {
            let index = self.order[position];
            if self.apps[index].window.open && !self.apps[index].window.minimized {
                return Some(index);
            }
        }
        None
    }

    pub(crate) fn window_rect(&self, index: usize, unit: i32, screen_width: i32, screen_height: i32) -> Rect {
        let slot = &self.apps[index];
        if slot.window.maximized {
            return Rect::new(0, 0, screen_width, screen_height - taskbar_height(unit));
        }

        let info = slot.app.info();
        let width = (info.width_units * unit).min(screen_width);
        let height = (info.height_units * unit).min(screen_height - taskbar_height(unit));
        Rect::new(slot.window.x, slot.window.y, width, height)
    }

    fn context(&self, width: i32, height: i32) -> Context {
        Context {
            platform: self.platform,
            screen_width: width,
            screen_height: height,
            unit: unit_for(height),
            uptime_ms: self.uptime_ms,
            settings: self.settings,
            total_memory_mb: self.total_memory_mb,
        }
    }

    fn raise(&mut self, app: usize) {
        if let Some(position) = self.order.iter().position(|&index| index == app) {
            self.order.remove(position);
        }
        self.order.push(app);
        self.dirty = true;
    }

    fn apply_settings(&mut self, settings: Settings) {
        if settings != self.settings {
            if settings.wallpaper != self.settings.wallpaper {
                self.wallpaper_dirty = true;
            }
            self.settings = settings;
            self.dirty = true;
        }
    }

    fn open_app(&mut self, app: usize, width: i32, height: i32, unit: i32) {
        if app >= self.apps.len() {
            return;
        }
        if !self.apps[app].window.open {
            let info = self.apps[app].app.info();
            let window_width = (info.width_units * unit).min(width);
            let window_height = (info.height_units * unit).min(height - taskbar_height(unit));
            let area_height = height - taskbar_height(unit);
            let cascade = (app % 4) as i32;
            if !self.apps[app].window.placed {
                self.apps[app].window.x = ((width - window_width) / 2 - unit * 3 + cascade * unit * 5).max(0);
                self.apps[app].window.y = ((area_height - window_height) / 2 - unit + cascade * unit * 2).max(0);
                self.apps[app].window.placed = true;
            }
            self.apps[app].window.open = true;
            let mut ctx = self.context(width, height);
            self.apps[app].app.opened(&mut ctx);
            self.apply_settings(ctx.settings);
        }
        self.apps[app].window.minimized = false;
        self.raise(app);
    }

    fn close_app(&mut self, app: usize, width: i32, height: i32) {
        if self.apps[app].window.open {
            self.apps[app].window.open = false;
            self.apps[app].window.minimized = false;
            self.apps[app].window.maximized = false;
            let mut ctx = self.context(width, height);
            self.apps[app].app.closed(&mut ctx);
            self.apply_settings(ctx.settings);
            self.dirty = true;
        }
    }

    fn task_slot(&self, app: usize) -> i32 {
        self.apps[..app].iter().filter(|slot| slot.window.open).count() as i32
    }

    fn hit(&self, x: i32, y: i32, width: i32, height: i32) -> Target {
        let unit = unit_for(height);

        if let Some(pos) = self.context_menu_pos {
            let menu = context_menu_rect(pos.0, pos.1, width, height, unit);
            for idx in 0..4 {
                if inside(context_item_rect(menu, idx, unit), x, y) {
                    return Target::ContextItem(idx);
                }
            }
            if inside(menu, x, y) {
                return Target::ContextMenu;
            }
        }

        let visible = visible_start_apps(self.apps.len(), height, unit);
        if self.start_open {
            for index in 0..visible + 2 {
                if inside(start_item(index, visible, height, unit), x, y) {
                    return Target::StartItem(index);
                }
            }
            if inside(start_menu(visible, height, unit), x, y) {
                return Target::StartBody;
            }
        }

        if inside(start_button(height, unit), x, y) {
            return Target::StartButton;
        }

        for index in 0..self.apps.len() {
            if self.apps[index].window.open && inside(task_button(self.task_slot(index), height, unit), x, y) {
                return Target::TaskButton(index);
            }
        }

        if inside(taskbar_rect(width, height, unit), x, y) {
            return Target::Taskbar;
        }

        for position in (0..self.order.len()).rev() {
            let index = self.order[position];
            if !self.apps[index].window.open || self.apps[index].window.minimized {
                continue;
            }
            let area = self.window_rect(index, unit, width, height);
            if !inside(area, x, y) {
                continue;
            }
            if inside(close_button(area, unit), x, y) {
                return Target::WinClose(index);
            }
            if inside(max_button(area, unit), x, y) {
                return Target::WinMax(index);
            }
            if inside(min_button(area, unit), x, y) {
                return Target::WinMin(index);
            }
            if inside(title_bar(area, unit), x, y) {
                return Target::WinTitle(index);
            }
            return Target::WinBody(index);
        }

        for index in 0..self.apps.len() {
            if inside(desktop_icon(index, height, unit), x, y) {
                return Target::Icon(index);
            }
        }

        Target::Desktop
    }

    fn click(&mut self, target: Target, mouse: &MouseState, width: i32, height: i32, unit: i32) -> DesktopAction {
        let keeps_menu = matches!(target, Target::StartButton | Target::StartItem(_) | Target::StartBody);
        if !keeps_menu && self.start_open {
            self.start_open = false;
            self.dirty = true;
        }

        let keeps_ctx = matches!(target, Target::ContextMenu | Target::ContextItem(_));
        if !keeps_ctx && self.context_menu_pos.is_some() {
            self.context_menu_pos = None;
            self.dirty = true;
        }

        match target {
            Target::StartButton => {
                self.start_open = !self.start_open;
                self.dirty = true;
            }
            Target::StartItem(index) => {
                let visible = visible_start_apps(self.apps.len(), height, unit);
                self.start_open = false;
                self.dirty = true;
                if index < visible {
                    self.open_app(index, width, height, unit);
                } else if index == visible {
                    return DesktopAction::Reboot;
                } else {
                    return DesktopAction::Shutdown;
                }
            }
            Target::TaskButton(app) => {
                if self.apps[app].window.minimized {
                    self.apps[app].window.minimized = false;
                    self.raise(app);
                } else if self.focused() == Some(app) {
                    self.apps[app].window.minimized = true;
                } else {
                    self.raise(app);
                }
                self.dirty = true;
            }
            Target::Icon(app) => self.open_app(app, width, height, unit),
            Target::WinClose(app) => self.close_app(app, width, height),
            Target::WinMax(app) => {
                self.apps[app].window.maximized = !self.apps[app].window.maximized;
                self.raise(app);
                self.dirty = true;
            }
            Target::WinMin(app) => {
                self.apps[app].window.minimized = true;
                self.dirty = true;
            }
            Target::WinTitle(app) => {
                self.raise(app);
                if !self.apps[app].window.maximized {
                    let area = self.window_rect(app, unit, width, height);
                    self.drag = Some(Drag {
                        app,
                        offset_x: mouse.x - area.x,
                        offset_y: mouse.y - area.y,
                    });
                }
            }
            Target::WinBody(app) => self.raise(app),
            Target::ContextItem(idx) => {
                self.context_menu_pos = None;
                self.dirty = true;
                match idx {
                    0 => {
                        self.settings.wallpaper = (self.settings.wallpaper + 1) % WALLPAPER_COUNT;
                        self.wallpaper_dirty = true;
                    }
                    1 => self.open_app(0, width, height, unit),
                    2 => self.open_app(1.min(self.apps.len().saturating_sub(1)), width, height, unit),
                    _ => {}
                }
            }
            _ => {}
        }
        DesktopAction::None
    }

    fn dispatch(&mut self, target: Target, mouse: &MouseState, key: Option<KeyCode>, width: i32, height: i32, unit: i32) {
        let pointer_app = match target {
            Target::WinBody(app) if self.drag.is_none() => Some(app),
            _ => None,
        };
        let focused = self.focused();
        let mut to_close: Option<usize> = None;
        let mut new_app_to_install: Option<Box<dyn App>> = None;

        for index in 0..self.apps.len() {
            if !self.apps[index].window.open || self.apps[index].window.minimized {
                continue;
            }
            let area = content_area(self.window_rect(index, unit, width, height), unit);
            let pointer = if pointer_app == Some(index) {
                Some(Pointer {
                    x: mouse.x,
                    y: mouse.y,
                    down: mouse.left_down,
                    clicked: mouse.left_clicked,
                })
            } else {
                None
            };
            let input = Input {
                area,
                pointer,
                key: if focused == Some(index) { key } else { None },
                focused: focused == Some(index),
            };
            let mut ctx = self.context(width, height);
            let update = self.apps[index].app.update(&mut ctx, &input);
            if update.redraw {
                self.dirty = true;
            }
            if update.close {
                to_close = Some(index);
            }
            if let Some(app) = update.install_app {
                new_app_to_install = Some(app);
            }
            self.apply_settings(ctx.settings);
        }

        if let Some(index) = to_close {
            self.close_app(index, width, height);
        }

        if let Some(app) = new_app_to_install {
            self.install_app(app);
            self.dirty = true;
        }
    }

    pub fn update(&mut self, mouse: &MouseState, key: Option<KeyCode>, width: i32, height: i32, uptime_ms: u32) -> DesktopAction {
        let unit = unit_for(height);
        if uptime_ms / 1000 != self.uptime_ms / 1000 {
            self.dirty = true;
        }
        self.uptime_ms = uptime_ms;
        let menu_was_open = self.start_open;
        if key == Some(KeyCode::Escape) {
            if self.start_open {
                self.start_open = false;
                self.dirty = true;
            }
            if self.context_menu_pos.is_some() {
                self.context_menu_pos = None;
                self.dirty = true;
            }
        }

        if mouse.right_clicked {
            self.context_menu_pos = Some((mouse.x, mouse.y));
            self.dirty = true;
        }

        if let Some(drag) = self.drag {
            if mouse.left_down {
                let window_width = self.window_rect(drag.app, unit, width, height).width;
                let x = (mouse.x - drag.offset_x).clamp(-(window_width - unit * 8), width - unit * 8);
                let y = (mouse.y - drag.offset_y).clamp(0, height - taskbar_height(unit) - title_height(unit));
                if x != self.apps[drag.app].window.x || y != self.apps[drag.app].window.y {
                    self.apps[drag.app].window.x = x;
                    self.apps[drag.app].window.y = y;
                    self.dirty = true;
                }
            } else {
                self.drag = None;
            }
        }
        let target = self.hit(mouse.x, mouse.y, width, height);
        let hover = if self.drag.is_some() { Target::Nothing } else { target };
        if hover != self.hover {
            self.hover = hover;
            self.dirty = true;
        }
        let mut action = DesktopAction::None;
        if mouse.left_clicked {
            action = self.click(target, mouse, width, height, unit);
        }
        let app_key = if menu_was_open && key == Some(KeyCode::Escape) { None } else { key };
        self.dispatch(target, mouse, app_key, width, height, unit);
        action
    }
}

pub fn draw_wallpaper(canvas: &mut Canvas<'_>, index: usize) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let wallpaper = &WALLPAPERS[index % WALLPAPER_COUNT];
    canvas.vertical_gradient(wallpaper.top, wallpaper.bottom);
    for glow in wallpaper.glows.iter() {
        canvas.blend_glow(
            width * glow.x / 100,
            height * glow.y / 100,
            height * glow.radius / 100,
            glow.color,
            glow.strength,
        );
    }
}

pub fn draw(canvas: &mut Canvas<'_>, wallpaper: &Canvas<'_>, state: &DesktopState) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let unit = unit_for(height);
    let ctx = state.context(width, height);
    let accent = state.settings.accent_color();
    canvas.copy_rect_from(wallpaper, 0, 0, 0, 0, width, height);
    shell::draw_icons(canvas, state, height, unit);
    let focused = state.focused();
    for position in 0..state.order.len() {
        let index = state.order[position];
        let slot = &state.apps[index];
        if !slot.window.open || slot.window.minimized {
            continue;
        }
        let area = state.window_rect(index, unit, width, height);
        window::draw_frame(
            canvas,
            slot.app.info(),
            index,
            state.hover,
            accent,
            area,
            focused == Some(index),
            slot.window.maximized,
            unit,
        );
        slot.app.draw(canvas, content_area(area, unit), &ctx);
    }
    shell::draw_taskbar(canvas, state, width, height, unit);
    if state.start_open {
        shell::draw_start_menu(canvas, state, height, unit);
    }
    if let Some(pos) = state.context_menu_pos {
        shell::draw_context_menu(canvas, state, pos, width, height, unit);
    }
}

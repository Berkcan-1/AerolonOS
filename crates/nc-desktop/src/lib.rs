#![no_std]

extern crate alloc;

mod layout;
mod shell;
mod theme;
mod window;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use nc_app::theme::{WALLPAPERS, WALLPAPER_COUNT};
use nc_app::{App, Context, Input, Pointer, Settings};
use nc_fs::{FileType, VfsManager};
use nc_gfx::{rgb, Canvas, Rect};
use nc_hal::{KeyCode, MouseState};

use layout::{
    animated_window_rect, close_button, content_area, context_item_rect, context_menu_rect,
    desktop_icon, desktop_modal_rect, dock_app_pill, dock_rect, dock_start_orb, inside, max_button,
    min_button, snake_head_pod_rect, snake_node_rect, snake_power_buttons,
    task_manager_action_btn, task_manager_close_rect, task_manager_rect, task_manager_row_rect,
    title_bar, title_height, unit_for,
};
use theme::{AQUA_CORE, AQUA_DEEP, AZURE_CORE, AZURE_DEEP, DANGER_CORAL, WARN_AMBER};

const DOUBLE_CLICK_MS: u32 = 460;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DesktopAction {
    None,
    Shutdown,
    Reboot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DesktopIconId {
    App(usize),
    Fs(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuKind {
    DesktopSurface,
    AppItem(usize),
    FsItem(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Nothing,
    Desktop,
    AppIcon(usize),
    FsIcon(usize),
    DockBody,
    StartButton,
    DockApp(usize),
    StartNode(usize),
    StartPod,
    PowerReboot,
    PowerShutdown,
    WinBody(usize),
    WinTitle(usize),
    WinClose(usize),
    WinMax(usize),
    WinMin(usize),
    ContextMenu,
    ContextItem(usize),
    ModalBody,
    ModalConfirm,
    ModalCancel,
    TaskMgrTitle,
    TaskMgrClose,
    TaskMgrRow(usize),
    TaskMgrKill,
    TaskMgrOptimize,
    TaskMgrBody,
}

#[derive(Clone)]
pub struct DesktopFsItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub custom_pos: Option<(i32, i32)>,
}

pub enum DesktopModal {
    None,
    CreateFolder {
        name_buf: String,
    },
    CreateTextFile {
        name_buf: String,
    },
    RenameItem {
        old_name: String,
        new_buf: String,
        is_dir: bool,
    },
    EditTextFile {
        name: String,
        content: String,
    },
}

#[derive(Clone, Copy)]
pub struct BgService {
    pub pid: u32,
    pub name: &'static str,
    pub cpu_pct: u8,
    pub mem_mb: u16,
    pub ticks: u64,
}

pub struct TaskRowView<'a> {
    pub pid: u32,
    pub name: &'a str,
    pub status: &'static str,
    pub cpu_pct: u8,
    pub mem_mb: u16,
    pub is_app: bool,
    pub app_index: Option<usize>,
}

pub struct TaskManagerState {
    pub open: bool,
    pub x: i32,
    pub y: i32,
    pub selected_row: usize,
    pub cpu_history: [u8; 32],
    pub hist_cursor: usize,
    pub dragging: Option<(i32, i32)>,
}

#[derive(Clone, Copy)]
pub(crate) struct WindowState {
    pub open: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub placed: bool,
    pub x: i32,
    pub y: i32,
    pub launch_delay: u8,
    pub launch_total: u8,
    pub open_anim: u32,
}

pub(crate) struct Slot {
    pub app: Box<dyn App>,
    pub window: WindowState,
    pub pinned_dock: bool,
    pub pinned_start: bool,
    pub icon_pos: Option<(i32, i32)>,
}

#[derive(Clone, Copy)]
struct Drag {
    app: usize,
    offset_x: i32,
    offset_y: i32,
}

#[derive(Clone, Copy)]
struct IconDrag {
    id: DesktopIconId,
    offset_x: i32,
    offset_y: i32,
    start_x: i32,
    start_y: i32,
    moved: bool,
}

pub struct DesktopState {
    platform: &'static str,
    pub(crate) total_memory_mb: u32,
    pub(crate) apps: Vec<Slot>,
    pub(crate) fs_items: Vec<DesktopFsItem>,
    pub(crate) bg_services: [BgService; 5],
    pub(crate) task_mgr: TaskManagerState,
    vfs: VfsManager,
    desktop_dir: &'static str,
    pub(crate) settings: Settings,
    pub(crate) start_open: bool,
    pub(crate) start_anim: u32,
    pub(crate) dock_reveal: u32,
    pub(crate) context_menu: Option<((i32, i32), ContextMenuKind)>,
    pub(crate) modal: DesktopModal,
    pub(crate) hover: Target,
    pub(crate) selected_icon: Option<DesktopIconId>,
    last_click_icon: Option<DesktopIconId>,
    last_click_ms: u32,
    pub(crate) uptime_ms: u32,
    order: Vec<usize>,
    drag: Option<Drag>,
    icon_drag: Option<IconDrag>,
    dirty: bool,
    wallpaper_dirty: bool,
}

impl DesktopState {
    pub fn new(
        platform: &'static str,
        total_memory_mb: u32,
        apps: Vec<Box<dyn App>>,
    ) -> DesktopState {
        let mut vfs = VfsManager::new();
        let desktop_dir = if vfs.create_dir_path("/data/desktop").is_ok()
            || vfs.list_dir_path("/data/desktop").is_ok()
        {
            "/data/desktop"
        } else {
            let _ = vfs.create_dir_path("/ram/desktop");
            "/ram/desktop"
        };

        let readme_path = alloc::format!("{}/Masaustu_Notu.txt", desktop_dir);
        if vfs.read_file_path(&readme_path).is_err() {
            let _ = vfs.write_file_path(
                &readme_path,
                b"NetizenCore Masaustune Hos Geldiniz!\n\n- Masaustu simgelerine TEK TIKLAYARAK secebilir ve surukleyip tasiyabilirsiniz.\n- Simgeleri CIFT TIKLAYARAK acabilirsiniz.\n- Herhangi bir yere sag tiklayip 'Gorev Yoneticisini Ac' ile arka plan islemlerini yonetebilirsiniz.\n",
            );
        }

        let mut state = DesktopState {
            platform,
            total_memory_mb,
            apps: Vec::new(),
            fs_items: Vec::new(),
            bg_services: [
                BgService {
                    pid: 1,
                    name: "cekirdek.zamanlayici",
                    cpu_pct: 2,
                    mem_mb: 14,
                    ticks: 0,
                },
                BgService {
                    pid: 2,
                    name: "vfs.indeksleyici",
                    cpu_pct: 1,
                    mem_mb: 8,
                    ticks: 0,
                },
                BgService {
                    pid: 3,
                    name: "ag.anahtarlama",
                    cpu_pct: 2,
                    mem_mb: 10,
                    ticks: 0,
                },
                BgService {
                    pid: 4,
                    name: "grafik.besteci",
                    cpu_pct: 4,
                    mem_mb: 18,
                    ticks: 0,
                },
                BgService {
                    pid: 5,
                    name: "donanim.gozcusu",
                    cpu_pct: 1,
                    mem_mb: 6,
                    ticks: 0,
                },
            ],
            task_mgr: TaskManagerState {
                open: false,
                x: 180,
                y: 80,
                selected_row: 0,
                cpu_history: [8; 32],
                hist_cursor: 0,
                dragging: None,
            },
            vfs,
            desktop_dir,
            settings: Settings::new(),
            start_open: false,
            start_anim: 0,
            dock_reveal: 256,
            context_menu: None,
            modal: DesktopModal::None,
            hover: Target::Nothing,
            selected_icon: None,
            last_click_icon: None,
            last_click_ms: 0,
            uptime_ms: 0,
            order: Vec::new(),
            drag: None,
            icon_drag: None,
            dirty: true,
            wallpaper_dirty: true,
        };
        for app in apps {
            state.install_app(app);
        }
        state.sync_desktop_fs();
        state
    }

    pub(crate) fn blur_radius(&self, base: usize) -> usize {
        match self.settings.glass_blur {
            0 => 0,
            1 => (base / 2).max(2),
            2 => base,
            _ => (base * 3 / 2).min(16),
        }
    }

    pub(crate) fn launching_app(&self) -> Option<usize> {
        self.apps.iter().position(|s| s.window.launch_delay > 0)
    }

    pub(crate) fn total_cpu_percent(&self) -> u32 {
        let mut sum = 0u32;
        for s in &self.bg_services {
            sum += s.cpu_pct as u32;
        }
        for slot in &self.apps {
            if slot.window.launch_delay > 0 {
                sum += 18;
            } else if slot.window.open && !slot.window.minimized {
                sum += 5;
            } else if slot.window.open {
                sum += 1;
            }
        }
        sum.min(100)
    }

    pub(crate) fn used_ram_mb(&self) -> u32 {
        let mut sum = 0u32;
        for s in &self.bg_services {
            sum += s.mem_mb as u32;
        }
        for slot in &self.apps {
            if slot.window.open || slot.window.launch_delay > 0 {
                sum += 12;
            }
        }
        sum.min(self.total_memory_mb)
    }

    pub(crate) fn collect_task_rows(&self) -> Vec<TaskRowView<'_>> {
        let mut list = Vec::new();
        for (idx, slot) in self.apps.iter().enumerate() {
            if slot.window.open || slot.window.launch_delay > 0 {
                let status = if slot.window.launch_delay > 0 {
                    "Yukleniyor"
                } else if slot.window.minimized {
                    "Simge Durumu"
                } else {
                    "Calisiyor"
                };
                let cpu = if slot.window.launch_delay > 0 {
                    18
                } else if slot.window.minimized {
                    1
                } else {
                    5
                };
                list.push(TaskRowView {
                    pid: 100 + idx as u32,
                    name: &slot.app.info().name,
                    status,
                    cpu_pct: cpu,
                    mem_mb: 12,
                    is_app: true,
                    app_index: Some(idx),
                });
            }
        }
        for s in &self.bg_services {
            list.push(TaskRowView {
                pid: s.pid,
                name: s.name,
                status: "Arka Plan",
                cpu_pct: s.cpu_pct,
                mem_mb: s.mem_mb,
                is_app: false,
                app_index: None,
            });
        }
        list
    }

    fn tick_background_processes(&mut self, uptime_ms: u32) {
        let phase = (uptime_ms / 250) as usize;
        let launching = self.launching_app().is_some();
        for (i, s) in self.bg_services.iter_mut().enumerate() {
            s.ticks = s.ticks.wrapping_add(1);
            let jitter = ((phase + i * 3) % 4) as u8;
            s.cpu_pct = match i {
                0 => 2 + (jitter & 1),
                1 => {
                    if launching {
                        12 + jitter
                    } else {
                        1 + (jitter & 1)
                    }
                }
                2 => 2 + (jitter % 2),
                3 => {
                    if self.settings.wallpaper_animated {
                        6 + jitter
                    } else {
                        3
                    }
                }
                _ => 1,
            };
        }
        let total = self.total_cpu_percent() as u8;
        let cur = self.task_mgr.hist_cursor;
        self.task_mgr.cpu_history[cur] = total;
        self.task_mgr.hist_cursor = (cur + 1) % 32;
        if self.task_mgr.open {
            self.dirty = true;
        }
    }

    pub fn sync_desktop_fs(&mut self) {
        let old_items = core::mem::take(&mut self.fs_items);
        if let Ok(entries) = self.vfs.list_dir_path(self.desktop_dir) {
            for e in entries {
                let prev_pos = old_items
                    .iter()
                    .find(|old| old.name == e.name)
                    .and_then(|old| old.custom_pos);
                self.fs_items.push(DesktopFsItem {
                    name: e.name,
                    is_dir: e.file_type == FileType::Directory,
                    size: e.size,
                    custom_pos: prev_pos,
                });
            }
        }
        self.dirty = true;
    }

    pub(crate) fn app_icon_rect(&self, index: usize, height: i32, unit: i32) -> Rect {
        let def = desktop_icon(index, height, unit);
        if let Some((px, py)) = self.apps[index].icon_pos {
            Rect::new(px, py, def.width, def.height)
        } else {
            def
        }
    }

    pub(crate) fn fs_icon_rect(&self, fs_idx: usize, height: i32, unit: i32) -> Rect {
        let def = desktop_icon(self.apps.len() + fs_idx, height, unit);
        if let Some((px, py)) = self.fs_items[fs_idx].custom_pos {
            Rect::new(px, py, def.width, def.height)
        } else {
            def
        }
    }

    pub fn install_app(&mut self, app: Box<dyn App>) {
        let target_id = app.info().id.clone();
        if let Some(existing) = self
            .apps
            .iter()
            .position(|slot| slot.app.info().id == target_id)
        {
            self.apps[existing].window.open = true;
            self.apps[existing].window.minimized = false;
            self.apps[existing].window.open_anim = 64;
            self.raise(existing);
            self.dirty = true;
            return;
        }
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
                launch_delay: 0,
                launch_total: 14,
                open_anim: 256,
            },
            pinned_dock: index < 4,
            pinned_start: true,
            icon_pos: None,
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

    pub(crate) fn dock_indices(&self) -> Vec<usize> {
        let mut list = Vec::new();
        for (i, slot) in self.apps.iter().enumerate() {
            if slot.pinned_dock || slot.window.open || slot.window.launch_delay > 0 {
                list.push(i);
            }
        }
        list
    }

    pub(crate) fn start_indices(&self) -> Vec<usize> {
        let mut list = Vec::new();
        for (i, slot) in self.apps.iter().enumerate() {
            if slot.pinned_start {
                list.push(i);
            }
        }
        if list.is_empty() {
            for i in 0..self.apps.len() {
                list.push(i);
            }
        }
        list
    }

    pub(crate) fn context_menu_item_count(&self, kind: ContextMenuKind) -> usize {
        match kind {
            ContextMenuKind::DesktopSurface => 8,
            ContextMenuKind::AppItem(_) => 5,
            ContextMenuKind::FsItem(_) => 4,
        }
    }

    pub(crate) fn context_menu_entry(
        &self,
        kind: ContextMenuKind,
        idx: usize,
    ) -> (&'static str, &'static str, u32) {
        match kind {
            ContextMenuKind::DesktopSurface => match idx {
                0 => ("Gorev Yoneticisini Ac", "Islem", AQUA_CORE),
                1 => ("Yeni Klasor Olustur", "/data", AZURE_CORE),
                2 => ("Yeni Metin Belgesi", ".txt", AQUA_CORE),
                3 => ("Masaustunu Yenile ve Hizala", "Izgara", AZURE_DEEP),
                4 => ("Sonraki Duvar Kagidi", "Tema", AQUA_DEEP),
                5 => (
                    if self.settings.wallpaper_animated {
                        "Canli Duvar Kagidi: Aktif"
                    } else {
                        "Canli Duvar Kagidi: Kapali"
                    },
                    "Hiz",
                    WARN_AMBER,
                ),
                6 => ("Dosya Yoneticisini Ac", "VFS", AZURE_CORE),
                _ => ("Ayarlar ve Ag Denetimi", "Sistem", AQUA_CORE),
            },
            ContextMenuKind::AppItem(app_idx) => {
                let pinned_dock = self
                    .apps
                    .get(app_idx)
                    .map(|s| s.pinned_dock)
                    .unwrap_or(false);
                let pinned_start = self
                    .apps
                    .get(app_idx)
                    .map(|s| s.pinned_start)
                    .unwrap_or(false);
                let is_open = self
                    .apps
                    .get(app_idx)
                    .map(|s| s.window.open)
                    .unwrap_or(false);
                match idx {
                    0 => ("Uygulamayi Ac / One Getir", "Calistir", AZURE_CORE),
                    1 => (
                        if pinned_dock {
                            "Ana Bardan Kaldir"
                        } else {
                            "Ana Bara Sabitle"
                        },
                        "Bar",
                        AQUA_CORE,
                    ),
                    2 => (
                        if pinned_start {
                            "Baslat Yilanindan Cikar"
                        } else {
                            "Baslat Yilanina Ekle"
                        },
                        "Yilan",
                        WARN_AMBER,
                    ),
                    3 => ("Gorev Yoneticisini Ac", "Islem", AQUA_DEEP),
                    _ => (
                        if is_open {
                            "Pencereyi Kapat"
                        } else {
                            "Hazir Durumda"
                        },
                        "",
                        DANGER_CORAL,
                    ),
                }
            }
            ContextMenuKind::FsItem(_) => match idx {
                0 => ("Ac / Icerigi Duzenle", "Not", AZURE_CORE),
                1 => ("Yeniden Adlandir", "Ad", AQUA_CORE),
                2 => ("Gorev Yoneticisini Ac", "Islem", WARN_AMBER),
                _ => ("Masaustunden Sil", "Sil", DANGER_CORAL),
            },
        }
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

    pub(crate) fn window_rect(
        &self,
        index: usize,
        unit: i32,
        screen_width: i32,
        screen_height: i32,
    ) -> Rect {
        let slot = &self.apps[index];
        let base = if slot.window.maximized {
            Rect::new(0, 0, screen_width, screen_height)
        } else {
            let info = slot.app.info();
            let width = (info.width_units * unit).min(screen_width);
            let height = (info.height_units * unit).min(screen_height - unit * 2);
            Rect::new(slot.window.x, slot.window.y, width, height)
        };
        animated_window_rect(base, slot.window.open_anim, unit)
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
        if !self.apps[app].window.open && self.apps[app].window.launch_delay == 0 {
            let info = self.apps[app].app.info();
            let window_width = (info.width_units * unit).min(width);
            let window_height = (info.height_units * unit).min(height - unit * 2);
            let cascade = (app % 4) as i32;
            if !self.apps[app].window.placed {
                self.apps[app].window.x =
                    ((width - window_width) / 2 - unit * 3 + cascade * unit * 4).max(0);
                self.apps[app].window.y =
                    ((height - window_height) / 2 - unit * 2 + cascade * unit * 2).max(0);
                self.apps[app].window.placed = true;
            }
            let bg_load_extra = (self.total_cpu_percent() / 12) as u8;
            let delay_frames = match self.settings.launch_anim_speed {
                1 => 20 + bg_load_extra,
                2 => 11 + bg_load_extra,
                _ => 2,
            };
            self.apps[app].window.launch_total = delay_frames.max(2);
            self.apps[app].window.launch_delay = delay_frames.max(2);
            self.apps[app].window.open_anim = 0;
            self.dirty = true;
            return;
        }
        if self.apps[app].window.open {
            if self.apps[app].window.minimized {
                self.apps[app].window.minimized = false;
                self.apps[app].window.open_anim = 64;
            }
            self.raise(app);
        }
    }

    fn open_app_by_id(&mut self, target_id: &str, width: i32, height: i32, unit: i32) {
        if let Some(idx) = self
            .apps
            .iter()
            .position(|slot| slot.app.info().id == target_id)
        {
            self.open_app(idx, width, height, unit);
        } else if !self.apps.is_empty() {
            self.open_app(0, width, height, unit);
        }
    }

    fn close_app(&mut self, app: usize, width: i32, height: i32) {
        if app < self.apps.len()
            && (self.apps[app].window.open || self.apps[app].window.launch_delay > 0)
        {
            self.apps[app].window.open = false;
            self.apps[app].window.launch_delay = 0;
            self.apps[app].window.minimized = false;
            self.apps[app].window.maximized = false;
            let mut ctx = self.context(width, height);
            self.apps[app].app.closed(&mut ctx);
            self.apply_settings(ctx.settings);
            self.dirty = true;
        }
    }

    fn open_fs_item(&mut self, fs_idx: usize, width: i32, height: i32, unit: i32) {
        if fs_idx >= self.fs_items.len() {
            return;
        }
        let item = self.fs_items[fs_idx].clone();
        if item.is_dir {
            self.open_app_by_id("nc.files", width, height, unit);
        } else {
            let full = alloc::format!("{}/{}", self.desktop_dir, item.name);
            let bytes = self.vfs.read_file_path(&full).unwrap_or_default();
            let text = String::from_utf8_lossy(&bytes).into_owned();
            self.modal = DesktopModal::EditTextFile {
                name: item.name,
                content: text,
            };
            self.dirty = true;
        }
    }

    fn commit_modal(&mut self) {
        let current = core::mem::replace(&mut self.modal, DesktopModal::None);
        match current {
            DesktopModal::CreateFolder { name_buf } => {
                let clean = if name_buf.trim().is_empty() {
                    "Yeni_Klasor"
                } else {
                    name_buf.trim()
                };
                let path = alloc::format!("{}/{}", self.desktop_dir, clean);
                let _ = self.vfs.create_dir_path(&path);
                self.sync_desktop_fs();
            }
            DesktopModal::CreateTextFile { name_buf } => {
                let raw = if name_buf.trim().is_empty() {
                    "notlar.txt"
                } else {
                    name_buf.trim()
                };
                let final_name = if raw.contains('.') {
                    String::from(raw)
                } else {
                    alloc::format!("{}.txt", raw)
                };
                let path = alloc::format!("{}/{}", self.desktop_dir, final_name);
                let _ = self.vfs.write_file_path(
                    &path,
                    b"Masaustu metin belgesi.\nCift tiklayarak duzenleyebilirsiniz.\n",
                );
                self.sync_desktop_fs();
            }
            DesktopModal::RenameItem {
                old_name,
                new_buf,
                is_dir,
            } => {
                let clean = new_buf.trim();
                if !clean.is_empty() && clean != old_name {
                    let old_path = alloc::format!("{}/{}", self.desktop_dir, old_name);
                    let new_path = alloc::format!("{}/{}", self.desktop_dir, clean);
                    if is_dir {
                        if self.vfs.create_dir_path(&new_path).is_ok() {
                            let _ = self.vfs.remove_path(&old_path);
                        }
                    } else if let Ok(data) = self.vfs.read_file_path(&old_path) {
                        if self.vfs.write_file_path(&new_path, &data).is_ok() {
                            let _ = self.vfs.remove_path(&old_path);
                        }
                    }
                    self.sync_desktop_fs();
                }
            }
            DesktopModal::EditTextFile { name, content } => {
                let path = alloc::format!("{}/{}", self.desktop_dir, name);
                let _ = self.vfs.write_file_path(&path, content.as_bytes());
                self.sync_desktop_fs();
            }
            DesktopModal::None => {}
        }
        self.dirty = true;
    }

    fn hit(&self, x: i32, y: i32, width: i32, height: i32) -> Target {
        let unit = unit_for(height);

        if !matches!(self.modal, DesktopModal::None) {
            let tall = matches!(self.modal, DesktopModal::EditTextFile { .. });
            let m_rect = desktop_modal_rect(width, height, unit, tall);
            let btn_w = unit * 7;
            let btn_h = unit * 2;
            let btn_y = m_rect.y + m_rect.height - btn_h - unit;
            let ok_r = Rect::new(
                m_rect.x + m_rect.width - btn_w - unit * 3 / 2,
                btn_y,
                btn_w,
                btn_h,
            );
            let cancel_r = Rect::new(ok_r.x - btn_w - unit / 2, btn_y, btn_w, btn_h);
            if inside(ok_r, x, y) {
                return Target::ModalConfirm;
            }
            if inside(cancel_r, x, y) {
                return Target::ModalCancel;
            }
            if inside(m_rect, x, y) {
                return Target::ModalBody;
            }
        }

        if let Some((pos, kind)) = self.context_menu {
            let count = self.context_menu_item_count(kind);
            let menu = context_menu_rect(pos.0, pos.1, count, width, height, unit);
            for idx in 0..count {
                if inside(context_item_rect(menu, idx, unit), x, y) {
                    return Target::ContextItem(idx);
                }
            }
            if inside(menu, x, y) {
                return Target::ContextMenu;
            }
        }

        if self.task_mgr.open {
            let tm = task_manager_rect(self.task_mgr.x, self.task_mgr.y, unit);
            if inside(tm, x, y) {
                if inside(task_manager_close_rect(tm, unit), x, y) {
                    return Target::TaskMgrClose;
                }
                if inside(title_bar(tm, unit), x, y) {
                    return Target::TaskMgrTitle;
                }
                if inside(task_manager_action_btn(tm, true, unit), x, y) {
                    return Target::TaskMgrKill;
                }
                if inside(task_manager_action_btn(tm, false, unit), x, y) {
                    return Target::TaskMgrOptimize;
                }
                let row_cnt = self.collect_task_rows().len().min(7);
                for r in 0..row_cnt {
                    if inside(task_manager_row_rect(tm, r, unit), x, y) {
                        return Target::TaskMgrRow(r);
                    }
                }
                return Target::TaskMgrBody;
            }
        }

        let dock_items = self.dock_indices();
        let dock = dock_rect(width, height, unit, dock_items.len(), self.dock_reveal);

        if self.start_open && self.start_anim > 32 {
            let snake_apps = self.start_indices();
            let head_pod = snake_head_pod_rect(snake_apps.len(), dock, unit, self.start_anim);
            let (reb_btn, shut_btn) = snake_power_buttons(head_pod, unit);
            if inside(reb_btn, x, y) {
                return Target::PowerReboot;
            }
            if inside(shut_btn, x, y) {
                return Target::PowerShutdown;
            }
            if inside(head_pod, x, y) {
                return Target::StartPod;
            }
            for (seq, &app_idx) in snake_apps.iter().enumerate() {
                let nr = snake_node_rect(seq, dock, unit, self.start_anim);
                if inside(nr, x, y) {
                    return Target::StartNode(app_idx);
                }
            }
        }

        if self.dock_reveal >= 32 {
            if inside(dock_start_orb(dock, unit), x, y) {
                return Target::StartButton;
            }
            for (slot_pos, &app_idx) in dock_items.iter().enumerate() {
                if inside(dock_app_pill(dock, slot_pos, unit), x, y) {
                    return Target::DockApp(app_idx);
                }
            }
            if inside(dock, x, y) {
                return Target::DockBody;
            }
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

        for f_idx in (0..self.fs_items.len()).rev() {
            if inside(self.fs_icon_rect(f_idx, height, unit), x, y) {
                return Target::FsIcon(f_idx);
            }
        }
        for index in (0..self.apps.len()).rev() {
            if inside(self.app_icon_rect(index, height, unit), x, y) {
                return Target::AppIcon(index);
            }
        }
        Target::Desktop
    }

    fn open_task_manager(&mut self, width: i32, height: i32, unit: i32) {
        self.task_mgr.open = true;
        let tm_w = unit * 34;
        let tm_h = unit * 23;
        self.task_mgr.x = ((width - tm_w) / 2).max(unit);
        self.task_mgr.y = ((height - tm_h) / 2 - unit).max(unit);
        self.dirty = true;
    }

    fn execute_context_action(
        &mut self,
        kind: ContextMenuKind,
        idx: usize,
        width: i32,
        height: i32,
        unit: i32,
    ) {
        match kind {
            ContextMenuKind::DesktopSurface => match idx {
                0 => {
                    self.open_task_manager(width, height, unit);
                }
                1 => {
                    let next_num = self.fs_items.len() + 1;
                    self.modal = DesktopModal::CreateFolder {
                        name_buf: alloc::format!("Klasor_{}", next_num),
                    };
                }
                2 => {
                    let next_num = self.fs_items.len() + 1;
                    self.modal = DesktopModal::CreateTextFile {
                        name_buf: alloc::format!("Not_{}.txt", next_num),
                    };
                }
                3 => {
                    for slot in &mut self.apps {
                        slot.icon_pos = None;
                    }
                    self.sync_desktop_fs();
                    for item in &mut self.fs_items {
                        item.custom_pos = None;
                    }
                }
                4 => {
                    self.settings.wallpaper = (self.settings.wallpaper + 1) % WALLPAPER_COUNT;
                    self.wallpaper_dirty = true;
                }
                5 => {
                    self.settings.wallpaper_animated = !self.settings.wallpaper_animated;
                }
                6 => {
                    self.open_app_by_id("nc.files", width, height, unit);
                }
                _ => {
                    self.open_app_by_id("nc.settings", width, height, unit);
                }
            },
            ContextMenuKind::AppItem(app_idx) => {
                if app_idx >= self.apps.len() {
                    return;
                }
                match idx {
                    0 => self.open_app(app_idx, width, height, unit),
                    1 => {
                        self.apps[app_idx].pinned_dock = !self.apps[app_idx].pinned_dock;
                    }
                    2 => {
                        self.apps[app_idx].pinned_start = !self.apps[app_idx].pinned_start;
                    }
                    3 => {
                        self.open_task_manager(width, height, unit);
                    }
                    _ => {
                        self.close_app(app_idx, width, height);
                    }
                }
            }
            ContextMenuKind::FsItem(fs_idx) => {
                if fs_idx >= self.fs_items.len() {
                    return;
                }
                match idx {
                    0 => self.open_fs_item(fs_idx, width, height, unit),
                    1 => {
                        let item = &self.fs_items[fs_idx];
                        self.modal = DesktopModal::RenameItem {
                            old_name: item.name.clone(),
                            new_buf: item.name.clone(),
                            is_dir: item.is_dir,
                        };
                    }
                    2 => {
                        self.open_task_manager(width, height, unit);
                    }
                    _ => {
                        let path =
                            alloc::format!("{}/{}", self.desktop_dir, self.fs_items[fs_idx].name);
                        let _ = self.vfs.remove_path(&path);
                        self.sync_desktop_fs();
                    }
                }
            }
        }
        self.dirty = true;
    }

    fn handle_desktop_icon_click(
        &mut self,
        icon_id: DesktopIconId,
        mouse: &MouseState,
        width: i32,
        height: i32,
        unit: i32,
    ) {
        let is_double = self.last_click_icon == Some(icon_id)
            && self.uptime_ms.wrapping_sub(self.last_click_ms) <= DOUBLE_CLICK_MS;

        self.selected_icon = Some(icon_id);
        self.dirty = true;

        if is_double {
            self.last_click_icon = None;
            self.icon_drag = None;
            match icon_id {
                DesktopIconId::App(app_idx) => self.open_app(app_idx, width, height, unit),
                DesktopIconId::Fs(fs_idx) => self.open_fs_item(fs_idx, width, height, unit),
            }
        } else {
            self.last_click_icon = Some(icon_id);
            self.last_click_ms = self.uptime_ms;
            let rect = match icon_id {
                DesktopIconId::App(app_idx) => self.app_icon_rect(app_idx, height, unit),
                DesktopIconId::Fs(fs_idx) => self.fs_icon_rect(fs_idx, height, unit),
            };
            self.icon_drag = Some(IconDrag {
                id: icon_id,
                offset_x: mouse.x - rect.x,
                offset_y: mouse.y - rect.y,
                start_x: mouse.x,
                start_y: mouse.y,
                moved: false,
            });
        }
    }

    fn click(
        &mut self,
        target: Target,
        mouse: &MouseState,
        width: i32,
        height: i32,
        unit: i32,
    ) -> DesktopAction {
        if !matches!(self.modal, DesktopModal::None) {
            match target {
                Target::ModalConfirm => self.commit_modal(),
                Target::ModalCancel => {
                    self.modal = DesktopModal::None;
                    self.dirty = true;
                }
                Target::ModalBody => {}
                _ => {
                    self.modal = DesktopModal::None;
                    self.dirty = true;
                }
            }
            return DesktopAction::None;
        }

        let keeps_menu = matches!(
            target,
            Target::StartButton
                | Target::StartNode(_)
                | Target::StartPod
                | Target::PowerReboot
                | Target::PowerShutdown
        );
        if !keeps_menu && self.start_open {
            self.start_open = false;
            self.dirty = true;
        }

        let keeps_ctx = matches!(target, Target::ContextMenu | Target::ContextItem(_));
        let active_ctx = self.context_menu;
        if !keeps_ctx && self.context_menu.is_some() {
            self.context_menu = None;
            self.dirty = true;
        }

        if !matches!(target, Target::AppIcon(_) | Target::FsIcon(_)) && self.selected_icon.is_some()
        {
            self.selected_icon = None;
            self.dirty = true;
        }

        match target {
            Target::TaskMgrClose => {
                self.task_mgr.open = false;
                self.dirty = true;
            }
            Target::TaskMgrTitle => {
                self.task_mgr.dragging = Some((
                    mouse.x - self.task_mgr.x,
                    mouse.y - self.task_mgr.y,
                ));
            }
            Target::TaskMgrRow(r) => {
                self.task_mgr.selected_row = r;
                self.dirty = true;
            }
            Target::TaskMgrOptimize => {
                for s in &mut self.bg_services {
                    s.cpu_pct = 1;
                }
                for h in &mut self.task_mgr.cpu_history {
                    *h = (*h / 2).max(4);
                }
                self.dirty = true;
            }
            Target::TaskMgrKill => {
                let target_app = {
                    let rows = self.collect_task_rows();
                    rows.get(self.task_mgr.selected_row)
                        .and_then(|r| r.app_index)
                };
                if let Some(app_idx) = target_app {
                    self.close_app(app_idx, width, height);
                    self.task_mgr.selected_row = 0;
                } else {
                    for s in &mut self.bg_services {
                        s.cpu_pct = 1;
                    }
                }
                self.dirty = true;
            }
            Target::StartButton => {
                self.start_open = !self.start_open;
                self.dirty = true;
            }
            Target::StartNode(app_idx) => {
                self.start_open = false;
                self.open_app(app_idx, width, height, unit);
                self.dirty = true;
            }
            Target::PowerReboot => {
                return DesktopAction::Reboot;
            }
            Target::PowerShutdown => {
                return DesktopAction::Shutdown;
            }
            Target::DockApp(app) => {
                if !self.apps[app].window.open {
                    self.open_app(app, width, height, unit);
                } else if self.apps[app].window.minimized {
                    self.apps[app].window.minimized = false;
                    self.apps[app].window.open_anim = 64;
                    self.raise(app);
                } else if self.focused() == Some(app) {
                    self.apps[app].window.minimized = true;
                } else {
                    self.raise(app);
                }
                self.dirty = true;
            }
            Target::AppIcon(app) => {
                self.handle_desktop_icon_click(DesktopIconId::App(app), mouse, width, height, unit);
            }
            Target::FsIcon(fs_idx) => {
                self.handle_desktop_icon_click(
                    DesktopIconId::Fs(fs_idx),
                    mouse,
                    width,
                    height,
                    unit,
                );
            }
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
                self.context_menu = None;
                if let Some((_, kind)) = active_ctx {
                    self.execute_context_action(kind, idx, width, height, unit);
                }
            }
            _ => {}
        }
        DesktopAction::None
    }

    fn dispatch(
        &mut self,
        target: Target,
        mouse: &MouseState,
        key: Option<KeyCode>,
        width: i32,
        height: i32,
        unit: i32,
    ) {
        let pointer_app = match target {
            Target::WinBody(app)
                if self.drag.is_none()
                    && self.icon_drag.is_none()
                    && self.context_menu.is_none() =>
            {
                Some(app)
            }
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

    fn handle_modal_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Escape => {
                self.modal = DesktopModal::None;
                self.dirty = true;
            }
            KeyCode::Enter => {
                if let DesktopModal::EditTextFile { ref mut content, .. } = self.modal {
                    content.push('\n');
                    self.dirty = true;
                } else {
                    self.commit_modal();
                }
            }
            KeyCode::Backspace => match self.modal {
                DesktopModal::CreateFolder { ref mut name_buf }
                | DesktopModal::CreateTextFile { ref mut name_buf } => {
                    name_buf.pop();
                    self.dirty = true;
                }
                DesktopModal::RenameItem {
                    ref mut new_buf, ..
                } => {
                    new_buf.pop();
                    self.dirty = true;
                }
                DesktopModal::EditTextFile {
                    ref mut content, ..
                } => {
                    content.pop();
                    self.dirty = true;
                }
                DesktopModal::None => {}
            },
            KeyCode::Space => match self.modal {
                DesktopModal::CreateFolder { ref mut name_buf }
                | DesktopModal::CreateTextFile { ref mut name_buf } => {
                    if name_buf.len() < 26 {
                        name_buf.push('_');
                        self.dirty = true;
                    }
                }
                DesktopModal::RenameItem {
                    ref mut new_buf, ..
                } => {
                    if new_buf.len() < 26 {
                        new_buf.push('_');
                        self.dirty = true;
                    }
                }
                DesktopModal::EditTextFile {
                    ref mut content, ..
                } => {
                    content.push(' ');
                    self.dirty = true;
                }
                DesktopModal::None => {}
            },
            KeyCode::Char(c) => match self.modal {
                DesktopModal::CreateFolder { ref mut name_buf }
                | DesktopModal::CreateTextFile { ref mut name_buf } => {
                    if c != '/' && c != '\\' && name_buf.len() < 26 {
                        name_buf.push(c);
                        self.dirty = true;
                    }
                }
                DesktopModal::RenameItem {
                    ref mut new_buf, ..
                } => {
                    if c != '/' && c != '\\' && new_buf.len() < 26 {
                        new_buf.push(c);
                        self.dirty = true;
                    }
                }
                DesktopModal::EditTextFile {
                    ref mut content, ..
                } => {
                    content.push(c);
                    self.dirty = true;
                }
                DesktopModal::None => {}
            },
            _ => {}
        }
    }

    pub fn update(
        &mut self,
        mouse: &MouseState,
        key: Option<KeyCode>,
        width: i32,
        height: i32,
        uptime_ms: u32,
    ) -> DesktopAction {
        let unit = unit_for(height);
        if uptime_ms / 1000 != self.uptime_ms / 1000 {
            self.dirty = true;
        }
        if uptime_ms / 250 != self.uptime_ms / 250 {
            self.tick_background_processes(uptime_ms);
        }
        if self.settings.wallpaper_animated && uptime_ms / 64 != self.uptime_ms / 64 {
            self.dirty = true;
        }
        self.uptime_ms = uptime_ms;

        let mut newly_opened: Option<usize> = None;
        let anim_step = match self.settings.launch_anim_speed {
            1 => 22u32,
            2 => 38u32,
            _ => 84u32,
        };
        for i in 0..self.apps.len() {
            if self.apps[i].window.launch_delay > 0 {
                self.apps[i].window.launch_delay -= 1;
                self.dirty = true;
                if self.apps[i].window.launch_delay == 0 {
                    self.apps[i].window.open = true;
                    self.apps[i].window.minimized = false;
                    self.apps[i].window.open_anim = 32;
                    newly_opened = Some(i);
                }
            } else if self.apps[i].window.open && self.apps[i].window.open_anim < 256 {
                self.apps[i].window.open_anim =
                    (self.apps[i].window.open_anim + anim_step).min(256);
                self.dirty = true;
            }
        }
        if let Some(app_idx) = newly_opened {
            let mut ctx = self.context(width, height);
            self.apps[app_idx].app.opened(&mut ctx);
            self.apply_settings(ctx.settings);
            self.raise(app_idx);
        }

        let want_dock = !self.settings.dock_auto_hide
            || self.start_open
            || mouse.y >= height - unit
            || (self.dock_reveal > 64 && mouse.y >= height - unit * 4 - unit / 2);
        if want_dock && self.dock_reveal < 256 {
            self.dock_reveal = (self.dock_reveal + 36).min(256);
            self.dirty = true;
        } else if !want_dock && self.dock_reveal > 0 {
            self.dock_reveal = self.dock_reveal.saturating_sub(28);
            self.dirty = true;
        }

        if self.start_open && self.start_anim < 256 {
            self.start_anim = (self.start_anim + 36).min(256);
            self.dirty = true;
        } else if !self.start_open && self.start_anim > 0 {
            self.start_anim = self.start_anim.saturating_sub(42);
            self.dirty = true;
        }

        if !matches!(self.modal, DesktopModal::None) {
            if let Some(k) = key {
                self.handle_modal_key(k);
            }
            let target = self.hit(mouse.x, mouse.y, width, height);
            if target != self.hover {
                self.hover = target;
                self.dirty = true;
            }
            if mouse.left_clicked {
                let _ = self.click(target, mouse, width, height, unit);
            }
            return DesktopAction::None;
        }

        let menu_was_open = self.start_open;
        if key == Some(KeyCode::Escape) {
            if self.start_open {
                self.start_open = false;
                self.dirty = true;
            }
            if self.context_menu.is_some() {
                self.context_menu = None;
                self.dirty = true;
            }
            if self.selected_icon.is_some() {
                self.selected_icon = None;
                self.dirty = true;
            }
        }

        if key == Some(KeyCode::Enter) && self.focused().is_none() {
            if let Some(sel) = self.selected_icon {
                match sel {
                    DesktopIconId::App(idx) => self.open_app(idx, width, height, unit),
                    DesktopIconId::Fs(idx) => self.open_fs_item(idx, width, height, unit),
                }
            }
        }

        if let Some((ox, oy)) = self.task_mgr.dragging {
            if mouse.left_down {
                let nx = (mouse.x - ox).clamp(4, width - unit * 16);
                let ny = (mouse.y - oy).clamp(4, height - unit * 6);
                if nx != self.task_mgr.x || ny != self.task_mgr.y {
                    self.task_mgr.x = nx;
                    self.task_mgr.y = ny;
                    self.dirty = true;
                }
            } else {
                self.task_mgr.dragging = None;
            }
        }

        if let Some(drag) = self.drag {
            if mouse.left_down {
                let window_width = self.window_rect(drag.app, unit, width, height).width;
                let x =
                    (mouse.x - drag.offset_x).clamp(-(window_width - unit * 8), width - unit * 8);
                let y = (mouse.y - drag.offset_y).clamp(0, height - title_height(unit) - unit);
                if x != self.apps[drag.app].window.x || y != self.apps[drag.app].window.y {
                    self.apps[drag.app].window.x = x;
                    self.apps[drag.app].window.y = y;
                    self.dirty = true;
                }
            } else {
                self.drag = None;
            }
        }

        if let Some(mut idrag) = self.icon_drag {
            if mouse.left_down {
                let dist = (mouse.x - idrag.start_x).abs() + (mouse.y - idrag.start_y).abs();
                if dist > 4 || idrag.moved {
                    idrag.moved = true;
                    self.icon_drag = Some(idrag);
                    let nx = (mouse.x - idrag.offset_x).clamp(4, width - unit * 7 - 4);
                    let ny = (mouse.y - idrag.offset_y).clamp(4, height - unit * 6 - 4);
                    match idrag.id {
                        DesktopIconId::App(idx) => {
                            if idx < self.apps.len() && self.apps[idx].icon_pos != Some((nx, ny)) {
                                self.apps[idx].icon_pos = Some((nx, ny));
                                self.dirty = true;
                            }
                        }
                        DesktopIconId::Fs(idx) => {
                            if idx < self.fs_items.len()
                                && self.fs_items[idx].custom_pos != Some((nx, ny))
                            {
                                self.fs_items[idx].custom_pos = Some((nx, ny));
                                self.dirty = true;
                            }
                        }
                    }
                }
            } else {
                self.icon_drag = None;
            }
        }

        let target = self.hit(mouse.x, mouse.y, width, height);

        if mouse.right_clicked {
            let kind = match target {
                Target::AppIcon(app_idx) => {
                    self.selected_icon = Some(DesktopIconId::App(app_idx));
                    ContextMenuKind::AppItem(app_idx)
                }
                Target::DockApp(app_idx) | Target::StartNode(app_idx) => {
                    ContextMenuKind::AppItem(app_idx)
                }
                Target::FsIcon(fs_idx) => {
                    self.selected_icon = Some(DesktopIconId::Fs(fs_idx));
                    ContextMenuKind::FsItem(fs_idx)
                }
                _ => ContextMenuKind::DesktopSurface,
            };
            self.context_menu = Some(((mouse.x, mouse.y), kind));
            self.start_open = false;
            self.dirty = true;
        }

        let hover = if self.drag.is_some()
            || self.icon_drag.is_some()
            || self.task_mgr.dragging.is_some()
        {
            Target::Nothing
        } else {
            target
        };
        if hover != self.hover {
            self.hover = hover;
            self.dirty = true;
        }

        let mut action = DesktopAction::None;
        if mouse.left_clicked {
            action = self.click(target, mouse, width, height, unit);
        }

        let app_key = if menu_was_open && key == Some(KeyCode::Escape) {
            None
        } else {
            key
        };
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
    canvas.blend_glow(
        width / 2,
        height * 46 / 100,
        height * 38 / 100,
        rgb(198, 244, 255),
        48,
    );
}

pub fn draw(canvas: &mut Canvas<'_>, wallpaper: &Canvas<'_>, state: &DesktopState) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let unit = unit_for(height);
    let ctx = state.context(width, height);
    let accent = state.settings.accent_color();

    canvas.reset_clip();
    canvas.copy_rect_from(wallpaper, 0, 0, 0, 0, width, height);
    shell::draw_animated_wallpaper_effects(canvas, state);
    shell::draw_icons(canvas, state, height, unit);

    let focused = state.focused();
    for position in 0..state.order.len() {
        let index = state.order[position];
        let slot = &state.apps[index];
        if !slot.window.open || slot.window.minimized {
            continue;
        }
        let area = state.window_rect(index, unit, width, height);
        canvas.reset_clip();
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
        if slot.window.open_anim >= 140 {
            let client_rect = content_area(area, unit);
            let clip_area = Rect::new(
                client_rect.x + 1,
                client_rect.y + 1,
                (client_rect.width - 2).max(0),
                (client_rect.height - 2).max(0),
            );
            canvas.set_clip(clip_area);
            slot.app.draw(canvas, client_rect, &ctx);
            canvas.reset_clip();
        }
    }

    shell::draw_task_manager(canvas, state, unit);
    shell::draw_launch_toast(canvas, state, width, height, unit);
    shell::draw_floating_dock(canvas, state, width, height, unit);
    if state.start_anim > 0 {
        shell::draw_serpentine_start_menu(canvas, state, width, height, unit);
    }
    if let Some((pos, kind)) = state.context_menu {
        shell::draw_context_menu(canvas, state, pos, kind, width, height, unit);
    }
    if !matches!(state.modal, DesktopModal::None) {
        shell::draw_desktop_modal(canvas, state, width, height, unit);
    }
}

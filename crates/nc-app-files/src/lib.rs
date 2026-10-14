#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use nc_app::text::Line;
use nc_app::{App, AppInfo, Context, Icon, Input, KeyCode, Update};
use nc_app_settings::SettingsApp;
use nc_app_stopwatch::StopwatchApp;
use nc_app_system::SystemApp;
use nc_font::{text_width, Style};
use nc_fs::{DirEntry, FileType, VfsManager};
use nc_gfx::{lerp_color, rgb, Canvas, Rect};
use nc_pkg::NtzcPackage;
use nc_ui::{draw_dot, draw_shadow, draw_text_color, text_style};

const MAX_BLUR_SPAN: usize = 2048;
const VISIBLE_ROWS: usize = 7;

const PORCELAIN_BG: u32 = rgb(245, 250, 253);
const SIDEBAR_TOP: u32 = rgb(238, 246, 251);
const SIDEBAR_BOT: u32 = rgb(228, 240, 247);
const SURFACE_WHITE: u32 = rgb(253, 254, 255);
const SURFACE_ALT: u32 = rgb(242, 248, 252);
const SURFACE_HOVER: u32 = rgb(231, 243, 251);
const SURFACE_SELECT: u32 = rgb(219, 239, 250);
const BORDER_SOFT: u32 = rgb(210, 225, 236);

const AZURE_BRIGHT: u32 = rgb(46, 140, 244);
const AZURE_DEEP: u32 = rgb(24, 102, 212);
const AQUA_MINT: u32 = rgb(28, 190, 170);
const AQUA_SOFT: u32 = rgb(156, 236, 224);
const AQUA_DEEP: u32 = rgb(16, 146, 132);
const DANGER_RED: u32 = rgb(228, 62, 58);

const INK_PRIMARY: u32 = rgb(18, 30, 46);
const INK_SECONDARY: u32 = rgb(62, 80, 100);
const INK_MUTED: u32 = rgb(118, 136, 156);
const INK_DIM: u32 = rgb(158, 174, 192);

enum ModalState {
    None,
    Install(NtzcPackage),
    ViewText {
        path: String,
        content: String,
        scroll_line: usize,
        modified: bool,
    },
    CreateFolder {
        name_buf: String,
    },
    CreateFile {
        name_buf: String,
    },
    DeleteConfirm {
        name: String,
        is_dir: bool,
    },
}

pub struct FilesApp {
    info: AppInfo,
    vfs: VfsManager,
    current_path: String,
    items: Vec<DirEntry>,
    scroll_offset: usize,
    selected_index: Option<usize>,
    hover_index: Option<usize>,
    hover_sidebar: Option<usize>,
    hover_back: bool,
    hover_up_scroll: bool,
    hover_dn_scroll: bool,
    hover_new_folder: bool,
    hover_new_file: bool,
    hover_delete: bool,
    status_banner: String,
    modal: ModalState,
    modal_hover_primary: bool,
    modal_hover_secondary: bool,
}

impl FilesApp {
    pub fn new() -> FilesApp {
        let mut vfs = VfsManager::new();
        let initial_path = String::from("/ram/documents");
        let items = vfs.list_dir_path(&initial_path).unwrap_or_else(|_| Vec::new());
        FilesApp {
            info: AppInfo::new("nc.files", "Dosyalar", "2.4.0", Icon::Generic, 46, 29),
            vfs,
            current_path: initial_path,
            items,
            scroll_offset: 0,
            selected_index: None,
            hover_index: None,
            hover_sidebar: None,
            hover_back: false,
            hover_up_scroll: false,
            hover_dn_scroll: false,
            hover_new_folder: false,
            hover_new_file: false,
            hover_delete: false,
            status_banner: String::from("Hazir - /ram bolumu okuma ve yazma icin aktif"),
            modal: ModalState::None,
            modal_hover_primary: false,
            modal_hover_secondary: false,
        }
    }

    fn is_writable_current(&self) -> bool {
        self.current_path == "/ram" || self.current_path.starts_with("/ram/")
    }

    fn refresh(&mut self) {
        self.items = self.vfs.list_dir_path(&self.current_path).unwrap_or_else(|_| Vec::new());
        self.hover_index = None;
        if let Some(sel) = self.selected_index {
            if sel >= self.items.len() {
                self.selected_index = None;
            }
        }
        if self.scroll_offset > self.items.len().saturating_sub(VISIBLE_ROWS) {
            self.scroll_offset = self.items.len().saturating_sub(VISIBLE_ROWS);
        }
    }

    fn enter_dir(&mut self, name: &str) {
        if self.current_path == "/" {
            self.current_path = alloc::format!("/{}", name);
        } else {
            self.current_path.push('/');
            self.current_path.push_str(name);
        }
        self.scroll_offset = 0;
        self.selected_index = None;
        self.refresh();
        self.status_banner = alloc::format!("Konum acildi: {}", self.current_path);
    }

    fn up_dir(&mut self) {
        if self.current_path == "/" || self.current_path.is_empty() {
            return;
        }
        if let Some(pos) = self.current_path.rfind('/') {
            if pos == 0 {
                self.current_path = String::from("/");
            } else {
                self.current_path.truncate(pos);
            }
        } else {
            self.current_path = String::from("/");
        }
        self.scroll_offset = 0;
        self.selected_index = None;
        self.refresh();
        self.status_banner = alloc::format!("Ust dizine gecildi: {}", self.current_path);
    }

    fn navigate_to(&mut self, target_path: &str) {
        self.current_path = String::from(target_path);
        self.scroll_offset = 0;
        self.selected_index = None;
        self.refresh();
        self.status_banner = alloc::format!("Bolum secildi: {}", self.current_path);
    }

    fn open_selected_entry(&mut self, idx: usize) {
        if idx >= self.items.len() {
            return;
        }
        let entry = self.items[idx].clone();
        if entry.file_type == FileType::Directory {
            self.enter_dir(&entry.name);
            return;
        }
        let full_path = if self.current_path == "/" {
            alloc::format!("/{}", entry.name)
        } else {
            alloc::format!("{}/{}", self.current_path, entry.name)
        };
        if entry.name.ends_with(".ntzc") {
            if let Ok(bytes) = self.vfs.read_file_path(&full_path) {
                if let Some(pkg) = NtzcPackage::parse(&bytes) {
                    self.modal = ModalState::Install(pkg);
                    self.modal_hover_primary = false;
                    self.modal_hover_secondary = false;
                } else {
                    self.status_banner = String::from("Paket basligi bozuk veya gecersiz (.ntzc)");
                }
            }
        } else if let Ok(bytes) = self.vfs.read_file_path(&full_path) {
            let text_str = String::from_utf8_lossy(&bytes).into_owned();
            self.modal = ModalState::ViewText {
                path: full_path,
                content: text_str,
                scroll_line: 0,
                modified: false,
            };
            self.modal_hover_primary = false;
            self.modal_hover_secondary = false;
        }
    }

    fn format_size(bytes: u64, buf: &mut [u8]) -> usize {
        if bytes == 0 {
            buf[0] = b'0';
            buf[1] = b' ';
            buf[2] = b'B';
            return 3;
        }
        let (val, unit): (u64, &[u8]) = if bytes >= 1024 * 1024 {
            (bytes / (1024 * 1024), b" MB" as &[u8])
        } else if bytes >= 1024 {
            (bytes / 1024, b" KB" as &[u8])
        } else {
            (bytes, b" B" as &[u8])
        };
        let mut temp = [0u8; 16];
        let mut tlen = 0;
        let mut n = val;
        while n > 0 {
            temp[tlen] = b'0' + (n % 10) as u8;
            n /= 10;
            tlen += 1;
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
        len
    }

    pub fn create_app_instance(pkg: &NtzcPackage) -> Box<dyn App> {
        match pkg.manifest.id.as_str() {
            "nc.settings" => Box::new(SettingsApp::new()),
            "nc.system" => Box::new(SystemApp::new()),
            "nc.stopwatch" => Box::new(StopwatchApp::new()),
            _ => Box::new(GenericInstalledApp::new(pkg)),
        }
    }

    fn sidebar_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit / 2, area.y + unit / 2, unit * 12, area.height - unit)
    }

    fn sidebar_item_rect(area: Rect, unit: i32, idx: usize) -> Rect {
        let side = Self::sidebar_rect(area, unit);
        let y = side.y + unit * 5 / 2 + idx as i32 * (unit * 12 / 5);
        Rect::new(side.x + unit / 2, y, side.width - unit, unit * 2 + unit / 6)
    }

    fn main_x(area: Rect, unit: i32) -> i32 {
        area.x + unit * 13 + unit / 4
    }

    fn main_w(area: Rect, unit: i32) -> i32 {
        area.width - unit * 13 - unit * 3 / 4
    }

    fn back_btn_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(Self::main_x(area, unit), area.y + unit * 3 / 4, unit * 4 + unit / 2, unit * 2)
    }

    fn new_folder_btn_rect(area: Rect, unit: i32) -> Rect {
        let right = Self::main_x(area, unit) + Self::main_w(area, unit);
        Rect::new(right - unit * 19, area.y + unit * 3 / 4, unit * 6 + unit / 2, unit * 2)
    }

    fn new_file_btn_rect(area: Rect, unit: i32) -> Rect {
        let right = Self::main_x(area, unit) + Self::main_w(area, unit);
        Rect::new(right - unit * 12, area.y + unit * 3 / 4, unit * 6, unit * 2)
    }

    fn delete_btn_rect(area: Rect, unit: i32) -> Rect {
        let right = Self::main_x(area, unit) + Self::main_w(area, unit);
        Rect::new(right - unit * 5 - unit / 2, area.y + unit * 3 / 4, unit * 5 + unit / 2, unit * 2)
    }

    fn path_box_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(
            Self::main_x(area, unit),
            area.y + unit * 3 + unit / 4,
            Self::main_w(area, unit),
            unit * 2,
        )
    }

    fn list_card_rect(area: Rect, unit: i32) -> Rect {
        let y = area.y + unit * 5 + unit * 3 / 4;
        let h = area.height - (y - area.y) - unit * 2 - unit / 4;
        Rect::new(Self::main_x(area, unit), y, Self::main_w(area, unit), h)
    }

    fn file_row_rect(area: Rect, unit: i32, visible_slot: usize) -> Rect {
        let card = Self::list_card_rect(area, unit);
        let top = card.y + unit * 2 + visible_slot as i32 * (unit * 2 + unit / 3);
        Rect::new(card.x + unit / 2, top, card.width - unit * 2, unit * 2 + unit / 6)
    }

    fn scroll_up_rect(area: Rect, unit: i32) -> Rect {
        let card = Self::list_card_rect(area, unit);
        Rect::new(card.x + card.width - unit * 5 / 4, card.y + unit * 2, unit, unit * 3 / 2)
    }

    fn scroll_dn_rect(area: Rect, unit: i32) -> Rect {
        let card = Self::list_card_rect(area, unit);
        Rect::new(
            card.x + card.width - unit * 5 / 4,
            card.y + card.height - unit * 2,
            unit,
            unit * 3 / 2,
        )
    }

    fn modal_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(
            area.x + unit * 4,
            area.y + unit * 2,
            area.width - unit * 8,
            area.height - unit * 4,
        )
    }
}

impl App for FilesApp {
    fn info(&self) -> &AppInfo {
        &self.info
    }

    fn update(&mut self, ctx: &mut Context, input: &Input) -> Update {
        let unit = ctx.unit;
        let mut redraw = false;

        let mut close_modal = false;
        let mut action_save_text: Option<(String, String)> = None;
        let mut action_create_dir: Option<String> = None;
        let mut action_create_file: Option<String> = None;
        let mut action_delete: Option<String> = None;
        let mut action_install: Option<Box<dyn App>> = None;

        let writable_base = if self.is_writable_current() {
            self.current_path.clone()
        } else {
            String::from("/ram")
        };
        let current_path_snapshot = self.current_path.clone();

        match self.modal {
            ModalState::Install(ref pkg) => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 8;
                let btn_h = unit * 2 + unit / 3;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let inst_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let canc_btn = Rect::new(inst_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    if key == KeyCode::Escape {
                        close_modal = true;
                    } else if key == KeyCode::Enter {
                        action_install = Some(Self::create_app_instance(pkg));
                        close_modal = true;
                    }
                }

                if let Some(pointer) = input.pointer {
                    let h_inst = inside(inst_btn, pointer.x, pointer.y);
                    let h_canc = inside(canc_btn, pointer.x, pointer.y);

                    if h_inst != self.modal_hover_primary || h_canc != self.modal_hover_secondary {
                        self.modal_hover_primary = h_inst;
                        self.modal_hover_secondary = h_canc;
                        redraw = true;
                    }

                    if pointer.clicked {
                        if h_inst {
                            action_install = Some(Self::create_app_instance(pkg));
                            close_modal = true;
                        } else if h_canc {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::ViewText {
                ref path,
                ref mut content,
                ref mut scroll_line,
                ref mut modified,
            } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let save_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let close_btn = Rect::new(save_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);
                let can_edit = path.starts_with("/ram");

                if let Some(key) = input.key {
                    match key {
                        KeyCode::Escape => {
                            close_modal = true;
                        }
                        KeyCode::Up => {
                            if *scroll_line > 0 {
                                *scroll_line -= 1;
                                redraw = true;
                            }
                        }
                        KeyCode::Down => {
                            let line_count = content.lines().count().max(1);
                            if *scroll_line + 4 < line_count {
                                *scroll_line += 1;
                                redraw = true;
                            }
                        }
                        KeyCode::Enter if can_edit => {
                            content.push('\n');
                            *modified = true;
                            redraw = true;
                        }
                        KeyCode::Space if can_edit => {
                            content.push(' ');
                            *modified = true;
                            redraw = true;
                        }
                        KeyCode::Backspace if can_edit => {
                            content.pop();
                            *modified = true;
                            redraw = true;
                        }
                        KeyCode::Char(c) if can_edit => {
                            content.push(c);
                            *modified = true;
                            redraw = true;
                        }
                        _ => {}
                    }
                }

                if let Some(pointer) = input.pointer {
                    let h_save = can_edit && inside(save_btn, pointer.x, pointer.y);
                    let h_close = inside(close_btn, pointer.x, pointer.y);

                    if h_save != self.modal_hover_primary || h_close != self.modal_hover_secondary {
                        self.modal_hover_primary = h_save;
                        self.modal_hover_secondary = h_close;
                        redraw = true;
                    }

                    if pointer.clicked {
                        if h_save {
                            action_save_text = Some((path.clone(), content.clone()));
                            close_modal = true;
                        } else if h_close {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::CreateFolder { ref mut name_buf } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    match key {
                        KeyCode::Escape => {
                            close_modal = true;
                        }
                        KeyCode::Enter => {
                            let folder_name = if name_buf.trim().is_empty() {
                                "yeni_klasor"
                            } else {
                                name_buf.trim()
                            };
                            action_create_dir = Some(alloc::format!("{}/{}", writable_base, folder_name));
                            close_modal = true;
                        }
                        KeyCode::Space => {
                            if name_buf.len() < 28 {
                                name_buf.push('_');
                                redraw = true;
                            }
                        }
                        KeyCode::Backspace => {
                            name_buf.pop();
                            redraw = true;
                        }
                        KeyCode::Char(c) => {
                            if c != '/' && c != '\\' && name_buf.len() < 28 {
                                name_buf.push(c);
                                redraw = true;
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(pointer) = input.pointer {
                    let h_create = inside(create_btn, pointer.x, pointer.y);
                    let h_cancel = inside(cancel_btn, pointer.x, pointer.y);

                    if h_create != self.modal_hover_primary || h_cancel != self.modal_hover_secondary {
                        self.modal_hover_primary = h_create;
                        self.modal_hover_secondary = h_cancel;
                        redraw = true;
                    }

                    if pointer.clicked {
                        if h_create {
                            let folder_name = if name_buf.trim().is_empty() {
                                "yeni_klasor"
                            } else {
                                name_buf.trim()
                            };
                            action_create_dir = Some(alloc::format!("{}/{}", writable_base, folder_name));
                            close_modal = true;
                        } else if h_cancel {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::CreateFile { ref mut name_buf } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    match key {
                        KeyCode::Escape => {
                            close_modal = true;
                        }
                        KeyCode::Enter => {
                            let file_name = if name_buf.trim().is_empty() {
                                "notlar.txt"
                            } else {
                                name_buf.trim()
                            };
                            action_create_file = Some(alloc::format!("{}/{}", writable_base, file_name));
                            close_modal = true;
                        }
                        KeyCode::Space => {
                            if name_buf.len() < 28 {
                                name_buf.push('_');
                                redraw = true;
                            }
                        }
                        KeyCode::Backspace => {
                            name_buf.pop();
                            redraw = true;
                        }
                        KeyCode::Char(c) => {
                            if c != '/' && c != '\\' && name_buf.len() < 28 {
                                name_buf.push(c);
                                redraw = true;
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(pointer) = input.pointer {
                    let h_create = inside(create_btn, pointer.x, pointer.y);
                    let h_cancel = inside(cancel_btn, pointer.x, pointer.y);

                    if h_create != self.modal_hover_primary || h_cancel != self.modal_hover_secondary {
                        self.modal_hover_primary = h_create;
                        self.modal_hover_secondary = h_cancel;
                        redraw = true;
                    }

                    if pointer.clicked {
                        if h_create {
                            let file_name = if name_buf.trim().is_empty() {
                                "notlar.txt"
                            } else {
                                name_buf.trim()
                            };
                            action_create_file = Some(alloc::format!("{}/{}", writable_base, file_name));
                            close_modal = true;
                        } else if h_cancel {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::DeleteConfirm { ref name, .. } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let del_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(del_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    if key == KeyCode::Escape {
                        close_modal = true;
                    } else if key == KeyCode::Enter {
                        let target_path = if current_path_snapshot == "/" {
                            alloc::format!("/{}", name)
                        } else {
                            alloc::format!("{}/{}", current_path_snapshot, name)
                        };
                        action_delete = Some(target_path);
                        close_modal = true;
                    }
                }

                if let Some(pointer) = input.pointer {
                    let h_del = inside(del_btn, pointer.x, pointer.y);
                    let h_cancel = inside(cancel_btn, pointer.x, pointer.y);

                    if h_del != self.modal_hover_primary || h_cancel != self.modal_hover_secondary {
                        self.modal_hover_primary = h_del;
                        self.modal_hover_secondary = h_cancel;
                        redraw = true;
                    }

                    if pointer.clicked {
                        if h_del {
                            let target_path = if current_path_snapshot == "/" {
                                alloc::format!("/{}", name)
                            } else {
                                alloc::format!("{}/{}", current_path_snapshot, name)
                            };
                            action_delete = Some(target_path);
                            close_modal = true;
                        } else if h_cancel {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::None => {}
        }

        if let Some((p, c)) = action_save_text {
            if self.vfs.write_file_path(&p, c.as_bytes()).is_ok() {
                self.status_banner = alloc::format!("Kaydedildi: {}", p);
            } else {
                self.status_banner = String::from("Uyari: Bu bolum salt okunurdur (/ram kullanin)");
            }
            self.refresh();
        }
        if let Some(p) = action_create_dir {
            if !self.is_writable_current() {
                self.current_path = String::from("/ram");
            }
            if self.vfs.create_dir_path(&p).is_ok() {
                self.status_banner = alloc::format!("Klasor olusturuldu: {}", p);
            } else {
                self.status_banner = String::from("Klasor olusturulamadi (zaten mevcut olabilir)");
            }
            self.refresh();
        }
        if let Some(p) = action_create_file {
            if !self.is_writable_current() {
                self.current_path = String::from("/ram");
            }
            let default_body = b"NetizenCore metin belgesi.\nBu dosyayi duzenleyip kaydedebilirsiniz.";
            if self.vfs.write_file_path(&p, default_body).is_ok() {
                self.status_banner = alloc::format!("Dosya olusturuldu: {}", p);
            } else {
                self.status_banner = String::from("Dosya yazma hatasi");
            }
            self.refresh();
        }
        if let Some(p) = action_delete {
            if self.vfs.remove_path(&p).is_ok() {
                self.status_banner = alloc::format!("Silindi: {}", p);
            } else {
                self.status_banner = String::from("Salt okunur sistem bolumunden oge silinemez");
            }
            self.selected_index = None;
            self.refresh();
        }

        if close_modal {
            self.modal = ModalState::None;
            redraw = true;
        }

        if let Some(app) = action_install {
            self.status_banner = String::from("Paket masaustune ve baslat menusune kuruldu");
            return Update::install(app);
        }

        if !matches!(self.modal, ModalState::None) {
            return Update {
                redraw,
                close: false,
                install_app: None,
            };
        }

        if let Some(key) = input.key {
            match key {
                KeyCode::Backspace => {
                    self.up_dir();
                    redraw = true;
                }
                KeyCode::Up => {
                    if let Some(sel) = self.selected_index {
                        if sel > 0 {
                            self.selected_index = Some(sel - 1);
                            if sel - 1 < self.scroll_offset {
                                self.scroll_offset = sel - 1;
                            }
                            redraw = true;
                        }
                    } else if !self.items.is_empty() {
                        self.selected_index = Some(0);
                        redraw = true;
                    }
                }
                KeyCode::Down => {
                    if let Some(sel) = self.selected_index {
                        if sel + 1 < self.items.len() {
                            self.selected_index = Some(sel + 1);
                            if sel + 1 >= self.scroll_offset + VISIBLE_ROWS {
                                self.scroll_offset = sel + 2 - VISIBLE_ROWS;
                            }
                            redraw = true;
                        }
                    } else if !self.items.is_empty() {
                        self.selected_index = Some(0);
                        redraw = true;
                    }
                }
                KeyCode::Enter => {
                    if let Some(sel) = self.selected_index {
                        self.open_selected_entry(sel);
                        redraw = true;
                    }
                }
                _ => {}
            }
        }

        let mut next_hover = None;
        let mut next_side_hover = None;
        let mut next_back_hover = false;
        let mut next_up_scroll = false;
        let mut next_dn_scroll = false;
        let mut next_new_folder = false;
        let mut next_new_file = false;
        let mut next_delete = false;

        if let Some(pointer) = input.pointer {
            if self.current_path != "/" {
                next_back_hover = inside(Self::back_btn_rect(input.area, unit), pointer.x, pointer.y);
            }
            next_new_folder = inside(Self::new_folder_btn_rect(input.area, unit), pointer.x, pointer.y);
            next_new_file = inside(Self::new_file_btn_rect(input.area, unit), pointer.x, pointer.y);
            next_delete = self.selected_index.is_some()
                && inside(Self::delete_btn_rect(input.area, unit), pointer.x, pointer.y);
            next_up_scroll = inside(Self::scroll_up_rect(input.area, unit), pointer.x, pointer.y);
            next_dn_scroll = inside(Self::scroll_dn_rect(input.area, unit), pointer.x, pointer.y);

            for i in 0..6 {
                if inside(Self::sidebar_item_rect(input.area, unit, i), pointer.x, pointer.y) {
                    next_side_hover = Some(i);
                    break;
                }
            }

            let visible_count = self.items.len().saturating_sub(self.scroll_offset).min(VISIBLE_ROWS);
            for slot in 0..visible_count {
                if inside(Self::file_row_rect(input.area, unit, slot), pointer.x, pointer.y) {
                    next_hover = Some(self.scroll_offset + slot);
                    break;
                }
            }

            if pointer.clicked {
                if next_back_hover {
                    self.up_dir();
                    redraw = true;
                } else if next_up_scroll && self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                    redraw = true;
                } else if next_dn_scroll && self.scroll_offset + VISIBLE_ROWS < self.items.len() {
                    self.scroll_offset += 1;
                    redraw = true;
                } else if next_new_folder {
                    self.modal = ModalState::CreateFolder {
                        name_buf: String::from("yeni_klasor"),
                    };
                    self.modal_hover_primary = false;
                    self.modal_hover_secondary = false;
                    redraw = true;
                } else if next_new_file {
                    self.modal = ModalState::CreateFile {
                        name_buf: String::from("notlar.txt"),
                    };
                    self.modal_hover_primary = false;
                    self.modal_hover_secondary = false;
                    redraw = true;
                } else if next_delete {
                    if let Some(idx) = self.selected_index {
                        if idx < self.items.len() {
                            let item = &self.items[idx];
                            self.modal = ModalState::DeleteConfirm {
                                name: item.name.clone(),
                                is_dir: item.file_type == FileType::Directory,
                            };
                            self.modal_hover_primary = false;
                            self.modal_hover_secondary = false;
                            redraw = true;
                        }
                    }
                } else if let Some(side_idx) = next_side_hover {
                    let target = match side_idx {
                        0 => "/",
                        1 => "/system",
                        2 => "/system/app",
                        3 => "/data",
                        4 => "/ram",
                        _ => "/ram/documents",
                    };
                    self.navigate_to(target);
                    redraw = true;
                } else if let Some(idx) = next_hover {
                    self.selected_index = Some(idx);
                    self.open_selected_entry(idx);
                    redraw = true;
                }
            }
        }

        if next_hover != self.hover_index
            || next_side_hover != self.hover_sidebar
            || next_back_hover != self.hover_back
            || next_up_scroll != self.hover_up_scroll
            || next_dn_scroll != self.hover_dn_scroll
            || next_new_folder != self.hover_new_folder
            || next_new_file != self.hover_new_file
            || next_delete != self.hover_delete
        {
            self.hover_index = next_hover;
            self.hover_sidebar = next_side_hover;
            self.hover_back = next_back_hover;
            self.hover_up_scroll = next_up_scroll;
            self.hover_dn_scroll = next_dn_scroll;
            self.hover_new_folder = next_new_folder;
            self.hover_new_file = next_new_file;
            self.hover_delete = next_delete;
            redraw = true;
        }

        Update {
            redraw,
            close: false,
            install_app: None,
        }
    }

    fn draw(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context) {
        let unit = ctx.unit;
        canvas.fill_rect(area.x, area.y, area.width, area.height, PORCELAIN_BG);

        let side_rect = Self::sidebar_rect(area, unit);
        draw_rounded_surface(canvas, side_rect, unit / 2, SIDEBAR_TOP, SIDEBAR_BOT, 255, BORDER_SOFT, true);

        let title_style = text_style((unit * 82 / 100).max(11), 16);
        let label_style = text_style((unit * 58 / 100).max(10), 15);
        let cap_style = text_style((unit * 48 / 100).max(9), 18);

        draw_dot(canvas, side_rect.x + unit, side_rect.y + unit * 5 / 4, 5, AQUA_MINT, 255);
        draw_text_color(
            canvas,
            "Depolama Agaci",
            side_rect.x + unit * 7 / 4,
            side_rect.y + unit * 4 / 5,
            &label_style,
            INK_PRIMARY,
        );

        let locations = [
            ("Kok Dizin (/)", "/", AZURE_DEEP),
            ("Sistem (/system)", "/system", AQUA_MINT),
            ("Paketler (/app)", "/system/app", AZURE_BRIGHT),
            ("Veri (/data)", "/data", AQUA_DEEP),
            ("RAM Disk (/ram)", "/ram", AZURE_BRIGHT),
            ("Belgelerim", "/ram/documents", AQUA_MINT),
        ];

        for (i, (name, target, dot_col)) in locations.iter().enumerate() {
            let item_rect = Self::sidebar_item_rect(area, unit, i);
            let selected = self.current_path == *target;
            let hovered = self.hover_sidebar == Some(i);

            if selected {
                draw_rounded_surface(
                    canvas,
                    item_rect,
                    unit / 3,
                    AZURE_BRIGHT,
                    AQUA_MINT,
                    255,
                    AZURE_DEEP,
                    true,
                );
            } else if hovered {
                draw_rounded_surface(
                    canvas,
                    item_rect,
                    unit / 3,
                    SURFACE_HOVER,
                    SURFACE_ALT,
                    255,
                    BORDER_SOFT,
                    false,
                );
            }

            let dot_c = if selected { rgb(255, 255, 255) } else { *dot_col };
            draw_dot(canvas, item_rect.x + unit * 3 / 4, item_rect.y + item_rect.height / 2, 4, dot_c, 255);

            let fg = if selected { rgb(255, 255, 255) } else { INK_PRIMARY };
            let ty = item_rect.y + (item_rect.height - label_style.cap_height) / 2;
            draw_text_color(canvas, name, item_rect.x + unit * 3 / 2, ty, &label_style, fg);
        }

        let meter_card = Rect::new(
            side_rect.x + unit / 2,
            side_rect.y + side_rect.height - unit * 5,
            side_rect.width - unit,
            unit * 9 / 2,
        );
        draw_rounded_surface(
            canvas,
            meter_card,
            unit / 3,
            SURFACE_WHITE,
            SURFACE_ALT,
            245,
            BORDER_SOFT,
            true,
        );
        let mount_tag = if self.current_path.starts_with("/system") {
            "EROFS Salt Okunur"
        } else if self.current_path.starts_with("/data") {
            "exFAT Birincil"
        } else if self.current_path.starts_with("/ram") {
            "MemFS Yazilabilir"
        } else {
            "VFS Sanal Kok"
        };
        draw_text_color(
            canvas,
            mount_tag,
            meter_card.x + unit * 2 / 3,
            meter_card.y + unit * 2 / 3,
            &cap_style,
            AZURE_DEEP,
        );

        let bar_r = Rect::new(
            meter_card.x + unit * 2 / 3,
            meter_card.y + unit * 2,
            meter_card.width - unit * 4 / 3,
            unit / 2,
        );
        draw_rounded_surface(canvas, bar_r, 3, SURFACE_ALT, BORDER_SOFT, 255, BORDER_SOFT, false);
        let used_w = (bar_r.width * 42 / 100).max(6);
        let fill_r = Rect::new(bar_r.x, bar_r.y, used_w, bar_r.height);
        draw_rounded_surface(canvas, fill_r, 3, AZURE_BRIGHT, AQUA_MINT, 255, AQUA_DEEP, false);

        let mut count_line = Line::new();
        count_line.number(self.items.len() as u32, 1).text(" oge listeleniyor");
        draw_text_color(
            canvas,
            count_line.as_str(),
            meter_card.x + unit * 2 / 3,
            meter_card.y + unit * 3,
            &cap_style,
            INK_MUTED,
        );

        let back_rect = Self::back_btn_rect(area, unit);
        draw_aero_pill_button(
            canvas,
            back_rect,
            "< Ust",
            self.hover_back,
            false,
            self.current_path != "/",
        );

        draw_aero_pill_button(
            canvas,
            Self::new_folder_btn_rect(area, unit),
            "+ Klasor",
            self.hover_new_folder,
            false,
            true,
        );
        draw_aero_pill_button(
            canvas,
            Self::new_file_btn_rect(area, unit),
            "+ Belge",
            self.hover_new_file,
            true,
            true,
        );
        draw_aero_pill_button(
            canvas,
            Self::delete_btn_rect(area, unit),
            "Sil",
            self.hover_delete,
            false,
            self.selected_index.is_some(),
        );

        let path_box = Self::path_box_rect(area, unit);
        draw_rounded_surface(
            canvas,
            path_box,
            unit / 3,
            SURFACE_WHITE,
            SURFACE_ALT,
            255,
            BORDER_SOFT,
            true,
        );
        draw_dot(
            canvas,
            path_box.x + unit * 3 / 4,
            path_box.y + path_box.height / 2,
            4,
            AZURE_BRIGHT,
            255,
        );
        let path_ty = path_box.y + (path_box.height - label_style.cap_height) / 2;
        draw_text_color(
            canvas,
            &self.current_path,
            path_box.x + unit * 3 / 2,
            path_ty,
            &label_style,
            INK_PRIMARY,
        );

        let mode_badge = Rect::new(
            path_box.x + path_box.width - unit * 6 - unit / 2,
            path_box.y + unit / 4,
            unit * 6,
            path_box.height - unit / 2,
        );
        let (m_text, m_top, m_bot, m_fg) = if self.is_writable_current() {
            ("YAZILABILIR", SURFACE_SELECT, rgb(208, 242, 236), AQUA_DEEP)
        } else {
            ("KORUMALI", SURFACE_ALT, BORDER_SOFT, INK_MUTED)
        };
        draw_rounded_surface(canvas, mode_badge, mode_badge.height / 2, m_top, m_bot, 255, BORDER_SOFT, false);
        let m_tw = text_width(m_text, &cap_style);
        draw_text_color(
            canvas,
            m_text,
            mode_badge.x + (mode_badge.width - m_tw) / 2,
            mode_badge.y + (mode_badge.height - cap_style.cap_height) / 2,
            &cap_style,
            m_fg,
        );

        let list_card = Self::list_card_rect(area, unit);
        draw_rounded_surface(
            canvas,
            list_card,
            unit / 2,
            SURFACE_WHITE,
            rgb(248, 252, 255),
            255,
            BORDER_SOFT,
            true,
        );

        let hdr_y = list_card.y + unit / 2;
        draw_text_color(canvas, "Ad", list_card.x + unit * 3, hdr_y, &cap_style, INK_MUTED);
        draw_text_color(
            canvas,
            "Tur",
            list_card.x + list_card.width - unit * 13,
            hdr_y,
            &cap_style,
            INK_MUTED,
        );
        draw_text_color(
            canvas,
            "Boyut",
            list_card.x + list_card.width - unit * 6,
            hdr_y,
            &cap_style,
            INK_MUTED,
        );
        canvas.fill_rect(
            list_card.x + unit / 2,
            list_card.y + unit * 3 / 2,
            list_card.width - unit,
            1,
            BORDER_SOFT,
        );

        if self.items.is_empty() {
            draw_text_color(
                canvas,
                "Bu klasor su anda bos. '+ Belge' veya '+ Klasor' ile yeni oge ekleyebilirsiniz.",
                list_card.x + unit * 3 / 2,
                list_card.y + unit * 4,
                &label_style,
                INK_MUTED,
            );
        }

        let visible_count = self.items.len().saturating_sub(self.scroll_offset).min(VISIBLE_ROWS);
        for slot in 0..visible_count {
            let idx = self.scroll_offset + slot;
            let item = &self.items[idx];
            let row = Self::file_row_rect(area, unit, slot);
            let is_selected = self.selected_index == Some(idx);
            let is_hov = self.hover_index == Some(idx);

            let (r_top, r_bot, r_brd) = if is_selected {
                (SURFACE_SELECT, rgb(206, 234, 248), AZURE_BRIGHT)
            } else if is_hov {
                (SURFACE_HOVER, SURFACE_ALT, rgb(180, 212, 232))
            } else if (slot & 1) == 1 {
                (SURFACE_ALT, SURFACE_ALT, BORDER_SOFT)
            } else {
                (SURFACE_WHITE, SURFACE_WHITE, BORDER_SOFT)
            };
            draw_rounded_surface(canvas, row, unit / 3, r_top, r_bot, 255, r_brd, false);

            let is_dir = item.file_type == FileType::Directory;
            let is_pkg = item.name.ends_with(".ntzc");
            let is_txt = item.name.ends_with(".txt") || item.name.ends_with(".prop") || item.name.ends_with(".cfg");

            draw_entry_icon(canvas, row.x + unit / 2, row.y + (row.height - 16) / 2, is_dir, is_pkg, is_txt);

            let text_y = row.y + (row.height - label_style.cap_height) / 2;
            draw_text_color(
                canvas,
                &item.name,
                row.x + unit * 2 + unit / 4,
                text_y,
                &label_style,
                INK_PRIMARY,
            );

            let (kind_str, kind_col) = if is_dir {
                ("Klasor", AZURE_DEEP)
            } else if is_pkg {
                ("Uygulama (.ntzc)", AQUA_DEEP)
            } else if is_txt {
                ("Metin Belgesi", INK_SECONDARY)
            } else {
                ("Sistem Dosyasi", INK_MUTED)
            };
            draw_text_color(
                canvas,
                kind_str,
                list_card.x + list_card.width - unit * 13,
                text_y,
                &cap_style,
                kind_col,
            );

            if !is_dir {
                let mut sz_buf = [0u8; 16];
                let sz_len = Self::format_size(item.size, &mut sz_buf);
                if let Ok(st) = core::str::from_utf8(&sz_buf[..sz_len]) {
                    draw_text_color(
                        canvas,
                        st,
                        list_card.x + list_card.width - unit * 6,
                        text_y,
                        &cap_style,
                        INK_SECONDARY,
                    );
                }
            } else {
                draw_text_color(
                    canvas,
                    "--",
                    list_card.x + list_card.width - unit * 6,
                    text_y,
                    &cap_style,
                    INK_DIM,
                );
            }
        }

        let up_btn = Self::scroll_up_rect(area, unit);
        let dn_btn = Self::scroll_dn_rect(area, unit);
        draw_aero_pill_button(canvas, up_btn, "^", self.hover_up_scroll, false, self.scroll_offset > 0);
        draw_aero_pill_button(
            canvas,
            dn_btn,
            "v",
            self.hover_dn_scroll,
            false,
            self.scroll_offset + VISIBLE_ROWS < self.items.len(),
        );

        let status_y = area.y + area.height - unit * 3 / 2;
        draw_dot(canvas, Self::main_x(area, unit) + 6, status_y + 5, 4, AQUA_MINT, 255);
        draw_text_color(
            canvas,
            &self.status_banner,
            Self::main_x(area, unit) + 18,
            status_y,
            &cap_style,
            INK_SECONDARY,
        );

        match self.modal {
            ModalState::Install(ref pkg) => {
                let m_rect = Self::modal_rect(area, unit);
                draw_frosted_modal(canvas, m_rect, unit);

                draw_entry_icon(canvas, m_rect.x + unit * 2, m_rect.y + unit * 3 / 2, false, true, false);
                let mut modal_title = Line::new();
                modal_title.text("Uygulama Paketini Kur: ").text(&pkg.manifest.name);
                draw_text_color(
                    canvas,
                    modal_title.as_str(),
                    m_rect.x + unit * 7 / 2,
                    m_rect.y + unit * 3 / 2,
                    &title_style,
                    INK_PRIMARY,
                );

                let role_pill = Rect::new(m_rect.x + unit * 2, m_rect.y + unit * 7 / 2, unit * 12, unit * 3 / 2);
                draw_rounded_surface(
                    canvas,
                    role_pill,
                    role_pill.height / 2,
                    SURFACE_SELECT,
                    rgb(212, 242, 236),
                    255,
                    AQUA_MINT,
                    false,
                );
                draw_text_color(
                    canvas,
                    pkg.role_label(),
                    role_pill.x + unit / 2,
                    role_pill.y + (role_pill.height - cap_style.cap_height) / 2,
                    &cap_style,
                    AQUA_DEEP,
                );

                let mut ver_line = Line::new();
                ver_line
                    .text("Kimlik: ")
                    .text(&pkg.manifest.id)
                    .text("   |   Surum: ")
                    .text(&pkg.manifest.version);
                draw_text_color(
                    canvas,
                    ver_line.as_str(),
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 11 / 2,
                    &cap_style,
                    INK_MUTED,
                );

                draw_text_color(
                    canvas,
                    "Bu paketin talep ettigi sistem izinleri:",
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 7,
                    &label_style,
                    INK_PRIMARY,
                );
                let labels = pkg.permission_labels();
                for (idx, lbl) in labels.iter().enumerate() {
                    let iy = m_rect.y + unit * 9 + idx as i32 * (unit * 3 / 2);
                    draw_dot(canvas, m_rect.x + unit * 5 / 2, iy + 5, 4, AZURE_BRIGHT, 255);
                    draw_text_color(canvas, lbl, m_rect.x + unit * 7 / 2, iy, &label_style, INK_SECONDARY);
                }

                let btn_w = unit * 8;
                let btn_h = unit * 2 + unit / 3;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let inst_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let canc_btn = Rect::new(inst_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_aero_pill_button(canvas, canc_btn, "Vazgec", self.modal_hover_secondary, false, true);
                draw_aero_pill_button(canvas, inst_btn, "Sisteme Kur", self.modal_hover_primary, true, true);
            }
            ModalState::ViewText {
                ref path,
                ref content,
                scroll_line,
                modified,
            } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_frosted_modal(canvas, m_rect, unit);

                let can_edit = path.starts_with("/ram");
                let title = alloc::format!("Belge Duzenleyici: {}", path);
                draw_text_color(
                    canvas,
                    &title,
                    m_rect.x + unit * 3 / 2,
                    m_rect.y + unit * 4 / 3,
                    &title_style,
                    INK_PRIMARY,
                );

                let status_msg = if !can_edit {
                    "Salt Okunur Sistem Dosyasi"
                } else if modified {
                    "Degistirildi - Kaydetmeye Hazir"
                } else {
                    "Klavye ile dogrudan yazabilirsiniz"
                };
                draw_text_color(
                    canvas,
                    status_msg,
                    m_rect.x + unit * 3 / 2,
                    m_rect.y + unit * 3,
                    &cap_style,
                    if modified { AQUA_DEEP } else { INK_MUTED },
                );

                let editor_box = Rect::new(
                    m_rect.x + unit * 3 / 2,
                    m_rect.y + unit * 9 / 2,
                    m_rect.width - unit * 3,
                    m_rect.height - unit * 17 / 2,
                );
                draw_multiline_editor(canvas, editor_box, content, scroll_line, can_edit, &label_style, &cap_style);

                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let save_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let close_btn = Rect::new(save_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_aero_pill_button(canvas, close_btn, "Kapat", self.modal_hover_secondary, false, true);
                draw_aero_pill_button(
                    canvas,
                    save_btn,
                    "Kaydet",
                    self.modal_hover_primary,
                    true,
                    can_edit,
                );
            }
            ModalState::CreateFolder { ref name_buf } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_frosted_modal(canvas, m_rect, unit);

                draw_text_color(
                    canvas,
                    "Yeni Klasor Olustur",
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 2,
                    &title_style,
                    INK_PRIMARY,
                );
                draw_text_color(
                    canvas,
                    "Klasor adini klavyeyle yazin ve Olustur dugmesine tiklayin:",
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 4,
                    &cap_style,
                    INK_MUTED,
                );

                let input_box = Rect::new(
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 6,
                    m_rect.width - unit * 4,
                    unit * 5 / 2,
                );
                draw_single_line_input(canvas, input_box, name_buf, &label_style);

                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_aero_pill_button(canvas, cancel_btn, "Iptal", self.modal_hover_secondary, false, true);
                draw_aero_pill_button(canvas, create_btn, "Olustur", self.modal_hover_primary, true, true);
            }
            ModalState::CreateFile { ref name_buf } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_frosted_modal(canvas, m_rect, unit);

                draw_text_color(
                    canvas,
                    "Yeni Metin Belgesi Olustur",
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 2,
                    &title_style,
                    INK_PRIMARY,
                );
                draw_text_color(
                    canvas,
                    "Dosya adini klavyeyle yazin (ornegin: notlar.txt):",
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 4,
                    &cap_style,
                    INK_MUTED,
                );

                let input_box = Rect::new(
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 6,
                    m_rect.width - unit * 4,
                    unit * 5 / 2,
                );
                draw_single_line_input(canvas, input_box, name_buf, &label_style);

                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_aero_pill_button(canvas, cancel_btn, "Iptal", self.modal_hover_secondary, false, true);
                draw_aero_pill_button(canvas, create_btn, "Olustur", self.modal_hover_primary, true, true);
            }
            ModalState::DeleteConfirm { ref name, is_dir } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_frosted_modal(canvas, m_rect, unit);

                let type_str = if is_dir { "klasoru" } else { "dosyasi" };
                let msg = alloc::format!("'{}' {} kalici olarak silinsin mi?", name, type_str);
                draw_text_color(
                    canvas,
                    "Silme Onayi",
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 2,
                    &title_style,
                    DANGER_RED,
                );
                draw_text_color(
                    canvas,
                    &msg,
                    m_rect.x + unit * 2,
                    m_rect.y + unit * 5,
                    &label_style,
                    INK_PRIMARY,
                );

                let btn_w = unit * 7 + unit / 2;
                let btn_h = unit * 2 + unit / 4;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let del_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(del_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_aero_pill_button(canvas, cancel_btn, "Vazgec", self.modal_hover_secondary, false, true);
                draw_aero_pill_button(canvas, del_btn, "Evet, Sil", self.modal_hover_primary, true, true);
            }
            ModalState::None => {}
        }
    }
}

pub struct GenericInstalledApp {
    info: AppInfo,
    uid: u32,
    role: &'static str,
    state_counter: i32,
    accumulator: i32,
    status_text: String,
    script_source: String,
    hover_btn1: bool,
    hover_btn2: bool,
    hover_btn3: bool,
}

impl GenericInstalledApp {
    pub fn new(pkg: &NtzcPackage) -> Self {
        let script = core::str::from_utf8(&pkg.payload)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(String::from)
            .unwrap_or_else(|| {
                String::from(
                    "fun onAction1() { state = state + 1; status = \"Moduler calisma zamani: Sayac arttirildi\" }\nfun onAction2() { state = state * 2; status = \"Moduler calisma zamani: Deger ikiye katlandi\" }\nfun onReset() { state = 0; status = \"Moduler calisma zamani: Bellek sifirlandi\" }",
                )
            });

        GenericInstalledApp {
            info: AppInfo::new(
                &pkg.manifest.id,
                &pkg.manifest.name,
                &pkg.manifest.version,
                pkg.manifest.icon,
                pkg.manifest.width_units,
                pkg.manifest.height_units,
            ),
            uid: pkg.uid,
            role: pkg.role_label(),
            state_counter: 1,
            accumulator: 0,
            status_text: String::from("NetizenCore .ntzc Konteyneri Hazir"),
            script_source: script,
            hover_btn1: false,
            hover_btn2: false,
            hover_btn3: false,
        }
    }

    fn run_handler(&mut self, handler_name: &str) {
        if let Some(pos) = self.script_source.find(handler_name) {
            let after = &self.script_source[pos..];
            if let Some(open_b) = after.find('{') {
                if let Some(close_b) = after[open_b + 1..].find('}') {
                    let body = &after[open_b + 1..open_b + 1 + close_b];
                    for stmt in body.split(';') {
                        let s = stmt.trim();
                        if s.contains("state = state +") {
                            let step = s
                                .split('+')
                                .nth(1)
                                .and_then(|v| v.trim().parse::<i32>().ok())
                                .unwrap_or(1);
                            self.state_counter = self.state_counter.saturating_add(step);
                            self.accumulator = self.accumulator.saturating_add(1);
                        } else if s.contains("state = state *") {
                            let factor = s
                                .split('*')
                                .nth(1)
                                .and_then(|v| v.trim().parse::<i32>().ok())
                                .unwrap_or(2);
                            self.state_counter = self.state_counter.saturating_mul(factor).min(999_999);
                            self.accumulator = self.accumulator.saturating_add(1);
                        } else if s.contains("state = 0") {
                            self.state_counter = 0;
                        } else if let Some((_, msg)) = s.split_once("status =") {
                            self.status_text = String::from(msg.trim().trim_matches('"'));
                        }
                    }
                }
            }
        }
    }

    fn btn1_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit * 3 / 2, area.y + area.height - unit * 7 / 2, unit * 8, unit * 2)
    }

    fn btn2_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit * 10 + unit / 2, area.y + area.height - unit * 7 / 2, unit * 8, unit * 2)
    }

    fn btn3_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit * 19 + unit / 2, area.y + area.height - unit * 7 / 2, unit * 7, unit * 2)
    }
}

impl App for GenericInstalledApp {
    fn info(&self) -> &AppInfo {
        &self.info
    }

    fn update(&mut self, ctx: &mut Context, input: &Input) -> Update {
        let unit = ctx.unit;
        let b1 = Self::btn1_rect(input.area, unit);
        let b2 = Self::btn2_rect(input.area, unit);
        let b3 = Self::btn3_rect(input.area, unit);
        let mut h1 = false;
        let mut h2 = false;
        let mut h3 = false;
        let mut redraw = false;

        if let Some(p) = input.pointer {
            h1 = inside(b1, p.x, p.y);
            h2 = inside(b2, p.x, p.y);
            h3 = inside(b3, p.x, p.y);
            if p.clicked {
                if h1 {
                    self.run_handler("onAction1");
                    redraw = true;
                } else if h2 {
                    self.run_handler("onAction2");
                    redraw = true;
                } else if h3 {
                    self.run_handler("onReset");
                    redraw = true;
                }
            }
        }

        if h1 != self.hover_btn1 || h2 != self.hover_btn2 || h3 != self.hover_btn3 {
            self.hover_btn1 = h1;
            self.hover_btn2 = h2;
            self.hover_btn3 = h3;
            redraw = true;
        }

        Update {
            redraw,
            close: false,
            install_app: None,
        }
    }

    fn draw(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context) {
        let unit = ctx.unit;
        canvas.fill_rect(area.x, area.y, area.width, area.height, PORCELAIN_BG);

        let card = Rect::new(
            area.x + unit * 3 / 2,
            area.y + unit,
            area.width - unit * 3,
            area.height - unit * 11 / 2,
        );
        draw_rounded_surface(
            canvas,
            card,
            unit / 2,
            SURFACE_WHITE,
            SURFACE_ALT,
            255,
            BORDER_SOFT,
            true,
        );

        let title_style = text_style((unit * 85 / 100).max(11), 16);
        let sub_style = text_style((unit * 56 / 100).max(9), 16);
        let big_style = text_style((unit * 16 / 10).max(18), 16);

        draw_dot(canvas, card.x + unit, card.y + unit * 4 / 3, 6, AQUA_MINT, 255);
        draw_text_color(
            canvas,
            &self.info.name,
            card.x + unit * 2,
            card.y + unit,
            &title_style,
            INK_PRIMARY,
        );

        let role_rect = Rect::new(
            card.x + card.width - unit * 10 - unit / 2,
            card.y + unit * 3 / 4,
            unit * 9 + unit / 2,
            unit * 3 / 2,
        );
        draw_rounded_surface(
            canvas,
            role_rect,
            role_rect.height / 2,
            SURFACE_SELECT,
            rgb(212, 242, 236),
            255,
            AQUA_MINT,
            false,
        );
        draw_text_color(
            canvas,
            self.role,
            role_rect.x + unit / 2,
            role_rect.y + (role_rect.height - sub_style.cap_height) / 2,
            &sub_style,
            AQUA_DEEP,
        );

        let mut val_line = Line::new();
        val_line.number(self.state_counter.max(0) as u32, 1);
        draw_text_color(
            canvas,
            val_line.as_str(),
            card.x + unit * 3 / 2,
            card.y + unit * 7 / 2,
            &big_style,
            AZURE_DEEP,
        );

        let mut meta = Line::new();
        meta.text("Paket UID: ")
            .number(self.uid, 1)
            .text("   |   Islem Sayisi: ")
            .number(self.accumulator.max(0) as u32, 1);
        draw_text_color(
            canvas,
            meta.as_str(),
            card.x + unit * 3 / 2,
            card.y + unit * 6,
            &sub_style,
            INK_MUTED,
        );
        draw_text_color(
            canvas,
            &self.status_text,
            card.x + unit * 3 / 2,
            card.y + unit * 15 / 2,
            &sub_style,
            AQUA_DEEP,
        );

        draw_aero_pill_button(
            canvas,
            Self::btn1_rect(area, unit),
            "+1 Arttir",
            self.hover_btn1,
            true,
            true,
        );
        draw_aero_pill_button(
            canvas,
            Self::btn2_rect(area, unit),
            "x2 Katla",
            self.hover_btn2,
            false,
            true,
        );
        draw_aero_pill_button(
            canvas,
            Self::btn3_rect(area, unit),
            "Sifirla",
            self.hover_btn3,
            false,
            true,
        );
    }
}

fn corner_cov(dx: i32, dy: i32, r: i32) -> u8 {
    if r <= 0 {
        return 255;
    }
    let d2 = dx * dx + dy * dy;
    let r2 = r * r;
    let inner = (r - 1).max(0) * (r - 1).max(0);
    if d2 <= inner {
        255
    } else if d2 >= r2 {
        0
    } else {
        ((r2 - d2) * 255 / (r2 - inner).max(1)).clamp(0, 255) as u8
    }
}

fn rounded_cov(x: i32, y: i32, rect: Rect, radius: i32) -> u8 {
    let r = radius.min(rect.width / 2).min(rect.height / 2);
    if r <= 0 {
        return 255;
    }
    let lx = rect.x + r;
    let rx = rect.x + rect.width - 1 - r;
    let ty = rect.y + r;
    let by = rect.y + rect.height - 1 - r;
    let dx = if x < lx {
        lx - x
    } else if x > rx {
        x - rx
    } else {
        0
    };
    let dy = if y < ty {
        ty - y
    } else if y > by {
        y - by
    } else {
        0
    };
    if dx == 0 || dy == 0 {
        255
    } else {
        corner_cov(dx, dy, r)
    }
}

fn draw_rounded_surface(
    canvas: &mut Canvas<'_>,
    rect: Rect,
    radius: i32,
    top_col: u32,
    bot_col: u32,
    alpha: u8,
    border_col: u32,
    top_glint: bool,
) {
    if rect.width <= 2 || rect.height <= 2 {
        return;
    }
    let r = radius.min(rect.width / 2).min(rect.height / 2).max(1);
    for y in rect.y..(rect.y + rect.height) {
        let ry = y - rect.y;
        let t = (ry * 255 / rect.height.max(1)) as u32;
        let fill = lerp_color(top_col, bot_col, t);
        let deep_row = ry >= r && ry < rect.height - r;

        if deep_row && alpha == 255 {
            canvas.fill_rect(rect.x + 1, y, rect.width - 2, 1, fill);
            canvas.fill_rect(rect.x, y, 1, 1, border_col);
            canvas.fill_rect(rect.x + rect.width - 1, y, 1, 1, border_col);
            continue;
        }

        for x in rect.x..(rect.x + rect.width) {
            let cov = rounded_cov(x, y, rect, r);
            if cov == 0 {
                continue;
            }
            let eff_a = (cov as u32 * alpha as u32 / 255) as u8;
            canvas.blend_pixel(x, y, fill, eff_a);

            let on_edge = x == rect.x
                || x == rect.x + rect.width - 1
                || y == rect.y
                || y == rect.y + rect.height - 1
                || cov < 235;
            if on_edge {
                canvas.blend_pixel(x, y, border_col, cov);
            } else if top_glint && ry == 1 && x > rect.x + r / 2 && x < rect.x + rect.width - r / 2 {
                canvas.blend_pixel(x, y, rgb(255, 255, 255), 210);
            }
        }
    }
}

fn blur_rect_in_place(canvas: &mut Canvas<'_>, rect: Rect, radius: usize) {
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let x0 = rect.x.clamp(0, cw);
    let y0 = rect.y.clamp(0, ch);
    let x1 = (rect.x + rect.width).clamp(0, cw);
    let y1 = (rect.y + rect.height).clamp(0, ch);
    let rw = (x1 - x0) as usize;
    let rh = (y1 - y0) as usize;
    if rw == 0 || rh == 0 || radius == 0 || rw > MAX_BLUR_SPAN || rh > MAX_BLUR_SPAN {
        return;
    }

    let stride = cw as usize;
    let pixels = canvas.pixels_mut();
    let mut scratch = [0u32; MAX_BLUR_SPAN];
    let r = radius.min(16);
    let win = (r * 2 + 1) as u32;

    for _ in 0..2 {
        for y in (y0 as usize)..(y1 as usize) {
            let base = y * stride + (x0 as usize);
            for i in 0..rw {
                scratch[i] = pixels[base + i];
            }
            let first = scratch[0];
            let mut sr = ((first >> 16) & 0xFF) * (r as u32 + 1);
            let mut sg = ((first >> 8) & 0xFF) * (r as u32 + 1);
            let mut sb = (first & 0xFF) * (r as u32 + 1);
            for i in 1..=r {
                let p = scratch[i.min(rw - 1)];
                sr += (p >> 16) & 0xFF;
                sg += (p >> 8) & 0xFF;
                sb += p & 0xFF;
            }
            for i in 0..rw {
                pixels[base + i] = ((sr / win) << 16) | ((sg / win) << 8) | (sb / win);
                let out_p = scratch[i.saturating_sub(r)];
                let in_p = scratch[(i + r + 1).min(rw - 1)];
                sr = sr + ((in_p >> 16) & 0xFF) - ((out_p >> 16) & 0xFF);
                sg = sg + ((in_p >> 8) & 0xFF) - ((out_p >> 8) & 0xFF);
                sb = sb + (in_p & 0xFF) - (out_p & 0xFF);
            }
        }

        for x in (x0 as usize)..(x1 as usize) {
            for j in 0..rh {
                scratch[j] = pixels[(y0 as usize + j) * stride + x];
            }
            let first = scratch[0];
            let mut sr = ((first >> 16) & 0xFF) * (r as u32 + 1);
            let mut sg = ((first >> 8) & 0xFF) * (r as u32 + 1);
            let mut sb = (first & 0xFF) * (r as u32 + 1);
            for j in 1..=r {
                let p = scratch[j.min(rh - 1)];
                sr += (p >> 16) & 0xFF;
                sg += (p >> 8) & 0xFF;
                sb += p & 0xFF;
            }
            for j in 0..rh {
                pixels[(y0 as usize + j) * stride + x] = ((sr / win) << 16) | ((sg / win) << 8) | (sb / win);
                let out_p = scratch[j.saturating_sub(r)];
                let in_p = scratch[(j + r + 1).min(rh - 1)];
                sr = sr + ((in_p >> 16) & 0xFF) - ((out_p >> 16) & 0xFF);
                sg = sg + ((in_p >> 8) & 0xFF) - ((out_p >> 8) & 0xFF);
                sb = sb + (in_p & 0xFF) - (out_p & 0xFF);
            }
        }
    }
}

fn draw_frosted_modal(canvas: &mut Canvas<'_>, rect: Rect, unit: i32) {
    draw_shadow(canvas, rect, unit / 2, 5, 16, rgb(18, 42, 68), 44);
    blur_rect_in_place(canvas, rect, 10);
    draw_rounded_surface(
        canvas,
        rect,
        unit / 2,
        rgb(254, 255, 255),
        rgb(241, 249, 253),
        232,
        AQUA_MINT,
        true,
    );
}

fn draw_aero_pill_button(
    canvas: &mut Canvas<'_>,
    rect: Rect,
    label: &str,
    hovered: bool,
    primary: bool,
    enabled: bool,
) {
    let radius = rect.height / 2;
    let st = text_style((rect.height * 42 / 100).max(9), 15);
    let tw = text_width(label, &st);
    let tx = rect.x + (rect.width - tw) / 2;
    let ty = rect.y + (rect.height - st.cap_height) / 2;

    if !enabled {
        draw_rounded_surface(
            canvas,
            rect,
            radius,
            SURFACE_ALT,
            rgb(232, 239, 245),
            255,
            BORDER_SOFT,
            false,
        );
        draw_text_color(canvas, label, tx, ty, &st, INK_DIM);
        return;
    }

    if primary {
        let (top_c, bot_c) = if hovered {
            (AZURE_DEEP, AQUA_DEEP)
        } else {
            (AZURE_BRIGHT, AQUA_MINT)
        };
        draw_rounded_surface(canvas, rect, radius, top_c, bot_c, 255, AZURE_DEEP, true);
        draw_text_color(canvas, label, tx, ty, &st, rgb(255, 255, 255));
    } else {
        let (top_c, bot_c, brd) = if hovered {
            (SURFACE_HOVER, rgb(218, 238, 249), AZURE_BRIGHT)
        } else {
            (SURFACE_WHITE, SURFACE_ALT, BORDER_SOFT)
        };
        draw_rounded_surface(canvas, rect, radius, top_c, bot_c, 250, brd, true);
        draw_text_color(canvas, label, tx, ty, &st, INK_PRIMARY);
    }
}

fn draw_entry_icon(canvas: &mut Canvas<'_>, x: i32, y: i32, is_dir: bool, is_pkg: bool, is_txt: bool) {
    if is_dir {
        canvas.fill_rect(x + 1, y + 2, 7, 3, AZURE_BRIGHT);
        let body = Rect::new(x, y + 4, 18, 12);
        draw_rounded_surface(canvas, body, 3, rgb(96, 178, 252), AZURE_BRIGHT, 255, AZURE_DEEP, true);
    } else if is_pkg {
        let orb = Rect::new(x + 1, y + 1, 16, 15);
        draw_rounded_surface(canvas, orb, 5, AQUA_SOFT, AQUA_MINT, 255, AQUA_DEEP, true);
        draw_dot(canvas, x + 9, y + 8, 3, rgb(255, 255, 255), 255);
    } else if is_txt {
        let page = Rect::new(x + 2, y + 1, 14, 15);
        draw_rounded_surface(canvas, page, 2, SURFACE_WHITE, SURFACE_ALT, 255, AZURE_BRIGHT, false);
        canvas.fill_rect(x + 5, y + 5, 8, 1, AZURE_BRIGHT);
        canvas.fill_rect(x + 5, y + 8, 8, 1, INK_MUTED);
        canvas.fill_rect(x + 5, y + 11, 6, 1, INK_MUTED);
    } else {
        let doc = Rect::new(x + 2, y + 1, 14, 15);
        draw_rounded_surface(canvas, doc, 2, SURFACE_WHITE, SURFACE_ALT, 255, BORDER_SOFT, false);
        draw_dot(canvas, x + 9, y + 8, 2, INK_MUTED, 255);
    }
}

fn draw_single_line_input(canvas: &mut Canvas<'_>, box_r: Rect, text: &str, style: &Style) {
    draw_rounded_surface(
        canvas,
        box_r,
        box_r.height / 3,
        SURFACE_WHITE,
        SURFACE_ALT,
        255,
        AZURE_BRIGHT,
        true,
    );
    let ty = box_r.y + (box_r.height - style.cap_height) / 2;
    let tx = box_r.x + 12;
    draw_text_color(canvas, text, tx, ty, style, INK_PRIMARY);
    let tw = text_width(text, style);
    canvas.fill_rect(tx + tw + 2, ty - 1, 2, style.cap_height + 2, AQUA_MINT);
}

fn draw_multiline_editor(
    canvas: &mut Canvas<'_>,
    box_r: Rect,
    content: &str,
    scroll_line: usize,
    editable: bool,
    line_style: &Style,
    num_style: &Style,
) {
    draw_rounded_surface(
        canvas,
        box_r,
        8,
        SURFACE_WHITE,
        rgb(248, 252, 255),
        255,
        if editable { AZURE_BRIGHT } else { BORDER_SOFT },
        true,
    );
    let gutter_w = 28;
    canvas.fill_rect(
        box_r.x + 2,
        box_r.y + 2,
        gutter_w,
        box_r.height - 4,
        SURFACE_ALT,
    );
    canvas.fill_rect(
        box_r.x + gutter_w + 2,
        box_r.y + 2,
        1,
        box_r.height - 4,
        BORDER_SOFT,
    );

    let line_step = (line_style.cap_height + 7).max(15);
    let max_lines = ((box_r.height - 14) / line_step).max(1) as usize;
    let total_lines = content.lines().count().max(1);

    for (vis_idx, raw_line) in content.lines().skip(scroll_line).take(max_lines).enumerate() {
        let ly = box_r.y + 8 + vis_idx as i32 * line_step;
        let mut lnum = Line::new();
        lnum.number((scroll_line + vis_idx + 1) as u32, 1);
        draw_text_color(canvas, lnum.as_str(), box_r.x + 7, ly + 1, num_style, INK_DIM);
        draw_text_color(canvas, raw_line, box_r.x + gutter_w + 10, ly, line_style, INK_PRIMARY);

        if editable && scroll_line + vis_idx + 1 == total_lines {
            let lw = text_width(raw_line, line_style);
            canvas.fill_rect(
                box_r.x + gutter_w + 12 + lw,
                ly - 1,
                2,
                line_style.cap_height + 2,
                AQUA_MINT,
            );
        }
    }

    if content.is_empty() && editable {
        canvas.fill_rect(
            box_r.x + gutter_w + 10,
            box_r.y + 8,
            2,
            line_style.cap_height + 2,
            AQUA_MINT,
        );
    }
}

fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

pub fn create_packaged_app(pkg: &NtzcPackage) -> Box<dyn App> {
    FilesApp::create_app_instance(pkg)
}

#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use nc_app::text::Line;
use nc_app::theme::{SIDEBAR, SIDEBAR_HOVER};
use nc_app::{App, AppInfo, Context, Icon, Input, KeyCode, Update};
use nc_app_settings::SettingsApp;
use nc_app_stopwatch::StopwatchApp;
use nc_app_system::SystemApp;
use nc_fs::{DirEntry, FileType, VfsManager};
use nc_gfx::{Canvas, Rect};
use nc_pkg::NtzcPackage;
use nc_ui::{
    draw_badge, draw_button, draw_card, draw_input_box, draw_panel, draw_text_color, text_style,
    PANEL_BORDER, PANEL_FILL, TEXT_MUTED, TEXT_PRIMARY, WHITE,
};

enum ModalState {
    None,
    Install(NtzcPackage),
    ViewText {
        path: String,
        content: String,
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
    selected_index: Option<usize>,
    hover_index: Option<usize>,
    hover_back: bool,
    hover_new_folder: bool,
    hover_new_file: bool,
    hover_delete: bool,
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
            info: AppInfo::new("nc.files", "FILES", "2.0.0", Icon::Generic, 44, 28),
            vfs,
            current_path: initial_path,
            items,
            selected_index: None,
            hover_index: None,
            hover_back: false,
            hover_new_folder: false,
            hover_new_file: false,
            hover_delete: false,
            modal: ModalState::None,
            modal_hover_primary: false,
            modal_hover_secondary: false,
        }
    }

    fn refresh(&mut self) {
        self.items = self.vfs.list_dir_path(&self.current_path).unwrap_or_else(|_| Vec::new());
        self.hover_index = None;
        self.selected_index = None;
    }

    fn enter_dir(&mut self, name: &str) {
        if self.current_path == "/" {
            self.current_path = alloc::format!("/{}", name);
        } else {
            self.current_path.push('/');
            self.current_path.push_str(name);
        }
        self.refresh();
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
        self.refresh();
    }

    fn navigate_to(&mut self, target_path: &str) {
        self.current_path = String::from(target_path);
        self.refresh();
    }

    fn format_size(bytes: u64, buf: &mut [u8]) -> usize {
        if bytes == 0 {
            buf[0] = b'-';
            return 1;
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

    fn create_app_instance(pkg: &NtzcPackage) -> Box<dyn App> {
        match pkg.manifest.id.as_str() {
            "nc.settings" => Box::new(SettingsApp::new()),
            "nc.system" => Box::new(SystemApp::new()),
            "nc.stopwatch" => Box::new(StopwatchApp::new()),
            _ => Box::new(GenericInstalledApp::new(pkg)),
        }
    }

    fn sidebar_item_rect(area: Rect, unit: i32, idx: usize) -> Rect {
        let side_x = area.x + unit / 2;
        let side_y = area.y + unit * 4 + idx as i32 * (unit * 3 + unit / 4);
        Rect::new(side_x, side_y, unit * 11, unit * 3)
    }

    fn back_btn_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit * 13, area.y + unit * 3 / 2, unit * 4, unit * 2)
    }

    fn new_folder_btn_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + area.width - unit * 18, area.y + unit * 3 / 2, unit * 5, unit * 2)
    }

    fn new_file_btn_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + area.width - unit * 12, area.y + unit * 3 / 2, unit * 5, unit * 2)
    }

    fn delete_btn_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + area.width - unit * 6, area.y + unit * 3 / 2, unit * 5, unit * 2)
    }

    fn path_box_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit * 13, area.y + unit * 4, area.width - unit * 14, unit * 2)
    }

    fn file_row_rect(area: Rect, unit: i32, idx: usize) -> Rect {
        let main_x = area.x + unit * 13;
        let main_w = area.width - unit * 14;
        let top = area.y + unit * 7 + idx as i32 * (unit * 5 / 2);
        Rect::new(main_x, top, main_w, unit * 2)
    }

    fn modal_rect(area: Rect, unit: i32) -> Rect {
        Rect::new(area.x + unit * 3, area.y + unit * 2, area.width - unit * 6, area.height - unit * 4)
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

        match self.modal {
            ModalState::Install(ref pkg) => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 8;
                let btn_h = unit * 2 + unit / 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let inst_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let canc_btn = Rect::new(inst_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

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
            ModalState::ViewText { ref path, ref mut content, ref mut modified } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let save_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let close_btn = Rect::new(save_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    match key {
                        KeyCode::Enter => {
                            content.push('\n');
                            *modified = true;
                            redraw = true;
                        }
                        KeyCode::Space => {
                            content.push(' ');
                            *modified = true;
                            redraw = true;
                        }
                        KeyCode::Escape => {
                            close_modal = true;
                        }
                        _ => {}
                    }
                }

                if let Some(pointer) = input.pointer {
                    let h_save = inside(save_btn, pointer.x, pointer.y);
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
                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    match key {
                        KeyCode::Escape => {
                            close_modal = true;
                        }
                        KeyCode::Space => {
                            name_buf.push('_');
                            redraw = true;
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
                            let folder_name = if name_buf.is_empty() { "new_folder" } else { name_buf.as_str() };
                            let target_path = if self.current_path == "/" {
                                alloc::format!("/ram/{}", folder_name)
                            } else {
                                alloc::format!("{}/{}", self.current_path, folder_name)
                            };
                            action_create_dir = Some(target_path);
                            close_modal = true;
                        } else if h_cancel {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::CreateFile { ref mut name_buf } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                if let Some(key) = input.key {
                    match key {
                        KeyCode::Escape => {
                            close_modal = true;
                        }
                        KeyCode::Space => {
                            name_buf.push('_');
                            redraw = true;
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
                            let file_name = if name_buf.is_empty() { "notes.txt" } else { name_buf.as_str() };
                            let target_path = if self.current_path == "/" {
                                alloc::format!("/ram/{}", file_name)
                            } else {
                                alloc::format!("{}/{}", self.current_path, file_name)
                            };
                            action_create_file = Some(target_path);
                            close_modal = true;
                        } else if h_cancel {
                            close_modal = true;
                        }
                    }
                }
            }
            ModalState::DeleteConfirm { ref name, .. } => {
                let m_rect = Self::modal_rect(input.area, unit);
                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let del_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(del_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

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
                            let target_path = if self.current_path == "/" {
                                alloc::format!("/{}", name)
                            } else {
                                alloc::format!("{}/{}", self.current_path, name)
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
            let _ = self.vfs.write_file_path(&p, c.as_bytes());
            self.refresh();
        }
        if let Some(p) = action_create_dir {
            let _ = self.vfs.create_dir_path(&p);
            self.refresh();
        }
        if let Some(p) = action_create_file {
            let _ = self.vfs.write_file_path(&p, b"NetizenCore Document Content");
            self.refresh();
        }
        if let Some(p) = action_delete {
            let _ = self.vfs.remove_path(&p);
            self.refresh();
        }

        if close_modal {
            self.modal = ModalState::None;
            redraw = true;
        }

        if let Some(app) = action_install {
            return Update::install(app);
        }

        if !matches!(self.modal, ModalState::None) {
            return Update { redraw, close: false, install_app: None };
        }

        let mut next_hover = None;
        let mut next_back_hover = false;
        let mut next_new_folder = false;
        let mut next_new_file = false;
        let mut next_delete = false;

        if let Some(pointer) = input.pointer {
            if self.current_path != "/" {
                next_back_hover = inside(Self::back_btn_rect(input.area, unit), pointer.x, pointer.y);
            }
            next_new_folder = inside(Self::new_folder_btn_rect(input.area, unit), pointer.x, pointer.y);
            next_new_file = inside(Self::new_file_btn_rect(input.area, unit), pointer.x, pointer.y);
            next_delete = self.selected_index.is_some() && inside(Self::delete_btn_rect(input.area, unit), pointer.x, pointer.y);

            for i in 0..self.items.len() {
                if inside(Self::file_row_rect(input.area, unit, i), pointer.x, pointer.y) {
                    next_hover = Some(i);
                    break;
                }
            }

            if pointer.clicked {
                if next_back_hover {
                    self.up_dir();
                    redraw = true;
                } else if next_new_folder {
                    self.modal = ModalState::CreateFolder { name_buf: String::from("new_folder") };
                    redraw = true;
                } else if next_new_file {
                    self.modal = ModalState::CreateFile { name_buf: String::from("notes.txt") };
                    redraw = true;
                } else if next_delete {
                    if let Some(idx) = self.selected_index {
                        if idx < self.items.len() {
                            let item = &self.items[idx];
                            self.modal = ModalState::DeleteConfirm {
                                name: item.name.clone(),
                                is_dir: item.file_type == FileType::Directory,
                            };
                            redraw = true;
                        }
                    }
                }

                for i in 0..4 {
                    if inside(Self::sidebar_item_rect(input.area, unit, i), pointer.x, pointer.y) {
                        let target = match i {
                            0 => "/",
                            1 => "/system",
                            2 => "/data",
                            _ => "/ram",
                        };
                        self.navigate_to(target);
                        redraw = true;
                    }
                }

                if let Some(idx) = next_hover {
                    self.selected_index = Some(idx);
                    let entry = self.items[idx].clone();
                    if entry.file_type == FileType::Directory {
                        self.enter_dir(&entry.name);
                        redraw = true;
                    } else {
                        let full_path = if self.current_path == "/" {
                            alloc::format!("/{}", entry.name)
                        } else {
                            alloc::format!("{}/{}", self.current_path, entry.name)
                        };

                        if entry.name.ends_with(".ntzc") {
                            if let Ok(bytes) = self.vfs.read_file_path(&full_path) {
                                if let Some(pkg) = NtzcPackage::parse(&bytes) {
                                    self.modal = ModalState::Install(pkg);
                                    redraw = true;
                                }
                            }
                        } else if entry.name.ends_with(".txt") || entry.name.ends_with(".prop") || entry.name.ends_with(".cfg") {
                            if let Ok(bytes) = self.vfs.read_file_path(&full_path) {
                                let text_str = String::from_utf8_lossy(&bytes).into_owned();
                                self.modal = ModalState::ViewText {
                                    path: full_path,
                                    content: text_str,
                                    modified: false,
                                };
                                redraw = true;
                            }
                        }
                    }
                }
            }
        }

        if next_hover != self.hover_index
            || next_back_hover != self.hover_back
            || next_new_folder != self.hover_new_folder
            || next_new_file != self.hover_new_file
            || next_delete != self.hover_delete
        {
            self.hover_index = next_hover;
            self.hover_back = next_back_hover;
            self.hover_new_folder = next_new_folder;
            self.hover_new_file = next_new_file;
            self.hover_delete = next_delete;
            redraw = true;
        }

        Update { redraw, close: false, install_app: None }
    }

    fn draw(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context) {
        let unit = ctx.unit;
        let side_rect = Rect::new(area.x + unit / 2, area.y + unit / 2, unit * 11 + unit / 2, area.height - unit);
        draw_panel(canvas, side_rect, unit / 2, SIDEBAR, 255, PANEL_BORDER, 255);

        let title_style = text_style(unit * 11 / 10, 20);
        let cap_style = text_style(unit * 55 / 100, 40);
        let label_style = text_style(unit * 60 / 100, 30);

        draw_text_color(canvas, "LOCATIONS", side_rect.x + unit / 2, area.y + unit * 2, &cap_style, TEXT_MUTED);

        let targets = [
            ("Root (/)", "/"),
            ("System (/system)", "/system"),
            ("Data (/data)", "/data"),
            ("RAM Disk (/ram)", "/ram"),
        ];

        for (i, (name, target)) in targets.iter().enumerate() {
            let item_rect = Self::sidebar_item_rect(area, unit, i);
            let selected = self.current_path.starts_with(target) && (target.len() > 1 || self.current_path == "/");
            let bg = if selected { ctx.settings.accent_color() } else { SIDEBAR };
            let fg = if selected { WHITE } else { TEXT_PRIMARY };
            draw_panel(canvas, item_rect, unit / 3, bg, 255, PANEL_BORDER, if selected { 0 } else { 255 });
            draw_text_color(canvas, name, item_rect.x + unit / 3, item_rect.y + (item_rect.height - label_style.cap_height) / 2, &label_style, fg);
        }

        if self.current_path != "/" {
            let back_rect = Self::back_btn_rect(area, unit);
            draw_button(canvas, back_rect, "BACK", self.hover_back, false);
        }

        draw_button(canvas, Self::new_folder_btn_rect(area, unit), "+FOLDER", self.hover_new_folder, false);
        draw_button(canvas, Self::new_file_btn_rect(area, unit), "+FILE", self.hover_new_file, false);

        if self.selected_index.is_some() {
            draw_button(canvas, Self::delete_btn_rect(area, unit), "DELETE", self.hover_delete, false);
        }

        let path_box = Self::path_box_rect(area, unit);
        draw_input_box(canvas, path_box, &self.current_path, false);

        let main_x = area.x + unit * 13;

        if self.items.is_empty() {
            draw_text_color(canvas, "THIS FOLDER IS EMPTY", main_x, area.y + unit * 8, &label_style, TEXT_MUTED);
        }

        for (i, item) in self.items.iter().enumerate() {
            let row = Self::file_row_rect(area, unit, i);
            let is_selected = self.selected_index == Some(i);
            let is_hov = self.hover_index == Some(i);
            let bg = if is_selected {
                ctx.settings.accent_color()
            } else if is_hov {
                SIDEBAR_HOVER
            } else {
                PANEL_FILL
            };
            let text_color = if is_selected { WHITE } else { TEXT_PRIMARY };

            draw_panel(canvas, row, unit / 3, bg, 255, PANEL_BORDER, 255);

            let is_pkg = item.name.ends_with(".ntzc");
            let is_txt = item.name.ends_with(".txt") || item.name.ends_with(".prop") || item.name.ends_with(".cfg");
            let is_dir = item.file_type == FileType::Directory;

            let (tag, badge_bg, badge_fg) = if is_dir {
                ("FOLDER", PANEL_BORDER, TEXT_PRIMARY)
            } else if is_pkg {
                ("APP", ctx.settings.accent_color(), WHITE)
            } else if is_txt {
                ("TEXT", PANEL_BORDER, TEXT_PRIMARY)
            } else {
                ("FILE", PANEL_BORDER, TEXT_MUTED)
            };

            let badge_rect = Rect::new(row.x + unit / 2, row.y + (row.height - unit * 3 / 2) / 2, unit * 5, unit * 3 / 2);
            draw_badge(canvas, badge_rect, tag, badge_bg, badge_fg);

            let text_y = row.y + (row.height - label_style.cap_height) / 2;
            draw_text_color(canvas, &item.name, row.x + unit * 6, text_y, &label_style, text_color);

            if !is_dir {
                let mut sz_buf = [0u8; 16];
                let sz_len = Self::format_size(item.size, &mut sz_buf);
                if let Ok(st) = core::str::from_utf8(&sz_buf[..sz_len]) {
                    draw_text_color(canvas, st, row.x + row.width - unit * 6, text_y, &cap_style, if is_selected { WHITE } else { TEXT_MUTED });
                }
            }
        }

        match self.modal {
            ModalState::Install(ref pkg) => {
                let m_rect = Self::modal_rect(area, unit);
                draw_card(canvas, m_rect, unit);

                let mut modal_title = Line::new();
                modal_title.text("INSTALL PACKAGE: ").text(&pkg.manifest.name);
                draw_text_color(canvas, modal_title.as_str(), m_rect.x + unit * 2, m_rect.y + unit * 2, &title_style, TEXT_PRIMARY);

                draw_text_color(canvas, pkg.role_label(), m_rect.x + unit * 2, m_rect.y + unit * 4, &cap_style, ctx.settings.accent_color());

                draw_text_color(canvas, "REQUIRED PERMISSIONS:", m_rect.x + unit * 2, m_rect.y + unit * 6, &cap_style, TEXT_MUTED);
                let labels = pkg.permission_labels();
                for (idx, lbl) in labels.iter().enumerate() {
                    let mut line = Line::new();
                    line.text("- ").text(lbl);
                    draw_text_color(canvas, line.as_str(), m_rect.x + unit * 2, m_rect.y + unit * 8 + idx as i32 * (unit * 3 / 2), &label_style, TEXT_PRIMARY);
                }

                let btn_w = unit * 8;
                let btn_h = unit * 2 + unit / 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let inst_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let canc_btn = Rect::new(inst_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_button(canvas, canc_btn, "CANCEL", self.modal_hover_secondary, false);
                draw_button(canvas, inst_btn, "INSTALL", self.modal_hover_primary, true);
            }
            ModalState::ViewText { ref path, ref content, modified } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_card(canvas, m_rect, unit);

                let title = alloc::format!("TEXT EDITOR: {}", path);
                draw_text_color(canvas, &title, m_rect.x + unit * 2, m_rect.y + unit * 2, &title_style, TEXT_PRIMARY);

                let status_msg = if modified { "MODIFIED (UNSAVED)" } else { "READ-ONLY" };
                draw_text_color(canvas, status_msg, m_rect.x + unit * 2, m_rect.y + unit * 4, &cap_style, ctx.settings.accent_color());

                let text_box = Rect::new(m_rect.x + unit * 2, m_rect.y + unit * 6, m_rect.width - unit * 4, m_rect.height - unit * 11);
                draw_input_box(canvas, text_box, content, true);

                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let save_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let close_btn = Rect::new(save_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_button(canvas, close_btn, "CLOSE", self.modal_hover_secondary, false);
                draw_button(canvas, save_btn, "SAVE", self.modal_hover_primary, true);
            }
            ModalState::CreateFolder { ref name_buf } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_card(canvas, m_rect, unit);

                draw_text_color(canvas, "CREATE NEW FOLDER", m_rect.x + unit * 2, m_rect.y + unit * 2, &title_style, TEXT_PRIMARY);
                draw_text_color(canvas, "Enter Folder Name:", m_rect.x + unit * 2, m_rect.y + unit * 5, &cap_style, TEXT_MUTED);

                let input_box = Rect::new(m_rect.x + unit * 2, m_rect.y + unit * 7, m_rect.width - unit * 4, unit * 3);
                draw_input_box(canvas, input_box, name_buf, true);

                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_button(canvas, cancel_btn, "CANCEL", self.modal_hover_secondary, false);
                draw_button(canvas, create_btn, "CREATE", self.modal_hover_primary, true);
            }
            ModalState::CreateFile { ref name_buf } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_card(canvas, m_rect, unit);

                draw_text_color(canvas, "CREATE NEW FILE", m_rect.x + unit * 2, m_rect.y + unit * 2, &title_style, TEXT_PRIMARY);
                draw_text_color(canvas, "Enter File Name:", m_rect.x + unit * 2, m_rect.y + unit * 5, &cap_style, TEXT_MUTED);

                let input_box = Rect::new(m_rect.x + unit * 2, m_rect.y + unit * 7, m_rect.width - unit * 4, unit * 3);
                draw_input_box(canvas, input_box, name_buf, true);

                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let create_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(create_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_button(canvas, cancel_btn, "CANCEL", self.modal_hover_secondary, false);
                draw_button(canvas, create_btn, "CREATE", self.modal_hover_primary, true);
            }
            ModalState::DeleteConfirm { ref name, is_dir } => {
                let m_rect = Self::modal_rect(area, unit);
                draw_card(canvas, m_rect, unit);

                let type_str = if is_dir { "Folder" } else { "File" };
                let msg = alloc::format!("Delete {} '{}'?", type_str, name);
                draw_text_color(canvas, "DELETE ITEM", m_rect.x + unit * 2, m_rect.y + unit * 2, &title_style, TEXT_PRIMARY);
                draw_text_color(canvas, &msg, m_rect.x + unit * 2, m_rect.y + unit * 5, &label_style, TEXT_PRIMARY);

                let btn_w = unit * 7;
                let btn_h = unit * 2;
                let btn_y = m_rect.y + m_rect.height - btn_h - unit;
                let del_btn = Rect::new(m_rect.x + m_rect.width - btn_w - unit, btn_y, btn_w, btn_h);
                let cancel_btn = Rect::new(del_btn.x - btn_w - unit / 2, btn_y, btn_w, btn_h);

                draw_button(canvas, cancel_btn, "CANCEL", self.modal_hover_secondary, false);
                draw_button(canvas, del_btn, "DELETE", self.modal_hover_primary, true);
            }
            ModalState::None => {}
        }
    }
}

pub struct GenericInstalledApp {
    info: AppInfo,
    uid: u32,
}

impl GenericInstalledApp {
    pub fn new(pkg: &NtzcPackage) -> Self {
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
        }
    }
}

impl App for GenericInstalledApp {
    fn info(&self) -> &AppInfo {
        &self.info
    }

    fn update(&mut self, _ctx: &mut Context, _input: &Input) -> Update {
        Update::NONE
    }

    fn draw(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context) {
        let unit = ctx.unit;
        let title_style = text_style(unit * 12 / 10, 20);
        let cap_style = text_style(unit * 6 / 10, 40);

        draw_panel(canvas, area, unit / 2, PANEL_FILL, 255, PANEL_BORDER, 255);
        draw_text_color(canvas, &self.info.name, area.x + unit * 2, area.y + unit * 2, &title_style, TEXT_PRIMARY);

        let role_str = if self.uid == 1000 { "System Package (Authorized)" } else { "User Package (Isolated)" };
        draw_text_color(canvas, role_str, area.x + unit * 2, area.y + unit * 5, &cap_style, ctx.settings.accent_color());
        draw_text_color(canvas, "APPLICATION RUNNING", area.x + unit * 2, area.y + unit * 8, &cap_style, TEXT_MUTED);
    }
}

fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

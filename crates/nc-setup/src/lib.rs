#![no_std]

use core::sync::atomic::{AtomicU64, Ordering};
use nc_font::text_width;
use nc_gfx::{lerp_color, rgb, Canvas, Rect};
use nc_hal::{BlockDeviceInfo, DeviceType, KeyCode, MouseState};
use nc_ui::{draw_dot, draw_shadow, draw_text_color, text_style};

pub const MAX_SETUP_DISKS: usize = 8;
const MAX_PARTITIONS: usize = 8;
const MAX_BLUR_SPAN: usize = 2048;

const PORCELAIN_TOP: u32 = rgb(246, 250, 253);
const PORCELAIN_BOT: u32 = rgb(231, 242, 247);
const CARD_WHITE: u32 = rgb(252, 254, 255);
const CARD_BORDER: u32 = rgb(212, 227, 237);
const SUBTLE_FILL: u32 = rgb(241, 247, 251);
const SUBTLE_HOVER: u32 = rgb(232, 243, 250);

const AZURE_BRIGHT: u32 = rgb(52, 144, 246);
const AZURE_DEEP: u32 = rgb(28, 108, 218);
const AQUA_MINT: u32 = rgb(32, 194, 174);
const AQUA_SOFT: u32 = rgb(146, 234, 220);
const AQUA_DEEP: u32 = rgb(18, 152, 138);
const TINT_SELECTED: u32 = rgb(230, 245, 252);

const INK_DARK: u32 = rgb(18, 30, 46);
const INK_BODY: u32 = rgb(58, 76, 96);
const INK_MUTED: u32 = rgb(116, 134, 154);
const INK_LIGHT: u32 = rgb(156, 172, 190);

static LAST_RENDER_HASH: AtomicU64 = AtomicU64::new(u64::MAX);

const LANGUAGES: [(&str, &str); 3] = [
    ("Turkce (Turkiye)", "Standart sistem dili ve yerel bicimlendirme"),
    ("English (United States)", "International system interface and messages"),
    ("English (United Kingdom)", "ISO-8859-1 / UTF-8 metric display profile"),
];

const TIMEZONES: [&str; 3] = [
    "UTC+03:00  Istanbul, Avrupa (24 saat)",
    "UTC+00:00  Evrensel EsGudumlu Saat",
    "UTC-05:00  Kuzey Amerika Dogu Saati",
];

const KEYBOARDS: [(&str, &str); 3] = [
    ("Turkce Q Klavye", "q w e r t y u i o p g u  |  a s d f g h j k l s i"),
    ("US QWERTY Standard", "q w e r t y u i o p [ ]  |  a s d f g h j k l ; '"),
    ("Turkce F Dizilimi", "f g g i o d r n h p q w  |  u i e a u t k m l y s"),
];

const ARCHITECTURE_NOTES: [&str; 22] = [
    "NetizenCore Isletim Sistemi Mimarisi ve .efra Surucu Sozlesmesi",
    "",
    "1. Saf ve Bagimsiz Cekirdek Yapisi",
    "   NetizenCore; Unix, POSIX veya Linux tabanli degildir. Onyukleyici",
    "   katmani NASM Assembly, cekirdek ve arayuz bilesenleri ise tamamen",
    "   no_std Rust ile sifirdan insa edilmistir.",
    "",
    "2. Hibrit Disk Bolumleme Semasi (MBR + VFS)",
    "   Kurulum sihirbazi secilen hedef diskin 0. sektorune (LBA 0) ozel",
    "   onyukleme kaydini yazar ve dort ana bolum olusturur:",
    "   - Bolum 1 (LBA 2048, 64 MB)    : Stage2 ve Cekirdek ikilisi",
    "   - Bolum 2 (LBA 133120, 2.0 GB) : Salt okunur EROFS /system agaci",
    "   - Bolum 3 (LBA 4327424, 256 MB): Sistem guvenlik ve anahtar kasasi",
    "   - Bolum 4 (LBA 4851712, Kalan) : Yazilabilir exFAT /data alani",
    "",
    "3. Moduler Generic .efra Surucu ve Hizlandirma Katmani",
    "   QEMU, VirtualBox ve Gercek PC donanimlari icin PAT Write-Combining,",
    "   64-bit Framebuffer Burst ve /data/drivers/*.efra paketleri kurulur.",
    "   Sistem icinden yeni .efra suruculeri derlenip guncellenebilir.",
    "",
    "4. Kurulum Onayi: Devam ederek disk yapilandirmasini onaylarsiniz.",
    "",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetupAction {
    None,
    Shutdown,
    Reboot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StepPhase {
    Welcome,
    Architecture,
    Storage,
    Installing,
    Completed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VolumeRole {
    Unallocated,
    BootCore,
    SystemErofs,
    SecurityVault,
    DataExfat,
    OpticalIso,
}

#[derive(Clone, Copy)]
pub struct VolumeEntry {
    pub disk_index: usize,
    pub index_on_disk: u8,
    pub role: VolumeRole,
    pub start_lba: u64,
    pub size_mb: u64,
    pub free_mb: u64,
}

impl VolumeEntry {
    const EMPTY: VolumeEntry = VolumeEntry {
        disk_index: 0,
        index_on_disk: 0,
        role: VolumeRole::Unallocated,
        start_lba: 0,
        size_mb: 0,
        free_mb: 0,
    };
}

pub struct SetupState {
    pub step: usize,
    phase: StepPhase,
    pub selected_disk: usize,
    pub disks: [BlockDeviceInfo; MAX_SETUP_DISKS],
    pub disk_count: usize,
    pub volumes: [VolumeEntry; MAX_PARTITIONS],
    pub volume_count: usize,
    pub selected_volume: usize,
    pub layout_prepared: bool,
    pub progress: u32,
    pub changed: bool,
    lang_index: usize,
    tz_index: usize,
    kbd_index: usize,
    terms_accepted: bool,
    doc_scroll: usize,
    profile_mode: usize,
    anim_ticks: u32,
    install_ticks: u32,
    reboot_ticks: u32,
    inspector_open: bool,
    banner_state: u8,
}

struct ShellLayout {
    top_pill: Rect,
    probe_btn: Rect,
    main_card: Rect,
    left_pane: Rect,
    right_pane: Rect,
    bottom_bar: Rect,
    back_btn: Rect,
    next_btn: Rect,
    power_btn: Rect,
}

fn compute_shell(width: i32, height: i32) -> ShellLayout {
    let shell_w = (width * 84 / 100).clamp(680, 940).min(width - 28);
    let shell_h = (height * 82 / 100).clamp(470, 610).min(height - 28);
    let shell_x = (width - shell_w) / 2;
    let shell_y = (height - shell_h) / 2;

    let top_pill = Rect::new(shell_x, shell_y, shell_w, 46);
    let probe_btn = Rect::new(top_pill.x + top_pill.width - 148, top_pill.y + 8, 138, 30);

    let main_y = top_pill.y + top_pill.height + 12;
    let bottom_h = 54;
    let main_h = shell_h - top_pill.height - bottom_h - 24;
    let main_card = Rect::new(shell_x, main_y, shell_w, main_h);

    let left_w = (shell_w * 35 / 100).clamp(220, 310);
    let left_pane = Rect::new(main_card.x + 12, main_card.y + 12, left_w, main_card.height - 24);
    let right_pane = Rect::new(
        left_pane.x + left_pane.width + 14,
        main_card.y + 12,
        main_card.width - left_w - 38,
        main_card.height - 24,
    );

    let bottom_bar = Rect::new(shell_x, main_card.y + main_card.height + 12, shell_w, bottom_h);
    let next_btn = Rect::new(bottom_bar.x + bottom_bar.width - 164, bottom_bar.y + 10, 152, 34);
    let back_btn = Rect::new(next_btn.x - 108, bottom_bar.y + 10, 98, 34);
    let power_btn = Rect::new(bottom_bar.x + 14, bottom_bar.y + 10, 132, 34);

    ShellLayout {
        top_pill,
        probe_btn,
        main_card,
        left_pane,
        right_pane,
        bottom_bar,
        back_btn,
        next_btn,
        power_btn,
    }
}

fn hit_rect(x: i32, y: i32, r: Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

fn wave_1000(deg: i32) -> i32 {
    let d = deg.rem_euclid(360);
    let (half, sign) = if d < 180 { (d, 1) } else { (d - 180, -1) };
    let num = 4 * half * (180 - half) * 1000;
    let den = 40500 - half * (180 - half);
    sign * (num / den.max(1))
}

fn organic_grain(x: i32, y: i32) -> i32 {
    let mut n = (x as u32).wrapping_mul(747_796_405).wrapping_add((y as u32).wrapping_mul(2_891_336_453));
    n = ((n >> ((n >> 28).wrapping_add(4))) ^ n).wrapping_mul(277_803_737);
    (((n >> 22) & 7) as i32) - 3
}

impl SetupState {
    pub fn new(disks_slice: &[BlockDeviceInfo]) -> SetupState {
        let mut disks = [BlockDeviceInfo::EMPTY; MAX_SETUP_DISKS];
        let count = disks_slice.len().min(MAX_SETUP_DISKS);
        for i in 0..count {
            disks[i] = disks_slice[i];
        }

        let mut state = SetupState {
            step: 0,
            phase: StepPhase::Welcome,
            selected_disk: 0,
            disks,
            disk_count: count,
            volumes: [VolumeEntry::EMPTY; MAX_PARTITIONS],
            volume_count: 0,
            selected_volume: 0,
            layout_prepared: true,
            progress: 0,
            changed: true,
            lang_index: 0,
            tz_index: 0,
            kbd_index: 0,
            terms_accepted: true,
            doc_scroll: 0,
            profile_mode: 0,
            anim_ticks: 0,
            install_ticks: 0,
            reboot_ticks: 0,
            inspector_open: false,
            banner_state: 0,
        };
        state.sync_volumes();
        LAST_RENDER_HASH.store(u64::MAX, Ordering::Relaxed);
        state
    }

    pub fn set_progress(&mut self, pct: u32, _stage_text: &'static str) {
        self.progress = (pct.min(100) * 256) / 100;
        self.changed = true;
        if pct >= 100 {
            self.phase = StepPhase::Completed;
            self.step = 3;
        }
    }

    fn sync_volumes(&mut self) {
        self.volume_count = 0;
        for (d_idx, dev) in self.disks[..self.disk_count].iter().enumerate() {
            if self.volume_count >= MAX_PARTITIONS {
                break;
            }
            if dev.device_type == DeviceType::OpticalDisc {
                self.volumes[self.volume_count] = VolumeEntry {
                    disk_index: d_idx,
                    index_on_disk: 1,
                    role: VolumeRole::OpticalIso,
                    start_lba: 0,
                    size_mb: dev.size_mb().max(16),
                    free_mb: 0,
                };
                self.volume_count += 1;
            } else if !self.layout_prepared {
                let total = dev.size_mb().max(4096);
                self.volumes[self.volume_count] = VolumeEntry {
                    disk_index: d_idx,
                    index_on_disk: 0,
                    role: VolumeRole::Unallocated,
                    start_lba: 2048,
                    size_mb: total,
                    free_mb: total,
                };
                self.volume_count += 1;
            } else {
                let total = dev.size_mb().max(4096);
                let boot_mb = 64u64;
                let sys_mb = 2048u64;
                let sec_mb = 256u64;
                let reserved = boot_mb + sys_mb + sec_mb;
                let data_mb = if total > reserved + 128 { total - reserved } else { 1536 };

                let entries = [
                    VolumeEntry {
                        disk_index: d_idx,
                        index_on_disk: 1,
                        role: VolumeRole::BootCore,
                        start_lba: 2048,
                        size_mb: boot_mb,
                        free_mb: 58,
                    },
                    VolumeEntry {
                        disk_index: d_idx,
                        index_on_disk: 2,
                        role: VolumeRole::SystemErofs,
                        start_lba: 133_120,
                        size_mb: sys_mb,
                        free_mb: 1620,
                    },
                    VolumeEntry {
                        disk_index: d_idx,
                        index_on_disk: 3,
                        role: VolumeRole::SecurityVault,
                        start_lba: 4_327_424,
                        size_mb: sec_mb,
                        free_mb: 250,
                    },
                    VolumeEntry {
                        disk_index: d_idx,
                        index_on_disk: 4,
                        role: VolumeRole::DataExfat,
                        start_lba: 4_851_712,
                        size_mb: data_mb,
                        free_mb: data_mb.saturating_sub(16),
                    },
                ];
                for e in entries {
                    if self.volume_count < MAX_PARTITIONS {
                        self.volumes[self.volume_count] = e;
                        self.volume_count += 1;
                    }
                }
            }
        }

        if self.volume_count == 0 {
            self.volumes[0] = VolumeEntry {
                disk_index: 0,
                index_on_disk: 0,
                role: VolumeRole::Unallocated,
                start_lba: 2048,
                size_mb: 20480,
                free_mb: 20480,
            };
            self.volume_count = 1;
        }

        for i in 0..self.volume_count {
            if self.volumes[i].role == VolumeRole::DataExfat || self.volumes[i].role == VolumeRole::Unallocated {
                self.selected_volume = i;
                self.selected_disk = self.volumes[i].disk_index;
                break;
            }
        }
    }

    fn can_continue(&self) -> bool {
        match self.phase {
            StepPhase::Welcome => true,
            StepPhase::Architecture => self.terms_accepted,
            StepPhase::Storage => {
                if self.selected_volume < self.volume_count {
                    self.volumes[self.selected_volume].role != VolumeRole::OpticalIso
                } else {
                    false
                }
            }
            StepPhase::Installing => false,
            StepPhase::Completed => true,
        }
    }

    fn step_forward(&mut self) -> SetupAction {
        match self.phase {
            StepPhase::Welcome => {
                self.phase = StepPhase::Architecture;
                self.step = 0;
                self.changed = true;
            }
            StepPhase::Architecture => {
                if self.terms_accepted {
                    self.phase = StepPhase::Storage;
                    self.step = 1;
                    self.changed = true;
                }
            }
            StepPhase::Storage => {
                if self.can_continue() {
                    if !self.layout_prepared {
                        self.layout_prepared = true;
                        self.sync_volumes();
                    }
                    self.phase = StepPhase::Installing;
                    self.install_ticks = 0;
                    self.progress = 6;
                    self.step = 1;
                    self.changed = true;
                }
            }
            StepPhase::Installing => {}
            StepPhase::Completed => {
                return SetupAction::Reboot;
            }
        }
        SetupAction::None
    }

    fn step_backward(&mut self) {
        match self.phase {
            StepPhase::Architecture => {
                self.phase = StepPhase::Welcome;
                self.step = 0;
                self.changed = true;
            }
            StepPhase::Storage => {
                self.phase = StepPhase::Architecture;
                self.step = 0;
                self.changed = true;
            }
            _ => {}
        }
    }

    pub fn update(&mut self, mouse: &MouseState, key: Option<KeyCode>, width: i32, height: i32) -> SetupAction {
        self.anim_ticks = self.anim_ticks.wrapping_add(1);
        let shell = compute_shell(width, height);

        if self.inspector_open {
            let modal = inspector_rect(width, height);
            let close_btn = Rect::new(modal.x + modal.width - 116, modal.y + modal.height - 44, 100, 30);
            if key == Some(KeyCode::Escape) || key == Some(KeyCode::Enter) {
                self.inspector_open = false;
                self.changed = true;
            } else if mouse.left_clicked && hit_rect(mouse.x, mouse.y, close_btn) {
                self.inspector_open = false;
                self.changed = true;
            }
            return SetupAction::None;
        }

        if let Some(k) = key {
            match k {
                KeyCode::Enter => {
                    let act = self.step_forward();
                    if act != SetupAction::None {
                        return act;
                    }
                }
                KeyCode::Space => {
                    if self.phase == StepPhase::Architecture {
                        self.terms_accepted = !self.terms_accepted;
                        self.changed = true;
                    } else {
                        let act = self.step_forward();
                        if act != SetupAction::None {
                            return act;
                        }
                    }
                }
                KeyCode::Escape => {
                    self.step_backward();
                }
                KeyCode::Up => match self.phase {
                    StepPhase::Welcome => {
                        self.lang_index = (self.lang_index + LANGUAGES.len() - 1) % LANGUAGES.len();
                        self.changed = true;
                    }
                    StepPhase::Architecture => {
                        if self.doc_scroll > 0 {
                            self.doc_scroll -= 1;
                            self.changed = true;
                        }
                    }
                    StepPhase::Storage => {
                        if self.selected_volume > 0 {
                            self.selected_volume -= 1;
                            self.selected_disk = self.volumes[self.selected_volume].disk_index;
                            self.changed = true;
                        }
                    }
                    _ => {}
                },
                KeyCode::Down => match self.phase {
                    StepPhase::Welcome => {
                        self.lang_index = (self.lang_index + 1) % LANGUAGES.len();
                        self.changed = true;
                    }
                    StepPhase::Architecture => {
                        if self.doc_scroll + 7 < ARCHITECTURE_NOTES.len() {
                            self.doc_scroll += 1;
                            self.changed = true;
                        }
                    }
                    StepPhase::Storage => {
                        if self.selected_volume + 1 < self.volume_count {
                            self.selected_volume += 1;
                            self.selected_disk = self.volumes[self.selected_volume].disk_index;
                            self.changed = true;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if mouse.left_clicked {
            if hit_rect(mouse.x, mouse.y, shell.probe_btn) {
                self.inspector_open = true;
                self.changed = true;
                return SetupAction::None;
            }
            if hit_rect(mouse.x, mouse.y, shell.power_btn) && self.phase != StepPhase::Installing {
                return SetupAction::Reboot;
            }
            if hit_rect(mouse.x, mouse.y, shell.next_btn) && self.can_continue() {
                let act = self.step_forward();
                if act != SetupAction::None {
                    return act;
                }
            } else if (self.phase == StepPhase::Architecture || self.phase == StepPhase::Storage)
                && hit_rect(mouse.x, mouse.y, shell.back_btn)
            {
                self.step_backward();
            } else {
                match self.phase {
                    StepPhase::Welcome => {
                        for i in 0..3 {
                            let r = welcome_lang_card(shell.right_pane, i);
                            if hit_rect(mouse.x, mouse.y, r) {
                                self.lang_index = i;
                                self.changed = true;
                            }
                        }
                        let tz_r = welcome_selector_row(shell.right_pane, 0);
                        let kb_r = welcome_selector_row(shell.right_pane, 1);
                        if hit_rect(mouse.x, mouse.y, tz_r) {
                            self.tz_index = (self.tz_index + 1) % TIMEZONES.len();
                            self.changed = true;
                        } else if hit_rect(mouse.x, mouse.y, kb_r) {
                            self.kbd_index = (self.kbd_index + 1) % KEYBOARDS.len();
                            self.changed = true;
                        }
                    }
                    StepPhase::Architecture => {
                        let doc_r = arch_doc_rect(shell.right_pane);
                        let up_r = Rect::new(doc_r.x + doc_r.width - 24, doc_r.y + 6, 18, 22);
                        let dn_r = Rect::new(doc_r.x + doc_r.width - 24, doc_r.y + doc_r.height - 28, 18, 22);
                        if hit_rect(mouse.x, mouse.y, up_r) && self.doc_scroll > 0 {
                            self.doc_scroll -= 1;
                            self.changed = true;
                        } else if hit_rect(mouse.x, mouse.y, dn_r)
                            && self.doc_scroll + 7 < ARCHITECTURE_NOTES.len()
                        {
                            self.doc_scroll += 1;
                            self.changed = true;
                        }
                        let m0 = arch_mode_card(shell.right_pane, 0);
                        let m1 = arch_mode_card(shell.right_pane, 1);
                        if hit_rect(mouse.x, mouse.y, m0) {
                            self.profile_mode = 0;
                            self.changed = true;
                        } else if hit_rect(mouse.x, mouse.y, m1) {
                            self.profile_mode = 1;
                            self.changed = true;
                        }
                        let toggle_r = arch_consent_rect(shell.right_pane);
                        if hit_rect(mouse.x, mouse.y, toggle_r) {
                            self.terms_accepted = !self.terms_accepted;
                            self.changed = true;
                        }
                    }
                    StepPhase::Storage => {
                        for i in 0..self.volume_count.min(5) {
                            let vr = storage_row_rect(shell.right_pane, i);
                            if hit_rect(mouse.x, mouse.y, vr) {
                                self.selected_volume = i;
                                self.selected_disk = self.volumes[i].disk_index;
                                self.changed = true;
                            }
                        }
                        for t in 0..4 {
                            let tr = storage_action_pill(shell.right_pane, t);
                            if hit_rect(mouse.x, mouse.y, tr) {
                                match t {
                                    0 => {
                                        self.sync_volumes();
                                        self.banner_state = 1;
                                    }
                                    1 => {
                                        self.layout_prepared = false;
                                        self.sync_volumes();
                                        self.banner_state = 2;
                                    }
                                    2 => {
                                        self.layout_prepared = true;
                                        self.sync_volumes();
                                        self.banner_state = 3;
                                    }
                                    _ => {
                                        self.layout_prepared = true;
                                        self.sync_volumes();
                                        self.banner_state = 4;
                                    }
                                }
                                self.changed = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if self.phase == StepPhase::Installing {
            self.install_ticks += 1;
            if self.install_ticks == 3 {
                self.step = 2;
            }
            let delta = match self.progress {
                0..=40 => 2,
                41..=105 => 1,
                106..=175 => 3,
                176..=225 => 1,
                _ => 2,
            };
            if self.install_ticks >= 3 && self.progress < 256 {
                self.progress = (self.progress + delta).min(256);
                self.changed = true;
                if self.progress >= 256 {
                    self.phase = StepPhase::Completed;
                    self.reboot_ticks = 0;
                    self.step = 3;
                }
            }
        } else if self.phase == StepPhase::Completed {
            self.reboot_ticks += 1;
            if self.reboot_ticks % 15 == 0 {
                self.changed = true;
            }
            if self.reboot_ticks >= 600 {
                return SetupAction::Reboot;
            }
        }

        SetupAction::None
    }
}

fn welcome_lang_card(pane: Rect, idx: usize) -> Rect {
    Rect::new(pane.x + 14, pane.y + 56 + idx as i32 * 50, pane.width - 28, 44)
}

fn welcome_selector_row(pane: Rect, idx: usize) -> Rect {
    Rect::new(pane.x + 14, pane.y + 224 + idx as i32 * 44, pane.width - 28, 36)
}

fn arch_doc_rect(pane: Rect) -> Rect {
    Rect::new(pane.x + 14, pane.y + 54, pane.width - 28, 152)
}

fn arch_mode_card(pane: Rect, idx: usize) -> Rect {
    let w = (pane.width - 38) / 2;
    Rect::new(pane.x + 14 + idx as i32 * (w + 10), pane.y + 216, w, 52)
}

fn arch_consent_rect(pane: Rect) -> Rect {
    Rect::new(pane.x + 14, pane.y + 278, pane.width - 28, 36)
}

fn storage_row_rect(pane: Rect, idx: usize) -> Rect {
    Rect::new(pane.x + 14, pane.y + 56 + idx as i32 * 36, pane.width - 28, 32)
}

fn storage_action_pill(pane: Rect, idx: usize) -> Rect {
    let w = (pane.width - 46) / 4;
    Rect::new(pane.x + 14 + idx as i32 * (w + 6), pane.y + 244, w, 28)
}

fn inspector_rect(width: i32, height: i32) -> Rect {
    let w = 540.min(width - 40);
    let h = 340.min(height - 40);
    Rect::new((width - w) / 2, (height - h) / 2, w, h)
}

fn compute_render_signature(width: i32, height: i32, state: &SetupState, mouse: &MouseState) -> u64 {
    let shell = compute_shell(width, height);
    let mut hover_mask = 0u64;
    if hit_rect(mouse.x, mouse.y, shell.probe_btn) {
        hover_mask |= 1 << 0;
    }
    if hit_rect(mouse.x, mouse.y, shell.power_btn) {
        hover_mask |= 1 << 1;
    }
    if hit_rect(mouse.x, mouse.y, shell.back_btn) {
        hover_mask |= 1 << 2;
    }
    if hit_rect(mouse.x, mouse.y, shell.next_btn) {
        hover_mask |= 1 << 3;
    }
    if state.inspector_open {
        let modal = inspector_rect(width, height);
        let close_btn = Rect::new(modal.x + modal.width - 116, modal.y + modal.height - 44, 100, 30);
        if hit_rect(mouse.x, mouse.y, close_btn) {
            hover_mask |= 1 << 4;
        }
    }
    match state.phase {
        StepPhase::Welcome => {
            for i in 0..3 {
                if hit_rect(mouse.x, mouse.y, welcome_lang_card(shell.right_pane, i)) {
                    hover_mask |= 1 << (5 + i);
                }
            }
            for i in 0..2 {
                if hit_rect(mouse.x, mouse.y, welcome_selector_row(shell.right_pane, i)) {
                    hover_mask |= 1 << (8 + i);
                }
            }
        }
        StepPhase::Architecture => {
            for i in 0..2 {
                if hit_rect(mouse.x, mouse.y, arch_mode_card(shell.right_pane, i)) {
                    hover_mask |= 1 << (10 + i);
                }
            }
            if hit_rect(mouse.x, mouse.y, arch_consent_rect(shell.right_pane)) {
                hover_mask |= 1 << 12;
            }
        }
        StepPhase::Storage => {
            for i in 0..state.volume_count.min(5) {
                if hit_rect(mouse.x, mouse.y, storage_row_rect(shell.right_pane, i)) {
                    hover_mask |= 1 << (13 + i);
                }
            }
            for i in 0..4 {
                if hit_rect(mouse.x, mouse.y, storage_action_pill(shell.right_pane, i)) {
                    hover_mask |= 1 << (18 + i);
                }
            }
        }
        _ => {}
    }

    let phase_code = match state.phase {
        StepPhase::Welcome => 1u64,
        StepPhase::Architecture => 2,
        StepPhase::Storage => 3,
        StepPhase::Installing => 4,
        StepPhase::Completed => 5,
    };

    let mut sig = hover_mask;
    sig ^= phase_code << 24;
    sig ^= (state.lang_index as u64) << 28;
    sig ^= (state.tz_index as u64) << 30;
    sig ^= (state.kbd_index as u64) << 32;
    sig ^= (state.doc_scroll as u64) << 34;
    sig ^= (state.profile_mode as u64) << 40;
    sig ^= (state.terms_accepted as u64) << 42;
    sig ^= (state.selected_volume as u64) << 43;
    sig ^= (state.layout_prepared as u64) << 47;
    sig ^= (state.banner_state as u64) << 48;
    sig ^= (state.inspector_open as u64) << 51;
    sig ^= (state.progress as u64) << 52;
    sig ^= ((state.reboot_ticks / 15) as u64) << 60;
    sig
}

fn draw_aero_glass_orb(
    canvas: &mut Canvas<'_>,
    cx: i32,
    cy: i32,
    radius: i32,
    core_col: u32,
    rim_col: u32,
    alpha_cap: u32,
) {
    if radius <= 2 {
        return;
    }
    let r2 = (radius as i64) * (radius as i64);
    let x0 = (cx - radius).max(0);
    let x1 = (cx + radius).min(canvas.width() as i32 - 1);
    let y0 = (cy - radius).max(0);
    let y1 = (cy + radius).min(canvas.height() as i32 - 1);

    for y in y0..=y1 {
        let dy = (y - cy) as i64;
        for x in x0..=x1 {
            let dx = (x - cx) as i64;
            let d2 = dx * dx + dy * dy;
            if d2 >= r2 {
                continue;
            }
            let ratio = (d2 * 255 / r2) as u32;
            let body_col = lerp_color(core_col, rim_col, ratio);
            let edge_boost = (ratio * ratio) >> 8;
            let base_alpha = ((255 - ratio * 150 / 255) * alpha_cap / 256).min(255) as u8;
            canvas.blend_pixel(x, y, body_col, base_alpha);

            if ratio > 195 {
                let rim_a = ((ratio - 195) * 3 * alpha_cap / 256).min(220) as u8;
                canvas.blend_pixel(x, y, rgb(255, 255, 255), rim_a);
            } else if dy < 0 && dx < radius as i64 / 2 {
                let hx = dx + (radius as i64 / 3);
                let hy = dy + (radius as i64 / 3);
                let hd2 = hx * hx + hy * hy;
                let hr2 = (radius as i64 * radius as i64) / 7;
                if hd2 < hr2 {
                    let gloss = ((hr2 - hd2) * 110 / hr2.max(1)) as u32 * alpha_cap / 256;
                    canvas.blend_pixel(x, y, rgb(255, 255, 255), (gloss + edge_boost / 8).min(240) as u8);
                }
            }
        }
    }
}

fn draw_porcelain_aero_backdrop(canvas: &mut Canvas<'_>) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let stride = w as usize;
    let pixels = canvas.pixels_mut();

    for y in 0..h {
        let ty = (y * 255 / h.max(1)) as u32;
        let base = lerp_color(PORCELAIN_TOP, PORCELAIN_BOT, ty);
        let br = ((base >> 16) & 0xFF) as i32;
        let bg = ((base >> 8) & 0xFF) as i32;
        let bb = (base & 0xFF) as i32;
        let row_off = y as usize * stride;

        for x in 0..w {
            let wave_arg = (x * 360 / w.max(1)) + (y * 140 / h.max(1));
            let ribbon_y = (h * 42 / 100) + (wave_1000(wave_arg) * (h / 9) / 1000);
            let dist_ribbon = (y - ribbon_y).abs();
            let ribbon_band = (96 - dist_ribbon).max(0);

            let wave2_arg = (x * 280 / w.max(1)) - (y * 120 / h.max(1)) + 90;
            let ribbon2_y = (h * 72 / 100) + (wave_1000(wave2_arg) * (h / 10) / 1000);
            let dist_ribbon2 = (y - ribbon2_y).abs();
            let mint_band = (84 - dist_ribbon2).max(0);

            let grain = organic_grain(x, y);
            let r = (br - ribbon_band / 12 - mint_band / 10 + grain).clamp(210, 255) as u8;
            let g = (bg + ribbon_band / 18 + mint_band / 12 + grain).clamp(225, 255) as u8;
            let b = (bb + ribbon_band / 8 + mint_band / 14 + grain).clamp(232, 255) as u8;
            pixels[row_off + x as usize] = rgb(r, g, b);
        }
    }

    draw_aero_glass_orb(canvas, w * 18 / 100, h * 24 / 100, h / 5, AQUA_SOFT, AZURE_BRIGHT, 72);
    draw_aero_glass_orb(canvas, w * 82 / 100, h * 76 / 100, h / 4, rgb(172, 224, 255), AQUA_MINT, 68);
    draw_aero_glass_orb(canvas, w * 54 / 100, h * 16 / 100, h / 8, rgb(210, 248, 240), AZURE_BRIGHT, 54);
}

fn blur_region(canvas: &mut Canvas<'_>, rect: Rect, radius: usize) {
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
    let r = radius.min(20);
    let window = (r * 2 + 1) as u32;

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
                pixels[base + i] = ((sr / window) << 16) | ((sg / window) << 8) | (sb / window);
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
                pixels[(y0 as usize + j) * stride + x] = ((sr / window) << 16) | ((sg / window) << 8) | (sb / window);
                let out_p = scratch[j.saturating_sub(r)];
                let in_p = scratch[(j + r + 1).min(rh - 1)];
                sr = sr + ((in_p >> 16) & 0xFF) - ((out_p >> 16) & 0xFF);
                sg = sg + ((in_p >> 8) & 0xFF) - ((out_p >> 8) & 0xFF);
                sb = sb + (in_p & 0xFF) - (out_p & 0xFF);
            }
        }
    }
}

fn corner_cut(dx: i32, dy: i32, radius: i32) -> u8 {
    if radius <= 0 {
        return 255;
    }
    let d2 = dx * dx + dy * dy;
    let r2 = radius * radius;
    let inner = (radius - 1).max(0) * (radius - 1).max(0);
    if d2 <= inner {
        255
    } else if d2 >= r2 {
        0
    } else {
        ((r2 - d2) * 255 / (r2 - inner).max(1)).clamp(0, 255) as u8
    }
}

fn pixel_rounded_coverage(x: i32, y: i32, rect: Rect, radius: i32) -> u8 {
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
        corner_cut(dx, dy, r)
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
        let row_t = (ry * 255 / rect.height.max(1)) as u32;
        let fill = lerp_color(top_col, bot_col, row_t);
        let deep_y = ry >= r && ry < rect.height - r;

        if deep_y && alpha == 255 {
            canvas.fill_rect(rect.x + 1, y, rect.width - 2, 1, fill);
            canvas.fill_rect(rect.x, y, 1, 1, border_col);
            canvas.fill_rect(rect.x + rect.width - 1, y, 1, 1, border_col);
            continue;
        }

        for x in rect.x..(rect.x + rect.width) {
            let cov = pixel_rounded_coverage(x, y, rect, r);
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

fn draw_frosted_card(
    canvas: &mut Canvas<'_>,
    rect: Rect,
    radius: i32,
    blur_rad: usize,
    opacity: u8,
    shadow_strength: u32,
) {
    if shadow_strength > 0 {
        draw_shadow(canvas, rect, radius, 5, 16, rgb(24, 52, 78), shadow_strength);
    }
    if blur_rad > 0 {
        blur_region(canvas, rect, blur_rad);
    }
    draw_rounded_surface(
        canvas,
        rect,
        radius,
        rgb(254, 255, 255),
        rgb(245, 250, 253),
        opacity,
        CARD_BORDER,
        true,
    );

    let glint_w = (rect.width * 56 / 100).max(20);
    let glint_x = rect.x + (rect.width - glint_w) / 2;
    for i in 0..glint_w {
        let dist = (i - glint_w / 2).abs();
        let fade = (255 - dist * 255 / (glint_w / 2).max(1)).clamp(0, 255) as u32;
        let col = lerp_color(AZURE_BRIGHT, AQUA_MINT, (i * 255 / glint_w.max(1)) as u32);
        canvas.blend_pixel(glint_x + i, rect.y, col, (fade * 140 / 255) as u8);
    }
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
    if !enabled {
        draw_rounded_surface(
            canvas,
            rect,
            radius,
            rgb(236, 242, 246),
            rgb(228, 236, 242),
            255,
            rgb(206, 218, 228),
            false,
        );
        let st = text_style(11, 16);
        let tw = text_width(label, &st);
        draw_text_color(
            canvas,
            label,
            rect.x + (rect.width - tw) / 2,
            rect.y + (rect.height - st.cap_height) / 2,
            &st,
            INK_LIGHT,
        );
        return;
    }

    if primary {
        for y in rect.y..(rect.y + rect.height) {
            let ry = y - rect.y;
            let v_t = (ry * 255 / rect.height.max(1)) as u32;
            for x in rect.x..(rect.x + rect.width) {
                let cov = pixel_rounded_coverage(x, y, rect, radius);
                if cov == 0 {
                    continue;
                }
                let rx = x - rect.x;
                let h_t = (rx * 255 / rect.width.max(1)) as u32;
                let base_h = if hovered {
                    lerp_color(AZURE_DEEP, AQUA_DEEP, h_t)
                } else {
                    lerp_color(AZURE_BRIGHT, AQUA_MINT, h_t)
                };
                let shaded = if ry < rect.height / 2 {
                    lerp_color(rgb(255, 255, 255), base_h, 195 + v_t / 5)
                } else {
                    lerp_color(base_h, rgb(12, 58, 96), (v_t - 128) / 4)
                };
                canvas.blend_pixel(x, y, shaded, cov);
            }
        }

        let gloss_rect = Rect::new(rect.x + 6, rect.y + 2, rect.width - 12, rect.height / 2 - 2);
        for y in gloss_rect.y..(gloss_rect.y + gloss_rect.height) {
            let fade = 95 - ((y - gloss_rect.y) * 85 / gloss_rect.height.max(1));
            for x in gloss_rect.x..(gloss_rect.x + gloss_rect.width) {
                if pixel_rounded_coverage(x, y, rect, radius) > 200 {
                    canvas.blend_pixel(x, y, rgb(255, 255, 255), fade.max(0) as u8);
                }
            }
        }

        let st = text_style(11, 16);
        let tw = text_width(label, &st);
        let tx = rect.x + (rect.width - tw) / 2;
        let ty = rect.y + (rect.height - st.cap_height) / 2;
        draw_text_color(canvas, label, tx, ty + 1, &st, rgb(14, 68, 112));
        draw_text_color(canvas, label, tx, ty, &st, rgb(255, 255, 255));
    } else {
        let top_c = if hovered { rgb(244, 251, 255) } else { rgb(252, 254, 255) };
        let bot_c = if hovered { rgb(228, 242, 250) } else { rgb(240, 246, 251) };
        let brd_c = if hovered { AZURE_BRIGHT } else { CARD_BORDER };
        draw_rounded_surface(canvas, rect, radius, top_c, bot_c, 245, brd_c, true);
        let st = text_style(11, 16);
        let tw = text_width(label, &st);
        draw_text_color(
            canvas,
            label,
            rect.x + (rect.width - tw) / 2,
            rect.y + (rect.height - st.cap_height) / 2,
            &st,
            INK_DARK,
        );
    }
}

fn draw_aero_radio(canvas: &mut Canvas<'_>, cx: i32, cy: i32, selected: bool) {
    if selected {
        draw_dot(canvas, cx, cy, 8, AQUA_MINT, 255);
        draw_dot(canvas, cx, cy - 1, 7, AZURE_BRIGHT, 220);
        draw_dot(canvas, cx - 2, cy - 2, 2, rgb(255, 255, 255), 210);
        draw_dot(canvas, cx, cy, 3, rgb(255, 255, 255), 255);
    } else {
        draw_dot(canvas, cx, cy, 8, CARD_BORDER, 255);
        draw_dot(canvas, cx, cy, 6, CARD_WHITE, 255);
    }
}

fn draw_emui_switch(canvas: &mut Canvas<'_>, x: i32, y: i32, active: bool) {
    let pill = Rect::new(x, y, 38, 20);
    let (c_top, c_bot, brd) = if active {
        (AZURE_BRIGHT, AQUA_MINT, AQUA_DEEP)
    } else {
        (rgb(206, 218, 228), rgb(192, 206, 218), rgb(178, 194, 208))
    };
    draw_rounded_surface(canvas, pill, 10, c_top, c_bot, 255, brd, true);
    let knob_x = if active { x + 28 } else { x + 10 };
    draw_dot(canvas, knob_x, y + 10, 7, rgb(255, 255, 255), 255);
    draw_dot(canvas, knob_x - 1, y + 8, 2, rgb(235, 248, 255), 200);
}

fn format_capacity(mb: u64, buf: &mut [u8]) -> usize {
    let mut pos = 0usize;
    let push_u64 = |mut v: u64, out: &mut [u8], p: &mut usize| {
        let mut tmp = [0u8; 20];
        let mut t = 0;
        if v == 0 {
            tmp[0] = b'0';
            t = 1;
        } else {
            while v > 0 {
                tmp[t] = b'0' + (v % 10) as u8;
                v /= 10;
                t += 1;
            }
        }
        for i in (0..t).rev() {
            if *p < out.len() {
                out[*p] = tmp[i];
                *p += 1;
            }
        }
    };

    if mb >= 1024 {
        let gb = mb / 1024;
        let dec = (mb % 1024) * 10 / 1024;
        push_u64(gb, buf, &mut pos);
        buf[pos] = b'.';
        pos += 1;
        push_u64(dec, buf, &mut pos);
        for b in b" GB" {
            buf[pos] = *b;
            pos += 1;
        }
    } else {
        push_u64(mb, buf, &mut pos);
        for b in b" MB" {
            buf[pos] = *b;
            pos += 1;
        }
    }
    pos
}

fn format_number(mut val: u64, buf: &mut [u8]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut t = 0;
    while val > 0 {
        tmp[t] = b'0' + (val % 10) as u8;
        val /= 10;
        t += 1;
    }
    for i in 0..t {
        buf[i] = tmp[t - 1 - i];
    }
    t
}

fn draw_top_navigation_pill(canvas: &mut Canvas<'_>, shell: &ShellLayout, state: &SetupState, mouse: &MouseState) {
    draw_frosted_card(canvas, shell.top_pill, 16, 8, 224, 14);

    let orb_x = shell.top_pill.x + 24;
    let orb_y = shell.top_pill.y + 23;
    draw_aero_glass_orb(canvas, orb_x, orb_y, 11, AQUA_SOFT, AZURE_BRIGHT, 255);
    draw_dot(canvas, orb_x, orb_y, 3, rgb(255, 255, 255), 255);

    let brand_st = text_style(13, 16);
    let sub_st = text_style(10, 16);
    draw_text_color(canvas, "NetizenCore", orb_x + 18, shell.top_pill.y + 11, &brand_st, INK_DARK);
    draw_text_color(canvas, "kurulum asistani", orb_x + 18, shell.top_pill.y + 27, &sub_st, INK_MUTED);

    let active_step: usize = match state.phase {
        StepPhase::Welcome => 0,
        StepPhase::Architecture => 1,
        StepPhase::Storage => 2,
        StepPhase::Installing | StepPhase::Completed => 3,
    };

    let steps = ["1. Bolge", "2. Mimari", "3. Depolama", "4. Kurulum"];
    let step_w = 96;
    let total_steps_w = step_w * 4 + 18;
    let start_x = shell.top_pill.x + (shell.top_pill.width - total_steps_w) / 2 + 12;
    let pill_y = shell.top_pill.y + 10;
    let st_font = text_style(10, 15);

    for (idx, title) in steps.iter().enumerate() {
        let r = Rect::new(start_x + idx as i32 * (step_w + 6), pill_y, step_w, 26);
        if idx == active_step {
            draw_rounded_surface(canvas, r, 13, AZURE_BRIGHT, AQUA_MINT, 255, AZURE_DEEP, true);
            let tw = text_width(title, &st_font);
            draw_text_color(
                canvas,
                title,
                r.x + (r.width - tw) / 2,
                r.y + (r.height - st_font.cap_height) / 2,
                &st_font,
                rgb(255, 255, 255),
            );
        } else if idx < active_step {
            draw_rounded_surface(canvas, r, 13, TINT_SELECTED, rgb(216, 242, 238), 255, AQUA_MINT, false);
            let tw = text_width(title, &st_font);
            draw_text_color(
                canvas,
                title,
                r.x + (r.width - tw) / 2,
                r.y + (r.height - st_font.cap_height) / 2,
                &st_font,
                AQUA_DEEP,
            );
        } else {
            draw_rounded_surface(canvas, r, 13, SUBTLE_FILL, SUBTLE_FILL, 200, CARD_BORDER, false);
            let tw = text_width(title, &st_font);
            draw_text_color(
                canvas,
                title,
                r.x + (r.width - tw) / 2,
                r.y + (r.height - st_font.cap_height) / 2,
                &st_font,
                INK_MUTED,
            );
        }
    }

    let probe_hov = hit_rect(mouse.x, mouse.y, shell.probe_btn);
    draw_aero_pill_button(canvas, shell.probe_btn, "Donanim Tanilama", probe_hov, false, true);
}

fn draw_left_showcase(canvas: &mut Canvas<'_>, pane: Rect, platform: &str, state: &SetupState) {
    draw_rounded_surface(
        canvas,
        pane,
        16,
        rgb(239, 248, 253),
        rgb(225, 242, 246),
        215,
        rgb(198, 222, 235),
        true,
    );

    let cx = pane.x + pane.width / 2;
    let art_cy = pane.y + 88;
    let pulse = wave_1000((state.anim_ticks as i32 * 4) % 360);
    let breath_r = 44 + (pulse * 3 / 1000);

    draw_aero_glass_orb(canvas, cx - 22, art_cy + 10, 26, AQUA_SOFT, AQUA_MINT, 155);
    draw_aero_glass_orb(canvas, cx + 24, art_cy - 12, 22, rgb(180, 224, 255), AZURE_BRIGHT, 150);
    draw_aero_glass_orb(canvas, cx, art_cy, breath_r, rgb(212, 246, 255), AZURE_BRIGHT, 225);

    match state.phase {
        StepPhase::Welcome => {
            for ring in [-14i32, 0, 14] {
                canvas.fill_rect(cx - 26, art_cy + ring, 52, 1, rgb(255, 255, 255));
            }
            canvas.fill_rect(cx, art_cy - 26, 1, 52, rgb(255, 255, 255));
        }
        StepPhase::Architecture => {
            for layer in 0..3 {
                let ly = art_cy - 18 + layer * 16;
                let r = Rect::new(cx - 24 + layer * 2, ly, 48 - layer * 4, 10);
                draw_rounded_surface(canvas, r, 4, rgb(255, 255, 255), TINT_SELECTED, 240, AZURE_BRIGHT, false);
            }
        }
        StepPhase::Storage => {
            for platter in 0..3 {
                let py = art_cy - 18 + platter * 14;
                let pr = Rect::new(cx - 24, py, 48, 10);
                draw_rounded_surface(canvas, pr, 5, rgb(255, 255, 255), AQUA_SOFT, 245, AQUA_DEEP, false);
                draw_dot(canvas, cx + 14, py + 5, 2, AZURE_BRIGHT, 255);
            }
        }
        StepPhase::Installing | StepPhase::Completed => {
            let ck = rgb(255, 255, 255);
            for s in 0..6 {
                canvas.fill_rect(cx - 12 + s, art_cy + 2 + s, 3, 3, ck);
            }
            for s in 0..12 {
                canvas.fill_rect(cx - 6 + s, art_cy + 8 - s, 3, 3, ck);
            }
        }
    }

    let title_st = text_style(14, 16);
    let body_st = text_style(10, 15);
    let cap_st = text_style(9, 18);

    let (headline, line1, line2) = match state.phase {
        StepPhase::Welcome => (
            "Temiz Baslangic",
            "Sade ve moduler .efra tabanli",
            "sifirdan tasarlanan calisma ortami.",
        ),
        StepPhase::Architecture => (
            "Hibrit Cekirdek Mimarisi",
            "EROFS salt okunur sistem agaci,",
            "exFAT /data ve .efra surucu tabani.",
        ),
        StepPhase::Storage => (
            "Akilli Disk Yerlesimi",
            "Tek tikla MBR, Stage2, Cekirdek",
            "ve VFS bolumlerini yapilandirin.",
        ),
        StepPhase::Installing => (
            "Sistem Diske Yaziliyor",
            "Sektorler ve .efra suruculeri",
            "dogrulanarak diske aktariliyor.",
        ),
        StepPhase::Completed => (
            "NetizenCore Hazir",
            "Sabit disk onyukleme kaydi",
            "basariyla etkinlestirildi.",
        ),
    };

    let hw = text_width(headline, &title_st);
    draw_text_color(canvas, headline, cx - hw / 2, pane.y + 154, &title_st, INK_DARK);
    let l1w = text_width(line1, &body_st);
    draw_text_color(canvas, line1, cx - l1w / 2, pane.y + 180, &body_st, INK_BODY);
    let l2w = text_width(line2, &body_st);
    draw_text_color(canvas, line2, cx - l2w / 2, pane.y + 198, &body_st, INK_BODY);

    let spec_card = Rect::new(pane.x + 14, pane.y + pane.height - 134, pane.width - 28, 120);
    draw_rounded_surface(canvas, spec_card, 12, CARD_WHITE, SUBTLE_FILL, 235, CARD_BORDER, true);

    draw_text_color(canvas, "SISTEM VE .EFRA OZETI", spec_card.x + 14, spec_card.y + 12, &cap_st, AZURE_DEEP);
    draw_text_color(canvas, "Platform:", spec_card.x + 14, spec_card.y + 34, &body_st, INK_MUTED);
    draw_text_color(canvas, platform, spec_card.x + 104, spec_card.y + 34, &body_st, INK_DARK);

    draw_text_color(canvas, "Surucu Tabani:", spec_card.x + 14, spec_card.y + 54, &body_st, INK_MUTED);
    draw_text_color(canvas, "Generic .efra + PAT WC", spec_card.x + 104, spec_card.y + 54, &body_st, AQUA_DEEP);

    draw_text_color(canvas, "Hedef Disk:", spec_card.x + 14, spec_card.y + 74, &body_st, INK_MUTED);
    let disk_model = if state.selected_disk < state.disk_count {
        state.disks[state.selected_disk].model_str()
    } else if state.disk_count > 0 {
        state.disks[0].model_str()
    } else {
        "Sanal ATA Disk"
    };
    draw_text_color(canvas, disk_model, spec_card.x + 104, spec_card.y + 74, &body_st, INK_DARK);

    draw_text_color(canvas, "Arayuz Motoru:", spec_card.x + 14, spec_card.y + 94, &body_st, INK_MUTED);
    draw_text_color(canvas, "nc-gfx / 64-Bit Aero", spec_card.x + 104, spec_card.y + 94, &body_st, AZURE_BRIGHT);
}

fn draw_step_welcome(canvas: &mut Canvas<'_>, pane: Rect, state: &SetupState, mouse: &MouseState) {
    let h_st = text_style(17, 15);
    let sub_st = text_style(10, 15);
    draw_text_color(canvas, "Bolge, dil ve klavye tercihleri", pane.x + 14, pane.y + 8, &h_st, INK_DARK);
    draw_text_color(
        canvas,
        "Kurulum sihirbazinin ve masaustu ortaminin varsayilan dilini secin.",
        pane.x + 14,
        pane.y + 32,
        &sub_st,
        INK_MUTED,
    );

    let title_st = text_style(11, 15);
    let desc_st = text_style(9, 15);

    for (idx, (lang_name, lang_desc)) in LANGUAGES.iter().enumerate() {
        let card = welcome_lang_card(pane, idx);
        let selected = state.lang_index == idx;
        let hovered = hit_rect(mouse.x, mouse.y, card);
        let (top_c, bot_c, brd_c) = if selected {
            (TINT_SELECTED, rgb(218, 241, 250), AZURE_BRIGHT)
        } else if hovered {
            (SUBTLE_HOVER, SUBTLE_FILL, rgb(176, 208, 228))
        } else {
            (CARD_WHITE, SUBTLE_FILL, CARD_BORDER)
        };
        draw_rounded_surface(canvas, card, 10, top_c, bot_c, 250, brd_c, true);
        draw_aero_radio(canvas, card.x + 20, card.y + card.height / 2, selected);
        draw_text_color(canvas, lang_name, card.x + 38, card.y + 9, &title_st, INK_DARK);
        draw_text_color(canvas, lang_desc, card.x + 38, card.y + 25, &desc_st, INK_MUTED);
    }

    let tz_row = welcome_selector_row(pane, 0);
    let kb_row = welcome_selector_row(pane, 1);

    for (row_rect, label, val) in [
        (tz_row, "Saat Dilimi:", TIMEZONES[state.tz_index]),
        (kb_row, "Klavye Duzeni:", KEYBOARDS[state.kbd_index].0),
    ] {
        let hov = hit_rect(mouse.x, mouse.y, row_rect);
        let brd = if hov { AQUA_MINT } else { CARD_BORDER };
        draw_rounded_surface(canvas, row_rect, 9, CARD_WHITE, SUBTLE_FILL, 250, brd, true);
        draw_text_color(canvas, label, row_rect.x + 14, row_rect.y + 12, &sub_st, INK_MUTED);
        draw_text_color(canvas, val, row_rect.x + 118, row_rect.y + 12, &title_st, INK_DARK);

        let badge = Rect::new(row_rect.x + row_rect.width - 74, row_rect.y + 6, 64, 24);
        draw_rounded_surface(canvas, badge, 12, TINT_SELECTED, rgb(215, 242, 238), 255, AQUA_MINT, false);
        draw_text_color(canvas, "Degistir", badge.x + 11, badge.y + 7, &desc_st, AQUA_DEEP);
    }

    let preview_bar = Rect::new(pane.x + 14, kb_row.y + 44, pane.width - 28, 30);
    draw_rounded_surface(canvas, preview_bar, 8, SUBTLE_FILL, rgb(232, 242, 248), 240, CARD_BORDER, false);
    draw_text_color(canvas, "Tus Haritasi:", preview_bar.x + 12, preview_bar.y + 10, &desc_st, AZURE_DEEP);
    draw_text_color(
        canvas,
        KEYBOARDS[state.kbd_index].1,
        preview_bar.x + 92,
        preview_bar.y + 10,
        &desc_st,
        INK_BODY,
    );
}

fn draw_step_architecture(canvas: &mut Canvas<'_>, pane: Rect, state: &SetupState, mouse: &MouseState) {
    let h_st = text_style(17, 15);
    let sub_st = text_style(10, 15);
    draw_text_color(canvas, "Sistem mimarisi ve .efra surucu katmani", pane.x + 14, pane.y + 8, &h_st, INK_DARK);
    draw_text_color(
        canvas,
        "NetizenCore dosya sistemi yapisini ve generic .efra surucu profilini secin.",
        pane.x + 14,
        pane.y + 32,
        &sub_st,
        INK_MUTED,
    );

    let doc_r = arch_doc_rect(pane);
    draw_rounded_surface(canvas, doc_r, 12, CARD_WHITE, rgb(247, 251, 254), 255, CARD_BORDER, true);

    let line_st = text_style(10, 15);
    let visible = 7usize;
    for i in 0..visible {
        let idx = state.doc_scroll + i;
        if idx < ARCHITECTURE_NOTES.len() {
            let ly = doc_r.y + 12 + i as i32 * 19;
            let is_header = idx == 0 || idx == 2 || idx == 7 || idx == 15 || idx == 20;
            let color = if is_header { AZURE_DEEP } else { INK_BODY };
            draw_text_color(canvas, ARCHITECTURE_NOTES[idx], doc_r.x + 14, ly, &line_st, color);
        }
    }

    let track_x = doc_r.x + doc_r.width - 16;
    let track_y = doc_r.y + 10;
    let track_h = doc_r.height - 20;
    canvas.fill_rect(track_x, track_y, 4, track_h, rgb(222, 232, 240));
    let max_sc = ARCHITECTURE_NOTES.len().saturating_sub(visible).max(1);
    let thumb_h = 34;
    let thumb_y = track_y + ((track_h - thumb_h) * state.doc_scroll.min(max_sc) as i32 / max_sc as i32);
    let thumb_r = Rect::new(track_x - 2, thumb_y, 8, thumb_h);
    draw_rounded_surface(canvas, thumb_r, 4, AZURE_BRIGHT, AQUA_MINT, 255, AZURE_DEEP, false);

    let title_st = text_style(11, 15);
    let desc_st = text_style(9, 15);
    let modes = [
        ("Evrensel Generic .efra", "QEMU + VBox + Gercek PC otomatik uyum"),
        ("Maksimum Performans", "PAT Write-Combining + 64-Bit Burst"),
    ];
    for (idx, (m_title, m_desc)) in modes.iter().enumerate() {
        let mr = arch_mode_card(pane, idx);
        let sel = state.profile_mode == idx;
        let hov = hit_rect(mouse.x, mouse.y, mr);
        let (top_c, bot_c, brd_c) = if sel {
            (TINT_SELECTED, rgb(216, 242, 246), AQUA_MINT)
        } else if hov {
            (SUBTLE_HOVER, SUBTLE_FILL, AZURE_BRIGHT)
        } else {
            (CARD_WHITE, SUBTLE_FILL, CARD_BORDER)
        };
        draw_rounded_surface(canvas, mr, 10, top_c, bot_c, 250, brd_c, true);
        draw_aero_radio(canvas, mr.x + 18, mr.y + mr.height / 2, sel);
        draw_text_color(canvas, m_title, mr.x + 34, mr.y + 11, &title_st, INK_DARK);
        draw_text_color(canvas, m_desc, mr.x + 34, mr.y + 29, &desc_st, INK_MUTED);
    }

    let consent = arch_consent_rect(pane);
    let c_hov = hit_rect(mouse.x, mouse.y, consent);
    let c_brd = if state.terms_accepted {
        AQUA_MINT
    } else if c_hov {
        AZURE_BRIGHT
    } else {
        CARD_BORDER
    };
    draw_rounded_surface(canvas, consent, 10, CARD_WHITE, SUBTLE_FILL, 250, c_brd, true);
    draw_emui_switch(canvas, consent.x + 12, consent.y + 8, state.terms_accepted);
    draw_text_color(
        canvas,
        "Mimari kosullari okudum, disk ve .efra surucu kurulumunu onayliyorum",
        consent.x + 60,
        consent.y + 12,
        &line_st,
        INK_DARK,
    );
}

fn draw_step_storage(canvas: &mut Canvas<'_>, pane: Rect, state: &SetupState, mouse: &MouseState) {
    let h_st = text_style(17, 15);
    let sub_st = text_style(10, 15);
    draw_text_color(canvas, "Hedef disk ve bolum yapisi", pane.x + 14, pane.y + 8, &h_st, INK_DARK);
    draw_text_color(
        canvas,
        "Kurulumun yapilacagi depolama birimini secin veya bolumleri yonetin.",
        pane.x + 14,
        pane.y + 32,
        &sub_st,
        INK_MUTED,
    );

    let row_st = text_style(10, 15);
    let small_st = text_style(9, 15);

    for i in 0..state.volume_count.min(5) {
        let vol = &state.volumes[i];
        let r = storage_row_rect(pane, i);
        let sel = state.selected_volume == i;
        let hov = hit_rect(mouse.x, mouse.y, r);
        let (top_c, bot_c, brd_c) = if sel {
            (TINT_SELECTED, rgb(214, 240, 250), AZURE_BRIGHT)
        } else if hov {
            (SUBTLE_HOVER, SUBTLE_FILL, rgb(172, 206, 228))
        } else {
            (CARD_WHITE, SUBTLE_FILL, CARD_BORDER)
        };
        draw_rounded_surface(canvas, r, 8, top_c, bot_c, 250, brd_c, true);

        let badge_col = match vol.role {
            VolumeRole::Unallocated => INK_MUTED,
            VolumeRole::BootCore => AZURE_DEEP,
            VolumeRole::SystemErofs => AQUA_MINT,
            VolumeRole::SecurityVault => rgb(98, 112, 228),
            VolumeRole::DataExfat => AZURE_BRIGHT,
            VolumeRole::OpticalIso => rgb(218, 146, 36),
        };
        draw_dot(canvas, r.x + 16, r.y + r.height / 2, 5, badge_col, 255);

        let (title, fs_tag) = match vol.role {
            VolumeRole::Unallocated => ("Sabit Disk Ayrilmamis Alan", "Ham Alan"),
            VolumeRole::BootCore => ("Bolum 1: Onyukleme ve Cekirdek", "RAW / Stage2"),
            VolumeRole::SystemErofs => ("Bolum 2: Sistem (/system)", "EROFS Salt Okunur"),
            VolumeRole::SecurityVault => ("Bolum 3: Guvenlik Kasasi", "Anahtar Deposu"),
            VolumeRole::DataExfat => ("Bolum 4: Veri ve .efra (/data)", "exFAT Birincil"),
            VolumeRole::OpticalIso => ("Optik Surucu: Kurulum Medyasi", "ATAPI ISO9660"),
        };
        draw_text_color(canvas, title, r.x + 30, r.y + 10, &row_st, INK_DARK);
        draw_text_color(canvas, fs_tag, r.x + 250, r.y + 11, &small_st, badge_col);

        let mut cap_buf = [0u8; 24];
        let cap_len = format_capacity(vol.size_mb, &mut cap_buf);
        if let Ok(cap_str) = core::str::from_utf8(&cap_buf[..cap_len]) {
            let cw = text_width(cap_str, &row_st);
            draw_text_color(canvas, cap_str, r.x + r.width - cw - 14, r.y + 10, &row_st, INK_BODY);
        }
    }

    let actions = ["Taramayi Yenile", "Tabloyu Sifirla", "Otomatik Bolumle", "Bicimlendir"];
    for (idx, act_label) in actions.iter().enumerate() {
        let ar = storage_action_pill(pane, idx);
        let hov = hit_rect(mouse.x, mouse.y, ar);
        draw_aero_pill_button(canvas, ar, act_label, hov, false, true);
    }

    let bar_rect = Rect::new(pane.x + 14, pane.y + 280, pane.width - 28, 34);
    draw_rounded_surface(canvas, bar_rect, 9, CARD_WHITE, SUBTLE_FILL, 245, CARD_BORDER, true);

    let seg_y = bar_rect.y + 6;
    let total_w = bar_rect.width - 20;
    let sx = bar_rect.x + 10;
    if state.layout_prepared {
        let w_boot = total_w * 8 / 100;
        let w_sys = total_w * 30 / 100;
        let w_sec = total_w * 12 / 100;
        let w_dat = total_w - w_boot - w_sys - w_sec;
        canvas.fill_rect(sx, seg_y, w_boot - 2, 8, AZURE_DEEP);
        canvas.fill_rect(sx + w_boot, seg_y, w_sys - 2, 8, AQUA_MINT);
        canvas.fill_rect(sx + w_boot + w_sys, seg_y, w_sec - 2, 8, rgb(110, 124, 232));
        canvas.fill_rect(sx + w_boot + w_sys + w_sec, seg_y, w_dat, 8, AZURE_BRIGHT);
    } else {
        canvas.fill_rect(sx, seg_y, total_w, 8, rgb(186, 198, 210));
    }

    let status_note = if !state.can_continue() {
        "Optik ISO surucusune kurulum yapilamaz; lutfen sabit disk uzerinden bir bolum secin."
    } else {
        match state.banner_state {
            1 => "ATA PIO kanallari (0..3) tarandi, disk geometrisi ve .efra baglantisi dogrulandi.",
            2 => "Bolum tablosu temizlendi; kurulum basladiginda otomatik olusturulacak.",
            3 => "4 bolumlu standart NetizenCore + /data/drivers (.efra) yerlesimi hazirlandi.",
            4 => "EROFS /system ve exFAT /data bolumleri kurulum icin bicimlendirildi.",
            _ => "Secilen disk uzerinde MBR, Cekirdek, /system, /data ve .efra suruculeri hazir.",
        }
    };
    draw_text_color(canvas, status_note, sx, bar_rect.y + 19, &small_st, INK_MUTED);
}

fn draw_step_installing(canvas: &mut Canvas<'_>, pane: Rect, state: &SetupState) {
    let h_st = text_style(17, 15);
    let sub_st = text_style(10, 15);
    draw_text_color(canvas, "NetizenCore diske kuruluyor...", pane.x + 14, pane.y + 8, &h_st, INK_DARK);
    draw_text_color(
        canvas,
        "Lutfen bekleyin, sistem bilesenleri ve generic .efra surucu paketleri yaziliyor.",
        pane.x + 14,
        pane.y + 32,
        &sub_st,
        INK_MUTED,
    );

    let pct = (state.progress * 100 / 256).min(100);
    let prog_box = Rect::new(pane.x + 14, pane.y + 60, pane.width - 28, 54);
    draw_rounded_surface(canvas, prog_box, 12, CARD_WHITE, SUBTLE_FILL, 250, CARD_BORDER, true);

    let pct_st = text_style(15, 15);
    let mut p_buf = [0u8; 12];
    p_buf[0] = b'%';
    let plen = 1 + format_number(pct as u64, &mut p_buf[1..]);
    if let Ok(ps) = core::str::from_utf8(&p_buf[..plen]) {
        draw_text_color(canvas, ps, prog_box.x + 16, prog_box.y + 10, &pct_st, AZURE_DEEP);
    }

    let track = Rect::new(prog_box.x + 16, prog_box.y + 32, prog_box.width - 32, 12);
    draw_rounded_surface(canvas, track, 6, SUBTLE_FILL, rgb(222, 234, 242), 255, CARD_BORDER, false);

    let fill_w = (track.width as u32 * state.progress / 256) as i32;
    if fill_w > 8 {
        let fill_r = Rect::new(track.x, track.y, fill_w, track.height);
        for y in fill_r.y..(fill_r.y + fill_r.height) {
            for x in fill_r.x..(fill_r.x + fill_r.width) {
                if pixel_rounded_coverage(x, y, fill_r, 6) > 120 {
                    let t = ((x - fill_r.x) * 255 / track.width.max(1)) as u32;
                    let col = lerp_color(AZURE_BRIGHT, AQUA_MINT, t);
                    canvas.fill_rect(x, y, 1, 1, col);
                }
            }
        }
        canvas.fill_rect(fill_r.x + 4, fill_r.y + 2, (fill_w - 8).max(1), 2, rgb(215, 250, 255));
    }

    let current_stage = match pct {
        0..=19 => 0,
        20..=44 => 1,
        45..=72 => 2,
        73..=91 => 3,
        _ => 4,
    };

    let milestones = [
        "Hibrit MBR ve stage2 onyukleme sektorleri yaziliyor (LBA 0 & 2048)",
        "64-bit NetizenCore cekirdek imaji Bolum 1'e aktariliyor (LBA 2056)",
        "EROFS /system agaci ve yerlesik .ntzc paketleri olusturuluyor",
        "exFAT /data/drivers altina 6 adet Generic .efra surucu paketi kuruluyor",
        "ATA PIO donanim onbellegi (0xE7 Flush) esitleniyor ve dogrulaniyor",
    ];

    let item_st = text_style(10, 15);
    for (idx, text) in milestones.iter().enumerate() {
        let iy = pane.y + 130 + idx as i32 * 30;
        let cx = pane.x + 26;
        let cy = iy + 6;
        if idx < current_stage {
            draw_dot(canvas, cx, cy, 7, AQUA_MINT, 255);
            draw_dot(canvas, cx, cy, 3, rgb(255, 255, 255), 255);
            draw_text_color(canvas, text, pane.x + 44, iy, &item_st, INK_BODY);
        } else if idx == current_stage {
            draw_dot(canvas, cx, cy, 8, AZURE_BRIGHT, 255);
            draw_dot(canvas, cx - 2, cy - 2, 2, rgb(255, 255, 255), 220);
            draw_text_color(canvas, text, pane.x + 44, iy, &item_st, INK_DARK);
        } else {
            draw_dot(canvas, cx, cy, 6, CARD_BORDER, 255);
            draw_text_color(canvas, text, pane.x + 44, iy, &item_st, INK_LIGHT);
        }
    }

    let telemetry = Rect::new(pane.x + 14, pane.y + 284, pane.width - 28, 28);
    draw_rounded_surface(canvas, telemetry, 8, TINT_SELECTED, rgb(222, 244, 242), 240, AQUA_MINT, false);
    let tel_msg = match current_stage {
        0 => "ATA PIO -> LBA 0..2055 blok yazimi tamamlandi",
        1 => "ATA PIO -> Cekirdek ikilisi dogrusal bloklara aktariliyor",
        2 => "VFS -> /system/app/{settings,system,stopwatch}.ntzc dugumleri yazildi",
        3 => "EfraCompiler -> /data/drivers/{generic-display,vbox-svga,generic-storage}.efra yazildi",
        _ => "Kurulum tamamlandi -> Yeniden baslatma asamasi hazirlaniyor",
    };
    let small_st = text_style(9, 15);
    draw_text_color(canvas, tel_msg, telemetry.x + 12, telemetry.y + 9, &small_st, AQUA_DEEP);
}

fn draw_step_completed(canvas: &mut Canvas<'_>, pane: Rect, state: &SetupState) {
    let h_st = text_style(17, 15);
    let sub_st = text_style(10, 15);
    draw_text_color(canvas, "Kurulum basariyla tamamlandi!", pane.x + 14, pane.y + 8, &h_st, INK_DARK);
    draw_text_color(
        canvas,
        "NetizenCore ve .efra surucu tabani sabit diskinize kuruldu.",
        pane.x + 14,
        pane.y + 32,
        &sub_st,
        INK_MUTED,
    );

    let card = Rect::new(pane.x + 14, pane.y + 68, pane.width - 28, 176);
    draw_rounded_surface(canvas, card, 14, CARD_WHITE, SUBTLE_FILL, 250, AQUA_MINT, true);

    let orb_cx = card.x + card.width / 2;
    let orb_cy = card.y + 52;
    draw_aero_glass_orb(canvas, orb_cx, orb_cy, 28, AQUA_SOFT, AQUA_MINT, 255);
    for s in 0..5 {
        canvas.fill_rect(orb_cx - 10 + s, orb_cy + 1 + s, 3, 3, rgb(255, 255, 255));
    }
    for s in 0..10 {
        canvas.fill_rect(orb_cx - 5 + s, orb_cy + 6 - s, 3, 3, rgb(255, 255, 255));
    }

    let secs = (10u32).saturating_sub(state.reboot_ticks / 60);
    let mut msg = [0u8; 48];
    let pfx = b"Sistem ";
    msg[..pfx.len()].copy_from_slice(pfx);
    let mut len = pfx.len();
    len += format_number(secs as u64, &mut msg[len..]);
    let sfx = b" saniye icinde yeniden baslatilacak...";
    msg[len..len + sfx.len()].copy_from_slice(sfx);
    len += sfx.len();

    let mid_st = text_style(12, 15);
    if let Ok(s) = core::str::from_utf8(&msg[..len]) {
        let sw = text_width(s, &mid_st);
        draw_text_color(canvas, s, orb_cx - sw / 2, card.y + 98, &mid_st, INK_DARK);
    }

    let bar = Rect::new(card.x + 36, card.y + 128, card.width - 72, 10);
    draw_rounded_surface(canvas, bar, 5, SUBTLE_FILL, rgb(220, 232, 240), 255, CARD_BORDER, false);
    let fill_w = (bar.width as u32 * state.reboot_ticks.min(600) / 600) as i32;
    if fill_w > 6 {
        let fr = Rect::new(bar.x, bar.y, fill_w, bar.height);
        draw_rounded_surface(canvas, fr, 5, AZURE_BRIGHT, AQUA_MINT, 255, AQUA_DEEP, false);
    }

    let hint_st = text_style(9, 15);
    let hint = "Ipucu: Ayarlar > Aygitlar (.efra) sekmesinden suruculeri derleyip guncelleyebilirsiniz.";
    let hw = text_width(hint, &hint_st);
    draw_text_color(canvas, hint, orb_cx - hw / 2, card.y + 150, &hint_st, INK_MUTED);
}

fn draw_hardware_inspector(
    canvas: &mut Canvas<'_>,
    width: i32,
    height: i32,
    platform: &str,
    state: &SetupState,
    mouse: &MouseState,
) {
    let modal = inspector_rect(width, height);
    draw_frosted_card(canvas, modal, 18, 12, 238, 42);

    let h_st = text_style(15, 15);
    let sub_st = text_style(10, 15);
    draw_text_color(
        canvas,
        "Donanim, Depolama ve .efra Surucu Tanilama Raporu",
        modal.x + 22,
        modal.y + 18,
        &h_st,
        INK_DARK,
    );
    let env_label = nc_driver::global().environment.label();
    draw_text_color(
        canvas,
        env_label,
        modal.x + 22,
        modal.y + 40,
        &sub_st,
        AZURE_DEEP,
    );

    let list_box = Rect::new(modal.x + 20, modal.y + 64, modal.width - 40, modal.height - 122);
    draw_rounded_surface(canvas, list_box, 12, SUBTLE_FILL, rgb(233, 244, 249), 250, CARD_BORDER, true);

    draw_text_color(canvas, "Aktif Profil:", list_box.x + 16, list_box.y + 12, &sub_st, INK_MUTED);
    draw_text_color(canvas, platform, list_box.x + 110, list_box.y + 12, &sub_st, AZURE_DEEP);
    draw_text_color(canvas, "PAT Write-Combining: Aktif", list_box.x + 240, list_box.y + 12, &sub_st, AQUA_DEEP);

    for i in 0..state.disk_count.min(3) {
        let d = &state.disks[i];
        let ry = list_box.y + 36 + i as i32 * 28;
        let row_r = Rect::new(list_box.x + 12, ry, list_box.width - 24, 24);
        draw_rounded_surface(canvas, row_r, 7, CARD_WHITE, CARD_WHITE, 255, CARD_BORDER, false);
        draw_dot(canvas, row_r.x + 12, row_r.y + 12, 4, AQUA_MINT, 255);
        draw_text_color(canvas, d.type_label(), row_r.x + 24, row_r.y + 7, &sub_st, AZURE_BRIGHT);
        draw_text_color(canvas, d.model_str(), row_r.x + 68, row_r.y + 7, &sub_st, INK_DARK);

        let mut mb_buf = [0u8; 24];
        let mb_len = format_capacity(d.size_mb(), &mut mb_buf);
        if let Ok(ms) = core::str::from_utf8(&mb_buf[..mb_len]) {
            let mw = text_width(ms, &sub_st);
            draw_text_color(canvas, ms, row_r.x + row_r.width - mw - 12, row_r.y + 7, &sub_st, INK_BODY);
        }
    }

    let reg = nc_driver::global();
    for (idx, dev) in reg.devices.iter().take(2).enumerate() {
        let ry = list_box.y + 124 + idx as i32 * 28;
        if ry + 24 <= list_box.y + list_box.height - 6 {
            let row_r = Rect::new(list_box.x + 12, ry, list_box.width - 24, 24);
            draw_rounded_surface(canvas, row_r, 7, TINT_SELECTED, CARD_WHITE, 255, AQUA_MINT, false);
            draw_dot(canvas, row_r.x + 12, row_r.y + 12, 4, AZURE_BRIGHT, 255);
            draw_text_color(canvas, &dev.hw_name, row_r.x + 24, row_r.y + 7, &sub_st, INK_DARK);
            let fw = text_width(&dev.driver_file, &sub_st);
            draw_text_color(
                canvas,
                &dev.driver_file,
                row_r.x + row_r.width - fw - 12,
                row_r.y + 7,
                &sub_st,
                AQUA_DEEP,
            );
        }
    }

    let close_btn = Rect::new(modal.x + modal.width - 116, modal.y + modal.height - 44, 100, 30);
    let hov = hit_rect(mouse.x, mouse.y, close_btn);
    draw_aero_pill_button(canvas, close_btn, "Kapat", hov, true, true);
}

pub fn draw(canvas: &mut Canvas<'_>, platform: &str, state: &SetupState, mouse: &MouseState) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;

    let sig = compute_render_signature(width, height, state, mouse);
    if !state.changed && LAST_RENDER_HASH.load(Ordering::Relaxed) == sig {
        return;
    }
    LAST_RENDER_HASH.store(sig, Ordering::Relaxed);

    let shell = compute_shell(width, height);

    draw_porcelain_aero_backdrop(canvas);
    draw_top_navigation_pill(canvas, &shell, state, mouse);

    draw_frosted_card(canvas, shell.main_card, 20, 10, 226, 22);
    draw_left_showcase(canvas, shell.left_pane, platform, state);

    match state.phase {
        StepPhase::Welcome => draw_step_welcome(canvas, shell.right_pane, state, mouse),
        StepPhase::Architecture => draw_step_architecture(canvas, shell.right_pane, state, mouse),
        StepPhase::Storage => draw_step_storage(canvas, shell.right_pane, state, mouse),
        StepPhase::Installing => draw_step_installing(canvas, shell.right_pane, state),
        StepPhase::Completed => draw_step_completed(canvas, shell.right_pane, state),
    }

    draw_frosted_card(canvas, shell.bottom_bar, 16, 8, 224, 14);

    if state.phase != StepPhase::Installing {
        let p_hov = hit_rect(mouse.x, mouse.y, shell.power_btn);
        draw_aero_pill_button(canvas, shell.power_btn, "Yeniden Baslat", p_hov, false, true);
    } else {
        let st = text_style(10, 15);
        draw_text_color(
            canvas,
            "Disk ve .efra surucu yazma islemi surerken gucu kesmeyin...",
            shell.bottom_bar.x + 18,
            shell.bottom_bar.y + 21,
            &st,
            INK_MUTED,
        );
    }

    if state.phase == StepPhase::Architecture || state.phase == StepPhase::Storage {
        let b_hov = hit_rect(mouse.x, mouse.y, shell.back_btn);
        draw_aero_pill_button(canvas, shell.back_btn, "Geri", b_hov, false, true);
    }

    if state.phase != StepPhase::Installing {
        let next_label = match state.phase {
            StepPhase::Welcome => "Devam Et",
            StepPhase::Architecture => "Onayla ve Gec",
            StepPhase::Storage => "Kuruluma Basla",
            StepPhase::Installing => "",
            StepPhase::Completed => "Simdi Yeniden Baslat",
        };
        let n_hov = hit_rect(mouse.x, mouse.y, shell.next_btn);
        draw_aero_pill_button(
            canvas,
            shell.next_btn,
            next_label,
            n_hov,
            true,
            state.can_continue(),
        );
    }

    if state.inspector_open {
        draw_hardware_inspector(canvas, width, height, platform, state, mouse);
    }
}

#![no_std]

use nc_arch::{inb, inw, outb, outw};
use nc_hal::{BlockDevice, BlockDeviceInfo, DeviceType};

pub const MAX_DISKS: usize = 4;

pub mod partition;
pub use partition::{parse_mbr_partitions, AtaPartition, PartitionInfo};

#[derive(Clone, Copy)]
pub struct AtaDevice {
    pub channel: u8,
    pub info: BlockDeviceInfo,
}

impl AtaDevice {
    pub const EMPTY: AtaDevice = AtaDevice {
        channel: 0,
        info: BlockDeviceInfo::EMPTY,
    };

    pub fn read_sector(&mut self, lba: u64, buf: &mut [u8]) -> Result<(), ()> {
        if lba >= 0x1000_0000 || buf.len() < 512 || self.info.device_type == DeviceType::OpticalDisc {
            return Err(());
        }
        let io_base = if self.channel < 2 { 0x01F0 } else { 0x0170 };
        let ctrl_base = if self.channel < 2 { 0x03F6 } else { 0x0376 };
        let drive_bit = self.channel & 1;

        unsafe {
            outb(io_base + 6, 0xE0 | (drive_bit << 4) | (((lba >> 24) & 0x0F) as u8));
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);
            outb(io_base + 2, 1);
            outb(io_base + 3, lba as u8);
            outb(io_base + 4, (lba >> 8) as u8);
            outb(io_base + 5, (lba >> 16) as u8);
            outb(io_base + 7, 0x20);
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);

            let mut spins = 0u32;
            while (inb(io_base + 7) & 0x80) != 0 {
                spins += 1;
                if spins > 100_000 {
                    return Err(());
                }
            }
            let status = inb(io_base + 7);
            if status & 0x01 != 0 || status & 0x08 == 0 {
                return Err(());
            }
            for i in 0..256 {
                let word = inw(io_base);
                buf[i * 2] = word as u8;
                buf[i * 2 + 1] = (word >> 8) as u8;
            }
        }
        Ok(())
    }

    pub fn write_sector(&mut self, lba: u64, buf: &[u8]) -> Result<(), ()> {
        if lba >= 0x1000_0000 || buf.len() < 512 || self.info.device_type == DeviceType::OpticalDisc {
            return Err(());
        }
        let io_base = if self.channel < 2 { 0x01F0 } else { 0x0170 };
        let ctrl_base = if self.channel < 2 { 0x03F6 } else { 0x0376 };
        let drive_bit = self.channel & 1;

        unsafe {
            outb(io_base + 6, 0xE0 | (drive_bit << 4) | (((lba >> 24) & 0x0F) as u8));
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);
            outb(io_base + 2, 1);
            outb(io_base + 3, lba as u8);
            outb(io_base + 4, (lba >> 8) as u8);
            outb(io_base + 5, (lba >> 16) as u8);
            outb(io_base + 7, 0x30);
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);
            inb(ctrl_base);

            let mut spins = 0u32;
            while (inb(io_base + 7) & 0x80) != 0 {
                spins += 1;
                if spins > 100_000 {
                    return Err(());
                }
            }
            let status = inb(io_base + 7);
            if status & 0x01 != 0 || status & 0x08 == 0 {
                return Err(());
            }
            for i in 0..256 {
                let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
                outw(io_base, word);
            }
            let mut write_spins = 0u32;
            while (inb(io_base + 7) & 0x80) != 0 {
                write_spins += 1;
                if write_spins > 100_000 {
                    return Err(());
                }
            }
            if inb(io_base + 7) & 0x01 != 0 {
                return Err(());
            }
        }
        Ok(())
    }

    pub fn flush(&mut self) {
        let io_base = if self.channel < 2 { 0x01F0 } else { 0x0170 };
        let drive_bit = self.channel & 1;
        unsafe {
            outb(io_base + 6, 0xE0 | (drive_bit << 4));
            outb(io_base + 7, 0xE7);
            let mut spins = 0u32;
            while (inb(io_base + 7) & 0x80) != 0 && spins < 100_000 {
                spins += 1;
            }
        }
    }

}

impl BlockDevice for AtaDevice {
    fn info(&self) -> BlockDeviceInfo {
        self.info
    }

    fn read_blocks(&mut self, lba: u64, count: u32, buffer: &mut [u8]) -> Result<(), ()> {
        for i in 0..count {
            let start = i as usize * 512;
            let end = start + 512;
            if end > buffer.len() {
                return Err(());
            }
            self.read_sector(lba + i as u64, &mut buffer[start..end])?;
        }
        Ok(())
    }

    fn write_blocks(&mut self, lba: u64, count: u32, buffer: &[u8]) -> Result<(), ()> {
        for i in 0..count {
            let start = i as usize * 512;
            let end = (start + 512).min(buffer.len());
            let mut sec = [0u8; 512];
            sec[..end - start].copy_from_slice(&buffer[start..end]);
            self.write_sector(lba + i as u64, &sec)?;
        }
        Ok(())
    }
}

pub fn is_installed(device: &mut AtaDevice) -> bool {
    if device.info.device_type == DeviceType::OpticalDisc {
        return false;
    }
    let mut sector = [0u8; 512];
    if device.read_sector(0, &mut sector).is_err() {
        return false;
    }
    (sector[510] == 0x55 && sector[511] == 0xAA)
        && (&sector[3..10] == b"NETIZEN" || (sector[446] == 0x80 && sector[450] == 0x83))
}

pub fn find_installed_disk() -> Option<AtaDevice> {
    for ch in 0..4u8 {
        if let Some(mut dev) = scan_channel(ch) {
            if dev.info.device_type == DeviceType::HardDisk && is_installed(&mut dev) {
                return Some(dev);
            }
        }
    }
    None
}

pub fn find_install_target_disk(preferred_channel: Option<u8>) -> Option<AtaDevice> {
    if let Some(ch) = preferred_channel {
        if let Some(dev) = scan_channel(ch) {
            if dev.info.device_type == DeviceType::HardDisk && dev.info.block_count > 0 {
                return Some(dev);
            }
        }
    }
    for ch in 0..4u8 {
        if let Some(dev) = scan_channel(ch) {
            if dev.info.device_type == DeviceType::HardDisk && dev.info.block_count > 0 {
                return Some(dev);
            }
        }
    }
    None
}

static MBR_CODE: &[u8] = include_bytes!("../mbr.bin");

fn append_bytes(buf: &mut [u8; 512], len: &mut usize, bytes: &[u8]) {
    for &b in bytes {
        if 28 + *len < 512 {
            buf[28 + *len] = b;
            *len += 1;
        }
    }
}

fn append_num(buf: &mut [u8; 512], len: &mut usize, mut val: i32) {
    if val == 0 {
        append_bytes(buf, len, b"0");
        return;
    }
    let mut tmp = [0u8; 10];
    let mut tlen = 0;
    while val > 0 {
        tmp[tlen] = b'0' + (val % 10) as u8;
        val /= 10;
        tlen += 1;
    }
    for i in (0..tlen).rev() {
        append_bytes(buf, len, &[tmp[i]]);
    }
}

pub fn make_ntzc_pkg(
    id: &str,
    name: &str,
    icon: &str,
    width: i32,
    height: i32,
    uid: u32,
    perms: u32,
) -> [u8; 512] {
    let mut buf = [0u8; 512];
    buf[0..4].copy_from_slice(b"NTZC");
    buf[4..8].copy_from_slice(&1u32.to_le_bytes());
    buf[8..12].copy_from_slice(&uid.to_le_bytes());
    buf[12..16].copy_from_slice(&perms.to_le_bytes());
    let mut m_len = 0usize;
    append_bytes(&mut buf, &mut m_len, b"id=");
    append_bytes(&mut buf, &mut m_len, id.as_bytes());
    append_bytes(&mut buf, &mut m_len, b"\nname=");
    append_bytes(&mut buf, &mut m_len, name.as_bytes());
    append_bytes(&mut buf, &mut m_len, b"\nversion=1.2.0\nicon=");
    append_bytes(&mut buf, &mut m_len, icon.as_bytes());
    append_bytes(&mut buf, &mut m_len, b"\nwidth_units=");
    append_num(&mut buf, &mut m_len, width);
    append_bytes(&mut buf, &mut m_len, b"\nheight_units=");
    append_num(&mut buf, &mut m_len, height);
    append_bytes(&mut buf, &mut m_len, b"\n");
    let payload_off = (28 + m_len) as u32;
    let sig = b"NC_ELF_MOD_V1";
    let mut p_len = 0usize;
    for &b in sig {
        if (payload_off as usize) + p_len < 512 {
            buf[(payload_off as usize) + p_len] = b;
            p_len += 1;
        }
    }
    buf[16..20].copy_from_slice(&(m_len as u32).to_le_bytes());
    buf[20..24].copy_from_slice(&payload_off.to_le_bytes());
    buf[24..28].copy_from_slice(&(p_len as u32).to_le_bytes());
    buf
}

pub fn make_sample_png(theme_id: u8) -> [u8; 512] {
    let mut buf = [0u8; 512];
    buf[0..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    buf[8..12].copy_from_slice(&13u32.to_be_bytes());
    buf[12..16].copy_from_slice(b"IHDR");
    buf[16..20].copy_from_slice(&160u32.to_be_bytes());
    buf[20..24].copy_from_slice(&100u32.to_be_bytes());
    buf[24] = 8;
    buf[25] = 6;
    buf[26] = theme_id;
    buf[29..33].copy_from_slice(&0xA1B2_C3D4u32.to_be_bytes());
    buf[33..37].copy_from_slice(&192u32.to_be_bytes());
    buf[37..41].copy_from_slice(b"IDAT");
    for i in 0..192 {
        buf[41 + i] = ((i as u32 * 17 + theme_id as u32 * 53) & 0xFF) as u8;
    }
    buf[237..241].copy_from_slice(b"IEND");
    buf
}

pub fn make_sample_mp4(scene_id: u8, duration_secs: u16) -> [u8; 512] {
    let mut buf = [0u8; 512];
    buf[0..4].copy_from_slice(&24u32.to_be_bytes());
    buf[4..8].copy_from_slice(b"ftyp");
    buf[8..12].copy_from_slice(b"isom");
    buf[12..16].copy_from_slice(&512u32.to_be_bytes());
    buf[16..20].copy_from_slice(b"isom");
    buf[20..24].copy_from_slice(b"mp42");

    buf[24..28].copy_from_slice(&64u32.to_be_bytes());
    buf[28..32].copy_from_slice(b"moov");
    buf[32..36].copy_from_slice(&56u32.to_be_bytes());
    buf[36..40].copy_from_slice(b"mvhd");
    buf[40] = scene_id;
    buf[44..48].copy_from_slice(&60u32.to_be_bytes());
    buf[48..52].copy_from_slice(&((duration_secs as u32) * 60).to_be_bytes());
    buf[52..54].copy_from_slice(&320u16.to_be_bytes());
    buf[54..56].copy_from_slice(&180u16.to_be_bytes());

    buf[88..92].copy_from_slice(&424u32.to_be_bytes());
    buf[92..96].copy_from_slice(b"mdat");
    buf[96..104].copy_from_slice(b"NCVID100");
    for i in 0..400 {
        buf[104 + i] = ((i as u32 * 29 + scene_id as u32 * 41) & 0xFF) as u8;
    }
    buf
}

fn write_system_erofs(device: &mut AtaDevice, start_lba: u64) -> Result<(), ()> {
    let mut sb = [0u8; 1024];
    sb[0..4].copy_from_slice(&0xE0F5_E1E2u32.to_le_bytes());
    sb[12] = 12;
    sb[14..16].copy_from_slice(&0u16.to_le_bytes());
    sb[32..36].copy_from_slice(&1u32.to_le_bytes());
    device.write_blocks(start_lba + 2, 2, &sb)?;

    let build_prop_text = b"ro.netizen.os.name=NetizenCore\nro.netizen.os.version=1.2.0\nro.netizen.arch=x86_64\nro.netizen.ui.style=EMUI-Porcelain-Aero\nro.netizen.vfs.mounts=/system,/data,/ram\nro.netizen.pkg.autoinstall=/system/app,/data/app\n";

    let mut meta = [0u8; 4096];
    let write_inode = |buf: &mut [u8; 4096], nid: usize, is_dir: bool, size: u32, blk: u32| {
        let off = nid * 32;
        buf[off..off + 2].copy_from_slice(&0u16.to_le_bytes());
        let mode: u16 = if is_dir { 0o040755 } else { 0o100644 };
        buf[off + 4..off + 6].copy_from_slice(&mode.to_le_bytes());
        buf[off + 8..off + 12].copy_from_slice(&size.to_le_bytes());
        buf[off + 16..off + 20].copy_from_slice(&blk.to_le_bytes());
    };
    write_inode(&mut meta, 0, true, 38, 2);
    write_inode(&mut meta, 1, true, 121, 3);
    write_inode(&mut meta, 2, false, build_prop_text.len() as u32, 4);
    write_inode(&mut meta, 3, false, 256, 5);
    write_inode(&mut meta, 4, false, 256, 6);
    write_inode(&mut meta, 5, false, 256, 7);
    write_inode(&mut meta, 6, false, 256, 8);
    write_inode(&mut meta, 7, false, 256, 9);
    device.write_blocks(start_lba + 8, 8, &meta)?;

    let mut root_dir = [0u8; 512];
    root_dir[0..8].copy_from_slice(&1u64.to_le_bytes());
    root_dir[8..10].copy_from_slice(&24u16.to_le_bytes());
    root_dir[10] = 2;
    root_dir[12..20].copy_from_slice(&2u64.to_le_bytes());
    root_dir[20..22].copy_from_slice(&27u16.to_le_bytes());
    root_dir[22] = 1;
    root_dir[24..27].copy_from_slice(b"app");
    root_dir[27..37].copy_from_slice(b"build.prop");
    root_dir[37] = 0;
    device.write_blocks(start_lba + 16, 1, &root_dir)?;

    let mut app_dir = [0u8; 512];
    let entries: [(u64, u16, &[u8]); 5] = [
        (3, 60, b"files.ntzc"),
        (4, 70, b"browser.ntzc"),
        (5, 82, b"settings.ntzc"),
        (6, 95, b"system.ntzc"),
        (7, 106, b"stopwatch.ntzc"),
    ];
    for (idx, (nid, noff, name)) in entries.iter().enumerate() {
        let base = idx * 12;
        app_dir[base..base + 8].copy_from_slice(&nid.to_le_bytes());
        app_dir[base + 8..base + 10].copy_from_slice(&noff.to_le_bytes());
        app_dir[base + 10] = 1;
        let ns = *noff as usize;
        app_dir[ns..ns + name.len()].copy_from_slice(name);
    }
    app_dir[120] = 0;
    device.write_blocks(start_lba + 24, 1, &app_dir)?;

    let mut prop_sec = [0u8; 512];
    prop_sec[..build_prop_text.len()].copy_from_slice(build_prop_text);
    device.write_blocks(start_lba + 32, 1, &prop_sec)?;

    let pkg_files = make_ntzc_pkg("nc.files", "Dosyalar", "generic", 46, 30, 1000, 7);
    device.write_blocks(start_lba + 40, 1, &pkg_files)?;
    let pkg_browser = make_ntzc_pkg("nc.browser", "Tarayici", "monitor", 44, 28, 1000, 5);
    device.write_blocks(start_lba + 48, 1, &pkg_browser)?;
    let pkg_settings = make_ntzc_pkg("nc.settings", "Ayarlar", "sliders", 38, 24, 1000, 3);
    device.write_blocks(start_lba + 56, 1, &pkg_settings)?;
    let pkg_system = make_ntzc_pkg("nc.system", "Sistem", "monitor", 32, 24, 1000, 1);
    device.write_blocks(start_lba + 64, 1, &pkg_system)?;
    let pkg_stopwatch = make_ntzc_pkg("nc.stopwatch", "Kronometre", "stopwatch", 26, 16, 1000, 0);
    device.write_blocks(start_lba + 72, 1, &pkg_stopwatch)?;

    Ok(())
}

fn write_exfat_entry(
    buf: &mut [u8],
    offset: usize,
    name: &str,
    is_dir: bool,
    cluster: u32,
    size: u64,
) -> usize {
    let chars: usize = name.chars().count().min(30);
    let name_entries = ((chars + 14) / 15).max(1);
    let secondary_count = 1 + name_entries;
    let total_bytes = (1 + secondary_count) * 32;
    if offset + total_bytes > buf.len() {
        return 0;
    }
    for b in &mut buf[offset..offset + total_bytes] {
        *b = 0;
    }
    buf[offset] = 0x85;
    buf[offset + 1] = secondary_count as u8;
    buf[offset + 4] = if is_dir { 0x10 } else { 0x20 };

    let s_off = offset + 32;
    buf[s_off] = 0xC0;
    buf[s_off + 1] = 0x01;
    buf[s_off + 3] = chars as u8;
    buf[s_off + 8..s_off + 16].copy_from_slice(&size.to_le_bytes());
    buf[s_off + 20..s_off + 24].copy_from_slice(&cluster.to_le_bytes());
    buf[s_off + 24..s_off + 32].copy_from_slice(&size.to_le_bytes());

    let mut ch_iter = name.chars();
    for n in 0..name_entries {
        let n_off = offset + 64 + n * 32;
        buf[n_off] = 0xC1;
        buf[n_off + 1] = 0x00;
        for c_idx in 0..15 {
            if let Some(ch) = ch_iter.next() {
                let code = ch as u16;
                let pos = n_off + 2 + c_idx * 2;
                buf[pos] = (code & 0xFF) as u8;
                buf[pos + 1] = (code >> 8) as u8;
            }
        }
    }
    total_bytes
}

pub fn write_data_exfat(device: &mut AtaDevice, start_lba: u64) -> Result<(), ()> {
    let mut vbr = [0u8; 512];
    vbr[0..3].copy_from_slice(&[0xEB, 0x76, 0x90]);
    vbr[3..11].copy_from_slice(b"EXFAT   ");
    vbr[80..84].copy_from_slice(&128u32.to_le_bytes());
    vbr[88..92].copy_from_slice(&256u32.to_le_bytes());
    vbr[96..100].copy_from_slice(&2u32.to_le_bytes());
    vbr[108] = 9;
    vbr[109] = 3;
    vbr[510] = 0x55;
    vbr[511] = 0xAA;
    device.write_blocks(start_lba, 1, &vbr)?;

    let mut fat = [0u8; 512];
    fat[0..4].copy_from_slice(&0xFFFF_FFF8u32.to_le_bytes());
    fat[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    for cl in 2..=10usize {
        let off = cl * 4;
        fat[off..off + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    }
    device.write_blocks(start_lba + 128, 1, &fat)?;
    let zero_sec = [0u8; 512];
    for s in 1..8 {
        device.write_blocks(start_lba + 128 + s, 1, &zero_sec)?;
    }

    for s in 0..(9 * 8) {
        device.write_blocks(start_lba + 256 + s, 1, &zero_sec)?;
    }

    let mut root_dir = [0u8; 512];
    let mut off = 0;
    off += write_exfat_entry(&mut root_dir, off, "app", true, 3, 4096);
    off += write_exfat_entry(&mut root_dir, off, "media", true, 4, 4096);
    let _ = write_exfat_entry(&mut root_dir, off, "documents", true, 5, 4096);
    device.write_blocks(start_lba + 256, 1, &root_dir)?;

    let mut app_dir = [0u8; 512];
    let _ = write_exfat_entry(&mut app_dir, 0, "calculator.ntzc", false, 6, 512);
    device.write_blocks(start_lba + 264, 1, &app_dir)?;

    let mut media_dir = [0u8; 512];
    let mut m_off = 0;
    m_off += write_exfat_entry(&mut media_dir, m_off, "aurora_cam.png", false, 7, 512);
    let _ = write_exfat_entry(&mut media_dir, m_off, "aero_tanitim.mp4", false, 8, 512);
    device.write_blocks(start_lba + 272, 1, &media_dir)?;

    let welcome_txt = b"NetizenCore Dosya Sistemi ve Uygulama Kilavuzu\n==============================================\n\n1. Disk Bolumleri:\n   - /system : EROFS salt okunur sistem dizini.\n   - /data   : exFAT kalici kullanici depolama birimi.\n   - /ram    : Yuksek hizli gecici bellek diski.\n\n2. Paket Yonetimi (.ntzc):\n   /system/app ve /data/app icindeki tum .ntzc paketleri\n   sistem acilisinda otomatik taranir ve masaustune kurulur.\n\n3. Medya ve Metin Destegi:\n   Bu dosyayi duzenleyip 'Diske Kaydet' ile kalici olarak\n   /data bolumune yazabilirsiniz!";
    let log_txt = b"[0.000] NetizenCore x86_64 cekirdegi baslatildi\n[0.120] VBE dogrusal framebuffer: 32-bit renk aktif\n[0.450] ATA PIO Kanal 0: Sabit disk MBR bolumleri baglandi\n[0.610] VFS: /system (EROFS), /data (exFAT), /ram (MemFS) hazir\n[0.780] Paket Tarayici: /system/app ve /data/app paketleri yuklendi\n";

    let mut docs_dir = [0u8; 512];
    let mut d_off = 0;
    d_off += write_exfat_entry(&mut docs_dir, d_off, "hosgeldiniz.txt", false, 9, welcome_txt.len() as u64);
    let _ = write_exfat_entry(&mut docs_dir, d_off, "sistem_gunlugu.txt", false, 10, log_txt.len() as u64);
    device.write_blocks(start_lba + 280, 1, &docs_dir)?;

    let pkg_calc = make_ntzc_pkg("nc.calculator", "Hesap Makinesi", "generic", 28, 22, 10001, 4);
    device.write_blocks(start_lba + 288, 1, &pkg_calc)?;

    let png_data = make_sample_png(0);
    device.write_blocks(start_lba + 296, 1, &png_data)?;

    let mp4_data = make_sample_mp4(0, 18);
    device.write_blocks(start_lba + 304, 1, &mp4_data)?;

  let mut welcome_sec = [0u8; 1024];
    let w_len = welcome_txt.len().min(welcome_sec.len());
    welcome_sec[..w_len].copy_from_slice(&welcome_txt[..w_len]);
    device.write_blocks(start_lba + 312, 2, &welcome_sec)?;

    let mut log_sec = [0u8; 512];
    let l_len = log_txt.len().min(log_sec.len());
    log_sec[..l_len].copy_from_slice(&log_txt[..l_len]);
    device.write_blocks(start_lba + 320, 1, &log_sec)?;

    Ok(())
}

pub fn install_to_disk(device: &mut AtaDevice, stage2: &[u8], kernel: &[u8]) -> Result<(), ()> {
    let total_sectors = device.info.block_count;
    let mut mbr = [0u8; 512];
    mbr[..MBR_CODE.len().min(446)].copy_from_slice(&MBR_CODE[..MBR_CODE.len().min(446)]);

    let write_entry = |mbr: &mut [u8; 512], index: usize, bootable: bool, start_lba: u32, count: u32| {
        let offset = 446 + index * 16;
        mbr[offset] = if bootable { 0x80 } else { 0x00 };
        mbr[offset + 1] = 0xFE;
        mbr[offset + 2] = 0xFF;
        mbr[offset + 3] = 0xFF;
        mbr[offset + 4] = 0x83;
        mbr[offset + 5] = 0xFE;
        mbr[offset + 6] = 0xFF;
        mbr[offset + 7] = 0xFF;
        mbr[offset + 8..offset + 12].copy_from_slice(&start_lba.to_le_bytes());
        mbr[offset + 12..offset + 16].copy_from_slice(&count.to_le_bytes());
    };

    let kernel_start: u32 = 2048;
    let kernel_sectors: u32 = 131_072;
    write_entry(&mut mbr, 0, true, kernel_start, kernel_sectors);

    let system_start: u32 = kernel_start + kernel_sectors;
    let system_sectors: u32 = 4_194_304;
    write_entry(&mut mbr, 1, false, system_start, system_sectors);

    let security_start: u32 = system_start + system_sectors;
    let security_sectors: u32 = 524_288;
    write_entry(&mut mbr, 2, false, security_start, security_sectors);

    let data_start: u32 = security_start + security_sectors;
    let data_sectors: u32 = if (total_sectors as u32) > data_start {
        (total_sectors as u32) - data_start
    } else {
        2_097_152
    };
    write_entry(&mut mbr, 3, false, data_start, data_sectors);

    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    device.write_sector(0, &mbr)?;

    let stage2_sectors = ((stage2.len() + 511) / 512) as u32;
    device.write_blocks(2048, stage2_sectors, stage2)?;
    let kernel_sectors_cnt = ((kernel.len() + 511) / 512) as u32;
    device.write_blocks(2048 + stage2_sectors as u64, kernel_sectors_cnt, kernel)?;

    write_system_erofs(device, system_start as u64)?;
    write_data_exfat(device, data_start as u64)?;
    device.flush();
    Ok(())
}

pub fn scan_channel(channel: u8) -> Option<AtaDevice> {
    if channel >= 4 {
        return None;
    }
    let io_base = if channel < 2 { 0x01F0 } else { 0x0170 };
    let ctrl_base = if channel < 2 { 0x03F6 } else { 0x0376 };
    let drive_select = if channel & 1 == 0 { 0xA0 } else { 0xB0 };

    unsafe {
        outb(ctrl_base, 0x02);
        for _ in 0..4 {
            inb(ctrl_base);
        }
        outb(io_base + 6, drive_select);
        for _ in 0..4 {
            inb(ctrl_base);
        }
        let init_status = inb(io_base + 7);
        if init_status == 0xFF {
            return None;
        }
        let mut spins = 0u32;
        while (inb(io_base + 7) & 0x80) != 0 {
            spins += 1;
            if spins > 100_000 {
                return None;
            }
        }
        outb(io_base + 2, 0);
        outb(io_base + 3, 0);
        outb(io_base + 4, 0);
        outb(io_base + 5, 0);
        outb(io_base + 7, 0xEC);
        for _ in 0..4 {
            inb(ctrl_base);
        }
        let status_after_cmd = inb(io_base + 7);
        if status_after_cmd == 0 {
            return None;
        }
        spins = 0;
        while (inb(io_base + 7) & 0x80) != 0 {
            spins += 1;
            if spins > 100_000 {
                return None;
            }
        }
        let lba_mid = inb(io_base + 4);
        let lba_high = inb(io_base + 5);
        let is_atapi = (lba_mid == 0x14 && lba_high == 0xEB) || (lba_mid == 0x69 && lba_high == 0x96);
        if is_atapi {
            outb(io_base + 7, 0xA1);
            for _ in 0..4 {
                inb(ctrl_base);
            }
            let mut atapi_spins = 0u32;
            while (inb(io_base + 7) & 0x80) != 0 {
                atapi_spins += 1;
                if atapi_spins > 20_000 {
                    return None;
                }
            }
        }
        let final_status = inb(io_base + 7);
        if final_status & 0x01 != 0 || final_status & 0x08 == 0 {
            return None;
        }
        let mut buffer = [0u16; 256];
        for i in 0..256 {
            buffer[i] = inw(io_base);
        }
        let _ = inb(io_base + 7);

        let mut model = [0u8; 40];
        for index in 0..20 {
            let word = buffer[27 + index];
            model[index * 2] = (word >> 8) as u8;
            model[index * 2 + 1] = (word & 0xFF) as u8;
        }
        let mut model_len = 40;
        while model_len > 0 && (model[model_len - 1] == b' ' || model[model_len - 1] == 0) {
            model_len -= 1;
        }
        let sectors = if !is_atapi {
            let lba48 = (buffer[83] & (1 << 10)) != 0;
            if lba48 {
                (buffer[100] as u64)
                    | ((buffer[101] as u64) << 16)
                    | ((buffer[102] as u64) << 32)
                    | ((buffer[103] as u64) << 48)
            } else {
                (buffer[60] as u64) | ((buffer[61] as u64) << 16)
            }
        } else {
            0
        };
        let device_type = if is_atapi {
            DeviceType::OpticalDisc
        } else {
            DeviceType::HardDisk
        };
        Some(AtaDevice {
            channel,
            info: BlockDeviceInfo {
                model,
                model_len,
                block_count: sectors,
                block_size: 512,
                device_type,
            },
        })
    }
}

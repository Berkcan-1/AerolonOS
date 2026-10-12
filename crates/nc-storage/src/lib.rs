#![no_std]

use nc_arch::{inb, inw, outb, outw};
use nc_hal::{BlockDevice, BlockDeviceInfo, DeviceType};

pub const MAX_DISKS: usize = 4;

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
        outb(io_base + 7, 0x30); // WRITE SECTORS
        
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
        
        // KRİTİK DÜZELTME: Veri yazıldıktan sonra diskin sektörü kaydetmesini bekle
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

static MBR_CODE: &[u8] = include_bytes!("../mbr.bin");

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
        let start_bytes = start_lba.to_le_bytes();
        let count_bytes = count.to_le_bytes();
        mbr[offset + 8..offset + 12].copy_from_slice(&start_bytes);
        mbr[offset + 12..offset + 16].copy_from_slice(&count_bytes);
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

    let kernel_sectors = ((kernel.len() + 511) / 512) as u32;
    device.write_blocks(2048 + stage2_sectors as u64, kernel_sectors, kernel)?;

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
        // Log: Port başlatma
        nc_arch::serial::write_str(" [ATA] nIEN ayarlanıyor...\r\n");
        outb(ctrl_base, 0x02); // Kesmeleri devre dışı bırak (nIEN = 1)
        for _ in 0..4 { inb(ctrl_base); }

        nc_arch::serial::write_str(" [ATA] Surucu seciliyor (0x");
        outb(io_base + 6, drive_select);
        for _ in 0..4 { inb(ctrl_base); }

        let init_status = inb(io_base + 7);
        if init_status == 0xFF {
            nc_arch::serial::write_str(" [ATA] Durum 0xFF: Kanal bos.\r\n");
            return None;
        }

        nc_arch::serial::write_str(" [ATA] BSY kontrol ediliyor...\r\n");
        let mut spins = 0u32;
        while (inb(io_base + 7) & 0x80) != 0 {
            spins += 1;
            if spins > 100_000 {
                nc_arch::serial::write_str(" [ATA] BSY timeout!\r\n");
                return None;
            }
        }

        nc_arch::serial::write_str(" [ATA] Registerlar sifirlaniyor ve IDENTIFY gonderiliyor...\r\n");
        outb(io_base + 2, 0);
        outb(io_base + 3, 0);
        outb(io_base + 4, 0);
        outb(io_base + 5, 0);
        outb(io_base + 7, 0xEC); // IDENTIFY
        for _ in 0..4 { inb(ctrl_base); }

        let status_after_cmd = inb(io_base + 7);
        if status_after_cmd == 0 {
            nc_arch::serial::write_str(" [ATA] Surucu yok (Status = 0).\r\n");
            return None;
        }

        nc_arch::serial::write_str(" [ATA] Komut sonrasi BSY bekleniyor...\r\n");
        spins = 0;
        while (inb(io_base + 7) & 0x80) != 0 {
            spins += 1;
            if spins > 100_000 {
                nc_arch::serial::write_str(" [ATA] Komut sonrasi BSY timeout!\r\n");
                return None;
            }
        }

        let lba_mid = inb(io_base + 4);
        let lba_high = inb(io_base + 5);
        let is_atapi = (lba_mid == 0x14 && lba_high == 0xEB) || (lba_mid == 0x69 && lba_high == 0x96);

        if is_atapi {
            nc_arch::serial::write_str(" [ATA] ATAPI surucu algilandi, IDENTIFY PACKET...\r\n");
            outb(io_base + 7, 0xA1);
            for _ in 0..4 { inb(ctrl_base); }
            let mut atapi_spins = 0u32;
            while (inb(io_base + 7) & 0x80) != 0 {
                atapi_spins += 1;
                if atapi_spins > 20_000 {
                    return None;
                }
            }
        }

        nc_arch::serial::write_str(" [ATA] DRQ bayragi kontrol ediliyor...\r\n");
        let final_status = inb(io_base + 7);
        if final_status & 0x01 != 0 {
            nc_arch::serial::write_str(" [ATA] ERR bayragi aktif!\r\n");
            return None;
        }
        if final_status & 0x08 == 0 {
            nc_arch::serial::write_str(" [ATA] DRQ aktif degil (Veri hazir degil).\r\n");
            return None;
        }

      nc_arch::serial::write_str(" [ATA] 256 Word veri okunuyor...\r\n");
        let mut buffer = [0u16; 256];
        for i in 0..256 {
            buffer[i] = inw(io_base);
        }

        // KRİTİK: Son sektör okunduktan sonra Status Register (io_base + 7) 
        // mutlaka bir kez okunmalıdır; bu işlem denetleyicinin bekleyen IRQ bayrağını temizler!
        let _ = inb(io_base + 7);

        nc_arch::serial::write_str(" [ATA] Model bilgisi cikariliyor...\r\n");
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

        nc_arch::serial::write_str(" [ATA] Surucu basariyla okundu!\r\n");

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

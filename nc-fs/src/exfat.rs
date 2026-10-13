extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use nc_hal::BlockDevice;
use crate::vfs::{DirEntry, FileSystem, FileType, FsError};

const EXFAT_FILE_ENTRY: u8 = 0x85;
const EXFAT_STREAM_ENTRY: u8 = 0xC0;
const EXFAT_NAME_ENTRY: u8 = 0xC1;
const EXFAT_ATTR_DIRECTORY: u16 = 0x0010;

pub struct Exfat {
    device: Box<dyn BlockDevice>,
    bytes_per_sector: u32,
    sectors_per_cluster: u32,
    cluster_size: u32,
    fat_offset_lba: u64,
    cluster_heap_offset_lba: u64,
    root_dir_cluster: u32,
}

impl Exfat {
    pub fn new(mut device: Box<dyn BlockDevice>) -> Result<Self, FsError> {
        let mut vbr = [0u8; 512];
        if device.read_blocks(0, 1, &mut vbr).is_err() {
            return Err(FsError::DiskError);
        }

        if &vbr[3..11] != b"EXFAT   " {
            return Err(FsError::InvalidSuperblock);
        }

        if vbr[510] != 0x55 || vbr[511] != 0xAA {
            return Err(FsError::InvalidSuperblock);
        }

        let bytes_per_sector_shift = vbr[108];
        let sectors_per_cluster_shift = vbr[109];
        let bytes_per_sector = 1u32 << bytes_per_sector_shift;
        let sectors_per_cluster = 1u32 << sectors_per_cluster_shift;
        let cluster_size = bytes_per_sector * sectors_per_cluster;

        let fat_offset_lba = u32::from_le_bytes([vbr[80], vbr[81], vbr[82], vbr[83]]) as u64;
        let cluster_heap_offset_lba = u32::from_le_bytes([vbr[88], vbr[89], vbr[90], vbr[91]]) as u64;
        let root_dir_cluster = u32::from_le_bytes([vbr[96], vbr[97], vbr[98], vbr[99]]);

        Ok(Exfat {
            device,
            bytes_per_sector,
            sectors_per_cluster,
            cluster_size,
            fat_offset_lba,
            cluster_heap_offset_lba,
            root_dir_cluster,
        })
    }

    fn cluster_to_lba(&self, cluster: u32) -> u64 {
        self.cluster_heap_offset_lba + ((cluster as u64 - 2) * self.sectors_per_cluster as u64)
    }

    fn next_cluster(&mut self, cluster: u32) -> Result<u32, FsError> {
        let fat_entry_offset = cluster as u64 * 4;
        let sector_offset = fat_entry_offset / self.bytes_per_sector as u64;
        let byte_in_sector = (fat_entry_offset % self.bytes_per_sector as u64) as usize;

        let mut buf = [0u8; 512];
        if self.device.read_blocks(self.fat_offset_lba + sector_offset, 1, &mut buf).is_err() {
            return Err(FsError::DiskError);
        }

        let next = u32::from_le_bytes([
            buf[byte_in_sector],
            buf[byte_in_sector + 1],
            buf[byte_in_sector + 2],
            buf[byte_in_sector + 3],
        ]);

        Ok(next)
    }

    fn read_cluster_chain(&mut self, start_cluster: u32, no_fat_chain: bool, size: u64) -> Result<Vec<u8>, FsError> {
        let mut buffer = Vec::new();
        let mut current_cluster = start_cluster;
        let mut bytes_left = size;

        loop {
            if current_cluster < 2 || current_cluster >= 0xFFFF_FFF7 {
                break;
            }

            let lba = self.cluster_to_lba(current_cluster);
            let mut cluster_buf = Vec::with_capacity(self.cluster_size as usize);
            cluster_buf.resize(self.cluster_size as usize, 0);

            if self.device.read_blocks(lba, self.sectors_per_cluster, &mut cluster_buf).is_err() {
                return Err(FsError::DiskError);
            }

            let to_copy = (bytes_left as usize).min(self.cluster_size as usize);
            buffer.extend_from_slice(&cluster_buf[..to_copy]);
            bytes_left = bytes_left.saturating_sub(to_copy as u64);

            if bytes_left == 0 {
                break;
            }

            if no_fat_chain {
                current_cluster += 1;
            } else {
                current_cluster = self.next_cluster(current_cluster)?;
            }
        }

        Ok(buffer)
    }

    fn parse_directory_data(&self, data: &[u8]) -> Result<Vec<DirEntry>, FsError> {
        let mut entries = Vec::new();
        let mut i = 0;

        while i + 32 <= data.len() {
            let entry_type = data[i];
            if entry_type == 0x00 {
                break;
            }

            if entry_type == EXFAT_FILE_ENTRY {
                let secondary_count = data[i + 1] as usize;
                let attr = u16::from_le_bytes([data[i + 4], data[i + 5]]);
                let is_dir = (attr & EXFAT_ATTR_DIRECTORY) != 0;

                let stream_idx = i + 32;
                if stream_idx + 32 <= data.len() && data[stream_idx] == EXFAT_STREAM_ENTRY {
                    let flags = data[stream_idx + 1];
                    let no_fat_chain = (flags & 0x02) != 0;
                    let first_cluster = u32::from_le_bytes([
                        data[stream_idx + 20],
                        data[stream_idx + 21],
                        data[stream_idx + 22],
                        data[stream_idx + 23],
                    ]);
                    let data_len = u64::from_le_bytes([
                        data[stream_idx + 24],
                        data[stream_idx + 25],
                        data[stream_idx + 26],
                        data[stream_idx + 27],
                        data[stream_idx + 28],
                        data[stream_idx + 29],
                        data[stream_idx + 30],
                        data[stream_idx + 31],
                    ]);

                    let mut name = String::new();
                    let name_entries_count = secondary_count.saturating_sub(1);

                    for n in 0..name_entries_count {
                        let name_offset = stream_idx + 32 + (n * 32);
                        if name_offset + 32 > data.len() || data[name_offset] != EXFAT_NAME_ENTRY {
                            break;
                        }
                        for c in 0..15 {
                            let ch_idx = name_offset + 2 + (c * 2);
                            let code = u16::from_le_bytes([data[ch_idx], data[ch_idx + 1]]);
                            if code == 0 {
                                break;
                            }
                            if let Some(ch) = char::from_u32(code as u32) {
                                name.push(ch);
                            }
                        }
                    }

                    if !name.is_empty() {
                        let file_type = if is_dir { FileType::Directory } else { FileType::RegularFile };
                        let flags_encoded = if no_fat_chain { 1u64 << 32 } else { 0 };
                        entries.push(DirEntry {
                            name,
                            file_type,
                            size: data_len,
                            id: (first_cluster as u64) | flags_encoded,
                        });
                    }
                }
                i += (1 + secondary_count) * 32;
            } else {
                i += 32;
            }
        }

        Ok(entries)
    }

    fn resolve_path(&mut self, path: &str) -> Result<(u32, bool, u64, bool), FsError> {
        let trimmed = path.trim_matches('/');
        if trimmed.is_empty() {
            return Ok((self.root_dir_cluster, false, 0, true));
        }

        let mut current_cluster = self.root_dir_cluster;
        let mut no_fat_chain = false;
        let mut current_size = u64::MAX;
        let mut is_dir = true;

        for segment in trimmed.split('/') {
            if !is_dir {
                return Err(FsError::NotADirectory);
            }

            let dir_data = self.read_cluster_chain(current_cluster, no_fat_chain, 512 * 1024)?;
            let entries = self.parse_directory_data(&dir_data)?;

            let found = match entries.into_iter().find(|e| e.name == segment) {
                Some(entry) => entry,
                None => return Err(FsError::NotFound),
            };

            current_cluster = (found.id & 0xFFFF_FFFF) as u32;
            no_fat_chain = (found.id & (1u64 << 32)) != 0;
            current_size = found.size;
            is_dir = found.file_type == FileType::Directory;
        }

        Ok((current_cluster, no_fat_chain, current_size, is_dir))
    }
}

impl FileSystem for Exfat {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let (cluster, no_fat, _, is_dir) = self.resolve_path(path)?;
        if !is_dir {
            return Err(FsError::NotADirectory);
        }
        let dir_data = self.read_cluster_chain(cluster, no_fat, 512 * 1024)?;
        self.parse_directory_data(&dir_data)
    }

    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, FsError> {
        let (cluster, no_fat, size, is_dir) = self.resolve_path(path)?;
        if is_dir {
            return Err(FsError::NotAFile);
        }
        self.read_cluster_chain(cluster, no_fat, size)
    }

    fn file_size(&mut self, path: &str) -> Result<u64, FsError> {
        let (_, _, size, is_dir) = self.resolve_path(path)?;
        if is_dir {
            return Err(FsError::NotAFile);
        }
        Ok(size)
    }
}

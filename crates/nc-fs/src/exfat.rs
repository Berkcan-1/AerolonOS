extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use nc_hal::BlockDevice;
use crate::vfs::{DirEntry, FileStat, FileSystem, FileType, FsError};

const EXFAT_FILE_ENTRY: u8 = 0x85;
const EXFAT_STREAM_ENTRY: u8 = 0xC0;
const EXFAT_NAME_ENTRY: u8 = 0xC1;
const EXFAT_ATTR_DIRECTORY: u16 = 0x0010;
const MAX_CLUSTERS_SCAN: u32 = 1024;

pub struct Exfat {
    device: Box<dyn BlockDevice>,
    #[allow(dead_code)]
    bytes_per_sector: u32,
    sectors_per_cluster: u32,
    cluster_size: u32,
    fat_offset_lba: u64,
    cluster_heap_offset_lba: u64,
    root_dir_cluster: u32,
}

struct RawEntryLoc {
    byte_offset: usize,
    total_bytes: usize,
    first_cluster: u32,
    no_fat_chain: bool,
    size: u64,
    is_dir: bool,
    name: String,
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

    fn read_fat_entry(&mut self, cluster: u32) -> Result<u32, FsError> {
        let fat_entry_offset = cluster as u64 * 4;
        let sector_offset = fat_entry_offset / 512;
        let byte_in_sector = (fat_entry_offset % 512) as usize;
        let mut buf = [0u8; 512];
        if self.device.read_blocks(self.fat_offset_lba + sector_offset, 1, &mut buf).is_err() {
            return Err(FsError::DiskError);
        }
        Ok(u32::from_le_bytes([
            buf[byte_in_sector],
            buf[byte_in_sector + 1],
            buf[byte_in_sector + 2],
            buf[byte_in_sector + 3],
        ]))
    }

    fn write_fat_entry(&mut self, cluster: u32, value: u32) -> Result<(), FsError> {
        let fat_entry_offset = cluster as u64 * 4;
        let sector_offset = fat_entry_offset / 512;
        let byte_in_sector = (fat_entry_offset % 512) as usize;
        let mut buf = [0u8; 512];
        if self.device.read_blocks(self.fat_offset_lba + sector_offset, 1, &mut buf).is_err() {
            return Err(FsError::DiskError);
        }
        buf[byte_in_sector..byte_in_sector + 4].copy_from_slice(&value.to_le_bytes());
        if self.device.write_blocks(self.fat_offset_lba + sector_offset, 1, &buf).is_err() {
            return Err(FsError::DiskError);
        }
        Ok(())
    }

    fn free_cluster_chain(&mut self, start_cluster: u32, no_fat_chain: bool) -> Result<(), FsError> {
        if start_cluster < 2 || start_cluster >= MAX_CLUSTERS_SCAN {
            return Ok(());
        }
        if no_fat_chain {
            self.write_fat_entry(start_cluster, 0)?;
            return Ok(());
        }
        let mut curr = start_cluster;
        let mut steps = 0usize;
        while curr >= 2 && curr < 0xFFFF_FFF7 && steps < 512 {
            let next = self.read_fat_entry(curr)?;
            self.write_fat_entry(curr, 0)?;
            if next == 0 || next >= 0xFFFF_FFF7 {
                break;
            }
            curr = next;
            steps += 1;
        }
        Ok(())
    }

    fn allocate_clusters(&mut self, count: usize) -> Result<Vec<u32>, FsError> {
        let needed = count.max(1);
        let mut allocated = Vec::with_capacity(needed);
        for candidate in 3..MAX_CLUSTERS_SCAN {
            let val = self.read_fat_entry(candidate)?;
            if val == 0 {
                allocated.push(candidate);
                if allocated.len() == needed {
                    break;
                }
            }
        }
        if allocated.len() < needed {
            return Err(FsError::StorageFull);
        }
        for i in 0..allocated.len() {
            let next_val = if i + 1 < allocated.len() {
                allocated[i + 1]
            } else {
                0xFFFF_FFFF
            };
            self.write_fat_entry(allocated[i], next_val)?;
        }
        Ok(allocated)
    }

    fn write_data_to_clusters(&mut self, clusters: &[u32], data: &[u8]) -> Result<(), FsError> {
        let csize = self.cluster_size as usize;
        let mut cluster_buf = Vec::with_capacity(csize);
        cluster_buf.resize(csize, 0);

        for (idx, &cl) in clusters.iter().enumerate() {
            for b in cluster_buf.iter_mut() {
                *b = 0;
            }
            let start = idx * csize;
            if start < data.len() {
                let copy_len = (data.len() - start).min(csize);
                cluster_buf[..copy_len].copy_from_slice(&data[start..start + copy_len]);
            }
            let lba = self.cluster_to_lba(cl);
            if self.device.write_blocks(lba, self.sectors_per_cluster, &cluster_buf).is_err() {
                return Err(FsError::DiskError);
            }
        }
        Ok(())
    }

    fn read_single_cluster(&mut self, cluster: u32) -> Result<Vec<u8>, FsError> {
        let csize = self.cluster_size as usize;
        let mut buf = Vec::with_capacity(csize);
        buf.resize(csize, 0);
        let lba = self.cluster_to_lba(cluster);
        if self.device.read_blocks(lba, self.sectors_per_cluster, &mut buf).is_err() {
            return Err(FsError::DiskError);
        }
        Ok(buf)
    }

    fn write_single_cluster(&mut self, cluster: u32, buf: &[u8]) -> Result<(), FsError> {
        let lba = self.cluster_to_lba(cluster);
        if self.device.write_blocks(lba, self.sectors_per_cluster, buf).is_err() {
            return Err(FsError::DiskError);
        }
        Ok(())
    }

    fn read_cluster_chain(&mut self, start_cluster: u32, no_fat_chain: bool, size: u64) -> Result<Vec<u8>, FsError> {
        let mut buffer = Vec::new();
        if size == 0 {
            return Ok(buffer);
        }
        let mut current_cluster = start_cluster;
        let mut bytes_left = size;
        let mut guard = 0usize;
        loop {
            if current_cluster < 2 || current_cluster >= 0xFFFF_FFF7 || guard > 512 {
                break;
            }
            guard += 1;
            let cluster_buf = self.read_single_cluster(current_cluster)?;
            let to_copy = (bytes_left as usize).min(self.cluster_size as usize);
            buffer.extend_from_slice(&cluster_buf[..to_copy]);
            bytes_left = bytes_left.saturating_sub(to_copy as u64);
            if bytes_left == 0 {
                break;
            }
            if no_fat_chain {
                current_cluster += 1;
            } else {
                current_cluster = self.read_fat_entry(current_cluster)?;
            }
        }
        Ok(buffer)
    }

    fn scan_raw_entries(&self, data: &[u8]) -> Vec<RawEntryLoc> {
        let mut list = Vec::new();
        let mut i = 0;
        while i + 32 <= data.len() {
            let entry_type = data[i];
            if entry_type == 0x00 {
                break;
            }
            if entry_type == EXFAT_FILE_ENTRY {
                let secondary_count = data[i + 1] as usize;
                let total_bytes = (1 + secondary_count) * 32;
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
                        list.push(RawEntryLoc {
                            byte_offset: i,
                            total_bytes,
                            first_cluster,
                            no_fat_chain,
                            size: data_len,
                            is_dir,
                            name,
                        });
                    }
                }
                i += total_bytes.max(32);
            } else {
                i += 32;
            }
        }
        list
    }

    fn parse_directory_data(&self, data: &[u8]) -> Result<Vec<DirEntry>, FsError> {
        let raw = self.scan_raw_entries(data);
        let mut entries = Vec::with_capacity(raw.len());
        for item in raw {
            let file_type = if item.is_dir { FileType::Directory } else { FileType::RegularFile };
            let flags_encoded = if item.no_fat_chain { 1u64 << 32 } else { 0 };
            entries.push(DirEntry {
                name: item.name,
                file_type,
                size: item.size,
                id: (item.first_cluster as u64) | flags_encoded,
            });
        }
        Ok(entries)
    }

    fn encode_entry_set(buf: &mut [u8], offset: usize, name: &str, is_dir: bool, cluster: u32, size: u64) -> Result<usize, FsError> {
        let chars_cnt = name.chars().count().min(30);
        let name_entries = ((chars_cnt + 14) / 15).max(1);
        let secondary_count = 1 + name_entries;
        let total_bytes = (1 + secondary_count) * 32;
        if offset + total_bytes > buf.len() {
            return Err(FsError::StorageFull);
        }
        for b in &mut buf[offset..offset + total_bytes] {
            *b = 0;
        }
        buf[offset] = EXFAT_FILE_ENTRY;
        buf[offset + 1] = secondary_count as u8;
        buf[offset + 4] = if is_dir { 0x10 } else { 0x20 };

        let s_off = offset + 32;
        buf[s_off] = EXFAT_STREAM_ENTRY;
        buf[s_off + 1] = 0x01;
        buf[s_off + 3] = chars_cnt as u8;
        buf[s_off + 8..s_off + 16].copy_from_slice(&size.to_le_bytes());
        buf[s_off + 20..s_off + 24].copy_from_slice(&cluster.to_le_bytes());
        buf[s_off + 24..s_off + 32].copy_from_slice(&size.to_le_bytes());

        let mut ch_iter = name.chars();
        for n in 0..name_entries {
            let n_off = offset + 64 + n * 32;
            buf[n_off] = EXFAT_NAME_ENTRY;
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
        Ok(total_bytes)
    }

    fn split_parent_and_name<'a>(&self, path: &'a str) -> Result<(&'a str, &'a str), FsError> {
        let trimmed = path.trim_matches('/');
        if trimmed.is_empty() {
            return Err(FsError::InvalidPath);
        }
        if let Some(pos) = trimmed.rfind('/') {
            let parent = &trimmed[..pos];
            let name = &trimmed[pos + 1..];
            if name.is_empty() {
                Err(FsError::InvalidPath)
            } else {
                Ok((parent, name))
            }
        } else {
            Ok(("", trimmed))
        }
    }

    fn resolve_path(&mut self, path: &str) -> Result<(u32, bool, u64, bool), FsError> {
        let trimmed = path.trim_matches('/');
        if trimmed.is_empty() {
            return Ok((self.root_dir_cluster, false, self.cluster_size as u64, true));
        }
        let mut current_cluster = self.root_dir_cluster;
        let mut no_fat_chain = false;
        let mut current_size = self.cluster_size as u64;
        let mut is_dir = true;

        for segment in trimmed.split('/') {
            if segment.is_empty() {
                continue;
            }
            if !is_dir {
                return Err(FsError::NotADirectory);
            }
            let dir_data = self.read_cluster_chain(current_cluster, no_fat_chain, self.cluster_size as u64)?;
            let entries = self.parse_directory_data(&dir_data)?;
            let found = match entries.into_iter().find(|e| e.name == segment) {
                Some(entry) => entry,
                None => return Err(FsError::NotFound),
            };
            current_cluster = (found.id & 0xFFFF_FFFF) as u32;
            no_fat_chain = (found.id & (1u64 << 32)) != 0;
            current_size = if found.file_type == FileType::Directory {
                self.cluster_size as u64
            } else {
                found.size
            };
            is_dir = found.file_type == FileType::Directory;
        }
        Ok((current_cluster, no_fat_chain, current_size, is_dir))
    }

    pub fn used_clusters_count(&mut self) -> u32 {
        let mut used = 0u32;
        for c in 2..256 {
            if let Ok(v) = self.read_fat_entry(c) {
                if v != 0 {
                    used += 1;
                }
            }
        }
        used
    }
}

impl FileSystem for Exfat {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let (cluster, no_fat, _, is_dir) = self.resolve_path(path)?;
        if !is_dir {
            return Err(FsError::NotADirectory);
        }
        let dir_data = self.read_cluster_chain(cluster, no_fat, self.cluster_size as u64)?;
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

    fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), FsError> {
        let (parent_path, file_name) = self.split_parent_and_name(path)?;
        let (parent_cluster, _, _, parent_is_dir) = self.resolve_path(parent_path)?;
        if !parent_is_dir {
            return Err(FsError::NotADirectory);
        }
        let mut dir_buf = self.read_single_cluster(parent_cluster)?;
        let raw_entries = self.scan_raw_entries(&dir_buf);

        let csize = self.cluster_size as usize;
        let needed_clusters = ((content.len() + csize - 1) / csize).max(1);

        if let Some(existing) = raw_entries.iter().find(|e| e.name == file_name) {
            if existing.is_dir {
                return Err(FsError::NotAFile);
            }
            self.free_cluster_chain(existing.first_cluster, existing.no_fat_chain)?;
            let new_clusters = self.allocate_clusters(needed_clusters)?;
            self.write_data_to_clusters(&new_clusters, content)?;

            let s_off = existing.byte_offset + 32;
            dir_buf[s_off + 1] = 0x01;
            let sz = content.len() as u64;
            dir_buf[s_off + 8..s_off + 16].copy_from_slice(&sz.to_le_bytes());
            dir_buf[s_off + 20..s_off + 24].copy_from_slice(&new_clusters[0].to_le_bytes());
            dir_buf[s_off + 24..s_off + 32].copy_from_slice(&sz.to_le_bytes());
            self.write_single_cluster(parent_cluster, &dir_buf)?;
            return Ok(());
        }

        let end_offset = raw_entries
            .last()
            .map(|e| e.byte_offset + e.total_bytes)
            .unwrap_or(0);
        let new_clusters = self.allocate_clusters(needed_clusters)?;
        self.write_data_to_clusters(&new_clusters, content)?;
        Self::encode_entry_set(
            &mut dir_buf,
            end_offset,
            file_name,
            false,
            new_clusters[0],
            content.len() as u64,
        )?;
        self.write_single_cluster(parent_cluster, &dir_buf)?;
        Ok(())
    }

    fn create_dir(&mut self, path: &str) -> Result<(), FsError> {
        let (parent_path, dir_name) = self.split_parent_and_name(path)?;
        let (parent_cluster, _, _, parent_is_dir) = self.resolve_path(parent_path)?;
        if !parent_is_dir {
            return Err(FsError::NotADirectory);
        }
        let mut dir_buf = self.read_single_cluster(parent_cluster)?;
        let raw_entries = self.scan_raw_entries(&dir_buf);
        if raw_entries.iter().any(|e| e.name == dir_name) {
            return Err(FsError::AlreadyExists);
        }
        let end_offset = raw_entries
            .last()
            .map(|e| e.byte_offset + e.total_bytes)
            .unwrap_or(0);

        let new_clusters = self.allocate_clusters(1)?;
        let empty_dir = alloc::vec![0u8; self.cluster_size as usize];
        self.write_data_to_clusters(&new_clusters, &empty_dir)?;

        Self::encode_entry_set(
            &mut dir_buf,
            end_offset,
            dir_name,
            true,
            new_clusters[0],
            self.cluster_size as u64,
        )?;
        self.write_single_cluster(parent_cluster, &dir_buf)?;
        Ok(())
    }

    fn remove(&mut self, path: &str) -> Result<(), FsError> {
        let (parent_path, target_name) = self.split_parent_and_name(path)?;
        let (parent_cluster, _, _, parent_is_dir) = self.resolve_path(parent_path)?;
        if !parent_is_dir {
            return Err(FsError::NotADirectory);
        }
        let mut dir_buf = self.read_single_cluster(parent_cluster)?;
        let raw_entries = self.scan_raw_entries(&dir_buf);
        let found = match raw_entries.iter().find(|e| e.name == target_name) {
            Some(e) => e,
            None => return Err(FsError::NotFound),
        };
        self.free_cluster_chain(found.first_cluster, found.no_fat_chain)?;

        let mut rebuilt = alloc::vec![0u8; dir_buf.len()];
        let mut write_cursor = 0usize;
        for entry in &raw_entries {
            if entry.name == target_name {
                continue;
            }
            let slice = &dir_buf[entry.byte_offset..entry.byte_offset + entry.total_bytes];
            rebuilt[write_cursor..write_cursor + entry.total_bytes].copy_from_slice(slice);
            write_cursor += entry.total_bytes;
        }
        dir_buf.copy_from_slice(&rebuilt);
        self.write_single_cluster(parent_cluster, &dir_buf)?;
        Ok(())
    }

    fn file_stat(&mut self, path: &str) -> Result<FileStat, FsError> {
        let (_, _, size, is_dir) = self.resolve_path(path)?;
        Ok(FileStat {
            size,
            is_dir,
            readonly: false,
        })
    }
}

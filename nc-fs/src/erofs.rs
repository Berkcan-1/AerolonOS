extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use nc_hal::BlockDevice;
use crate::vfs::{DirEntry, FileSystem, FileType, FsError};

const EROFS_MAGIC: u32 = 0xE0F5_E1E2;
const EROFS_SUPERBLOCK_OFFSET: u64 = 1024;
const EROFS_INODE_FLAT_PLAIN: u16 = 0;
const EROFS_INODE_FLAT_INLINE: u16 = 1;

pub struct Erofs {
    device: Box<dyn BlockDevice>,
    block_size: usize,
    root_nid: u64,
    meta_blkaddr: u32,
}

impl Erofs {
    pub fn new(mut device: Box<dyn BlockDevice>) -> Result<Self, FsError> {
        let mut buf = [0u8; 1024];
        if device.read_blocks(EROFS_SUPERBLOCK_OFFSET / 512, 2, &mut buf).is_err() {
            return Err(FsError::DiskError);
        }

        let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != EROFS_MAGIC {
            return Err(FsError::InvalidSuperblock);
        }

        let blkszbits = buf[12];
        let block_size = 1usize << blkszbits;
        let root_nid = u16::from_le_bytes([buf[14], buf[15]]) as u64;
        let meta_blkaddr = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);

        Ok(Erofs {
            device,
            block_size,
            root_nid,
            meta_blkaddr,
        })
    }

    fn read_bytes(&mut self, offset: u64, buffer: &mut [u8]) -> Result<(), FsError> {
        let start_sector = offset / 512;
        let start_offset = (offset % 512) as usize;
        let sector_count = ((start_offset + buffer.len() + 511) / 512) as u32;

        let mut temp = Vec::with_capacity(sector_count as usize * 512);
        temp.resize(sector_count as usize * 512, 0);

        if self.device.read_blocks(start_sector, sector_count, &mut temp).is_err() {
            return Err(FsError::DiskError);
        }

        buffer.copy_from_slice(&temp[start_offset..start_offset + buffer.len()]);
        Ok(())
    }

    fn read_inode(&mut self, nid: u64) -> Result<InternalInode, FsError> {
        let inode_offset = (self.meta_blkaddr as u64 * self.block_size as u64) + (nid * 32);
        let mut buf = [0u8; 32];
        self.read_bytes(inode_offset, &mut buf)?;

        let format = u16::from_le_bytes([buf[0], buf[1]]);
        let mode = u16::from_le_bytes([buf[4], buf[5]]);
        let size = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as u64;
        let raw_blkaddr = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);

        let is_dir = (mode & 0o170000) == 0o040000;
        let layout = (format >> 1) & 0x07;

        Ok(InternalInode {
            nid,
            size,
            is_dir,
            layout,
            raw_blkaddr,
            inode_offset,
        })
    }

    fn read_inode_data(&mut self, inode: &InternalInode) -> Result<Vec<u8>, FsError> {
        let mut data = Vec::with_capacity(inode.size as usize);
        data.resize(inode.size as usize, 0);

        if inode.layout == EROFS_INODE_FLAT_PLAIN {
            let offset = inode.raw_blkaddr as u64 * self.block_size as u64;
            self.read_bytes(offset, &mut data)?;
        } else if inode.layout == EROFS_INODE_FLAT_INLINE {
            let inline_offset = inode.inode_offset + 32;
            self.read_bytes(inline_offset, &mut data)?;
        } else {
            return Err(FsError::Corrupted);
        }

        Ok(data)
    }

    fn parse_directory(&self, data: &[u8]) -> Result<Vec<DirEntry>, FsError> {
        let mut entries = Vec::new();
        let mut cursor = 0;

        while cursor + 12 <= data.len() {
            let first_nameoff = u16::from_le_bytes([data[cursor + 8], data[cursor + 9]]) as usize;
            if first_nameoff < 12 || first_nameoff > self.block_size {
                break;
            }

            let dirent_count = first_nameoff / 12;
            let block_limit = (cursor + self.block_size).min(data.len());
            let block_data = &data[cursor..block_limit];

            for i in 0..dirent_count {
                let ent_offset = i * 12;
                if ent_offset + 12 > block_data.len() {
                    break;
                }

                let nid = u64::from_le_bytes([
                    block_data[ent_offset],
                    block_data[ent_offset + 1],
                    block_data[ent_offset + 2],
                    block_data[ent_offset + 3],
                    block_data[ent_offset + 4],
                    block_data[ent_offset + 5],
                    block_data[ent_offset + 6],
                    block_data[ent_offset + 7],
                ]);

                let nameoff = u16::from_le_bytes([
                    block_data[ent_offset + 8],
                    block_data[ent_offset + 9],
                ]) as usize;

                let file_type_byte = block_data[ent_offset + 10];
                let file_type = match file_type_byte {
                    1 => FileType::RegularFile,
                    2 => FileType::Directory,
                    _ => FileType::Unknown,
                };

                let name_end = if i + 1 < dirent_count {
                    u16::from_le_bytes([
                        block_data[ent_offset + 12 + 8],
                        block_data[ent_offset + 12 + 9],
                    ]) as usize
                } else {
                    let mut end = nameoff;
                    while end < block_data.len() && block_data[end] != 0 {
                        end += 1;
                    }
                    end
                };

                if nameoff < block_data.len() && name_end <= block_data.len() && name_end >= nameoff {
                    if let Ok(name_str) = core::str::from_utf8(&block_data[nameoff..name_end]) {
                        if name_str != "." && name_str != ".." {
                            entries.push(DirEntry {
                                name: String::from(name_str),
                                file_type,
                                size: 0,
                                id: nid,
                            });
                        }
                    }
                }
            }

            cursor += self.block_size;
        }

        Ok(entries)
    }

    fn resolve_path(&mut self, path: &str) -> Result<InternalInode, FsError> {
        let mut current_inode = self.read_inode(self.root_nid)?;
        let trimmed = path.trim_matches('/');
        if trimmed.is_empty() {
            return Ok(current_inode);
        }

        for segment in trimmed.split('/') {
            if !current_inode.is_dir {
                return Err(FsError::NotADirectory);
            }
            let data = self.read_inode_data(&current_inode)?;
            let entries = self.parse_directory(&data)?;
            let found = match entries.into_iter().find(|e| e.name == segment) {
                Some(entry) => entry,
                None => return Err(FsError::NotFound),
            };
            current_inode = self.read_inode(found.id)?;
        }

        Ok(current_inode)
    }
}

impl FileSystem for Erofs {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let inode = self.resolve_path(path)?;
        if !inode.is_dir {
            return Err(FsError::NotADirectory);
        }
        let data = self.read_inode_data(&inode)?;
        let mut entries = self.parse_directory(&data)?;
        for entry in entries.iter_mut() {
            if let Ok(target_inode) = self.read_inode(entry.id) {
                entry.size = target_inode.size;
            }
        }
        Ok(entries)
    }

    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, FsError> {
        let inode = self.resolve_path(path)?;
        if inode.is_dir {
            return Err(FsError::NotAFile);
        }
        self.read_inode_data(&inode)
    }

    fn file_size(&mut self, path: &str) -> Result<u64, FsError> {
        let inode = self.resolve_path(path)?;
        if inode.is_dir {
            return Err(FsError::NotAFile);
        }
        Ok(inode.size)
    }
}

struct InternalInode {
    #[allow(dead_code)]
    nid: u64,
    size: u64,
    is_dir: bool,
    layout: u16,
    raw_blkaddr: u32,
    inode_offset: u64,
}

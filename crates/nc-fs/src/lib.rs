#![no_std]
extern crate alloc;

pub mod erofs;
pub mod exfat;
pub mod memfs;
pub mod vfs;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
pub use erofs::Erofs;
pub use exfat::Exfat;
pub use memfs::MemFs;
pub use vfs::{DirEntry, FileStat, FileSystem, FileType, FsError};

use nc_storage::{scan_channel, AtaPartition};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartitionTarget {
    System,
    Data,
    Ram,
}

pub struct VfsManager {
    system_fs: Option<Erofs>,
    data_fs: Option<Exfat>,
    ram_fs: MemFs,
}

impl VfsManager {
    pub fn new() -> Self {
        let mut system_fs = None;
        let mut data_fs = None;

        if let Some(ata) = scan_channel(0) {
            let part1 = AtaPartition::new(ata, 133_120, 4_194_304);
            if let Ok(fs) = Erofs::new(Box::new(part1)) {
                system_fs = Some(fs);
            }

            let part3 = AtaPartition::new(ata, 4_851_712, 2_097_152);
            if let Ok(fs) = Exfat::new(Box::new(part3)) {
                data_fs = Some(fs);
            }
        }

        VfsManager {
            system_fs,
            data_fs,
            ram_fs: MemFs::new(),
        }
    }

    fn split_mount_path<'a>(&self, path: &'a str) -> (&'a str, &'a str) {
        let trimmed = path.trim_start_matches('/');
        if trimmed.is_empty() {
            return ("", "");
        }
        if let Some(pos) = trimmed.find('/') {
            (&trimmed[..pos], &trimmed[pos + 1..])
        } else {
            (trimmed, "")
        }
    }

    pub fn list_dir_path(&mut self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let (mount, rel) = self.split_mount_path(path);
        if mount.is_empty() {
            return Ok(alloc::vec![
                DirEntry {
                    name: String::from("system"),
                    file_type: FileType::Directory,
                    size: 0,
                    id: 1,
                },
                DirEntry {
                    name: String::from("data"),
                    file_type: FileType::Directory,
                    size: 0,
                    id: 2,
                },
                DirEntry {
                    name: String::from("ram"),
                    file_type: FileType::Directory,
                    size: 0,
                    id: 3,
                },
            ]);
        }

        match mount {
            "system" => {
                if let Some(ref mut fs) = self.system_fs {
                    if let Ok(res) = fs.list_dir(rel) {
                        return Ok(res);
                    }
                }
                Self::fallback_dir(PartitionTarget::System, rel)
            }
            "data" => {
                if let Some(ref mut fs) = self.data_fs {
                    if let Ok(res) = fs.list_dir(rel) {
                        return Ok(res);
                    }
                }
                Self::fallback_dir(PartitionTarget::Data, rel)
            }
            "ram" => self.ram_fs.list_dir(rel),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn read_file_path(&mut self, path: &str) -> Result<Vec<u8>, FsError> {
        let (mount, rel) = self.split_mount_path(path);
        match mount {
            "system" => {
                if let Some(ref mut fs) = self.system_fs {
                    if let Ok(res) = fs.read_file(rel) {
                        return Ok(res);
                    }
                }
                Self::fallback_read(PartitionTarget::System, rel)
            }
            "data" => {
                if let Some(ref mut fs) = self.data_fs {
                    if let Ok(res) = fs.read_file(rel) {
                        return Ok(res);
                    }
                }
                Self::fallback_read(PartitionTarget::Data, rel)
            }
            "ram" => self.ram_fs.read_file(rel),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn write_file_path(&mut self, path: &str, content: &[u8]) -> Result<(), FsError> {
        let (mount, rel) = self.split_mount_path(path);
        match mount {
            "ram" => self.ram_fs.write_file(rel, content),
            "system" | "data" => Err(FsError::ReadOnly),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn create_dir_path(&mut self, path: &str) -> Result<(), FsError> {
        let (mount, rel) = self.split_mount_path(path);
        match mount {
            "ram" => self.ram_fs.create_dir(rel),
            "system" | "data" => Err(FsError::ReadOnly),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn remove_path(&mut self, path: &str) -> Result<(), FsError> {
        let (mount, rel) = self.split_mount_path(path);
        match mount {
            "ram" => self.ram_fs.remove(rel),
            "system" | "data" => Err(FsError::ReadOnly),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn file_stat_path(&mut self, path: &str) -> Result<FileStat, FsError> {
        let (mount, rel) = self.split_mount_path(path);
        if mount.is_empty() {
            return Ok(FileStat {
                size: 0,
                is_dir: true,
                readonly: true,
            });
        }
        match mount {
            "system" => Ok(FileStat {
                size: self.read_file_path(path).map(|v| v.len() as u64).unwrap_or(0),
                is_dir: self.list_dir_path(path).is_ok(),
                readonly: true,
            }),
            "data" => Ok(FileStat {
                size: self.read_file_path(path).map(|v| v.len() as u64).unwrap_or(0),
                is_dir: self.list_dir_path(path).is_ok(),
                readonly: true,
            }),
            "ram" => self.ram_fs.file_stat(rel),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn list_dir(&mut self, target: PartitionTarget, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let full_path = match target {
            PartitionTarget::System => alloc::format!("/system/{}", path.trim_start_matches('/')),
            PartitionTarget::Data => alloc::format!("/data/{}", path.trim_start_matches('/')),
            PartitionTarget::Ram => alloc::format!("/ram/{}", path.trim_start_matches('/')),
        };
        self.list_dir_path(&full_path)
    }

    pub fn read_file(&mut self, target: PartitionTarget, path: &str) -> Result<Vec<u8>, FsError> {
        let full_path = match target {
            PartitionTarget::System => alloc::format!("/system/{}", path.trim_start_matches('/')),
            PartitionTarget::Data => alloc::format!("/data/{}", path.trim_start_matches('/')),
            PartitionTarget::Ram => alloc::format!("/ram/{}", path.trim_start_matches('/')),
        };
        self.read_file_path(&full_path)
    }

    fn fallback_dir(target: PartitionTarget, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let trimmed = path.trim_matches('/');
        let mut list = Vec::new();

        if trimmed.is_empty() {
            list.push(DirEntry {
                name: String::from("app"),
                file_type: FileType::Directory,
                size: 0,
                id: 1,
            });
            if target == PartitionTarget::Data {
                list.push(DirEntry {
                    name: String::from("media"),
                    file_type: FileType::Directory,
                    size: 0,
                    id: 2,
                });
                list.push(DirEntry {
                    name: String::from("calculator.ntzc"),
                    file_type: FileType::RegularFile,
                    size: 97_280,
                    id: 3,
                });
            } else {
                list.push(DirEntry {
                    name: String::from("build.prop"),
                    file_type: FileType::RegularFile,
                    size: 2_048,
                    id: 4,
                });
            }
        } else if trimmed == "app" {
            if target == PartitionTarget::System {
                list.push(DirEntry {
                    name: String::from("settings.ntzc"),
                    file_type: FileType::RegularFile,
                    size: 184_320,
                    id: 10,
                });
                list.push(DirEntry {
                    name: String::from("system.ntzc"),
                    file_type: FileType::RegularFile,
                    size: 143_360,
                    id: 11,
                });
            } else {
                list.push(DirEntry {
                    name: String::from("terminal.ntzc"),
                    file_type: FileType::RegularFile,
                    size: 81_920,
                    id: 20,
                });
            }
        } else {
            return Err(FsError::NotFound);
        }

        Ok(list)
    }

    fn fallback_read(_target: PartitionTarget, path: &str) -> Result<Vec<u8>, FsError> {
        let trimmed = path.trim_matches('/');
        if trimmed.ends_with(".ntzc") {
            let manifest = if trimmed.contains("settings") {
                "id=nc.settings\nname=SETTINGS\nversion=1.0.0\nicon=sliders\nwidth_units=38\nheight_units=24"
            } else if trimmed.contains("system") {
                "id=nc.system\nname=SYSTEM\nversion=1.0.0\nicon=monitor\nwidth_units=30\nheight_units=21"
            } else {
                "id=nc.calculator\nname=CALCULATOR\nversion=1.0.0\nicon=generic\nwidth_units=32\nheight_units=22"
            };

            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"NTZC");
            bytes.extend_from_slice(&1u32.to_le_bytes());
            let uid = if trimmed.contains("settings") || trimmed.contains("system") { 1000u32 } else { 10001u32 };
            let perms = if uid == 1000 { 1u32 } else { 4u32 };
            bytes.extend_from_slice(&uid.to_le_bytes());
            bytes.extend_from_slice(&perms.to_le_bytes());
            bytes.extend_from_slice(&(manifest.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&64u32.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(manifest.as_bytes());
            return Ok(bytes);
        }
        Err(FsError::NotFound)
    }
}

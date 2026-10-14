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

use nc_storage::{
    make_ntzc_pkg, make_sample_mp4, make_sample_png, scan_channel, AtaPartition,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartitionTarget {
    System,
    Data,
    Ram,
}

#[derive(Clone, Debug)]
pub struct PartitionMetrics {
    pub mount_point: &'static str,
    pub label: &'static str,
    pub fs_type: &'static str,
    pub start_lba: u64,
    pub sector_count: u64,
    pub total_mb: u64,
    pub used_mb: u64,
    pub readonly: bool,
    pub online: bool,
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
        if let Some(ata) = nc_storage::find_installed_disk().or_else(|| scan_channel(0)) {
            let part1 = AtaPartition::new(ata, 133_120, 4_194_304);
            if let Ok(fs) = Erofs::new(Box::new(part1)) {
                system_fs = Some(fs);
            }
            let part3 = AtaPartition::new(ata, 4_851_712, 2_097_152);
            if let Ok(mut fs) = Exfat::new(Box::new(part3)) {
                Self::ensure_data_structure(&mut fs);
                data_fs = Some(fs);
            }
        }
        let mut ram_fs = MemFs::new();
        let _ = ram_fs.write_file("media/ornek_gorsel.png", &make_sample_png(1));
        let _ = ram_fs.write_file("media/tanitim_klip.mp4", &make_sample_mp4(1, 24));
        VfsManager {
            system_fs,
            data_fs,
            ram_fs,
        }
    }

    fn ensure_data_structure(fs: &mut Exfat) {
        let _ = fs.create_dir("app");
        let _ = fs.create_dir("media");
        let _ = fs.create_dir("documents");
        let _ = fs.create_dir("drivers");
        if fs.read_file("app/calculator.ntzc").is_err() {
            let pkg = make_ntzc_pkg("nc.calculator", "Hesap Makinesi", "generic", 28, 22, 10001, 4);
            let _ = fs.write_file("app/calculator.ntzc", &pkg);
        }
        if fs.read_file("media/aurora_cam.png").is_err() {
            let png = make_sample_png(0);
            let _ = fs.write_file("media/aurora_cam.png", &png);
        }
        if fs.read_file("media/aero_tanitim.mp4").is_err() {
            let mp4 = make_sample_mp4(0, 18);
            let _ = fs.write_file("media/aero_tanitim.mp4", &mp4);
        }
        if fs.read_file("documents/hosgeldiniz.txt").is_err() {
            let txt = b"NetizenCore Kalici Veri Bolumu (/data - exFAT)\n==============================================\nBu bolumde olusturdugunuz tum .txt, .png, .mp4 ve .ntzc\ndosyalari dogrudan sabit diske yazilir ve silinmez.\n";
            let _ = fs.write_file("documents/hosgeldiniz.txt", txt);
        }
    }

    pub fn partition_metrics(&mut self) -> Vec<PartitionMetrics> {
        let sys_online = self.system_fs.is_some();
        let data_online = self.data_fs.is_some();
        let data_used_mb = if let Some(ref mut dfs) = self.data_fs {
            ((dfs.used_clusters_count() as u64) * 4 / 1024).max(16)
        } else {
            24
        };
        alloc::vec![
            PartitionMetrics {
                mount_point: "/boot",
                label: "Bolum 1: Stage2 + Cekirdek",
                fs_type: "RAW Binary",
                start_lba: 2048,
                sector_count: 131_072,
                total_mb: 64,
                used_mb: 8,
                readonly: true,
                online: true,
            },
            PartitionMetrics {
                mount_point: "/system",
                label: "Bolum 2: Sistem Agaci",
                fs_type: "EROFS",
                start_lba: 133_120,
                sector_count: 4_194_304,
                total_mb: 2048,
                used_mb: 384,
                readonly: true,
                online: sys_online,
            },
            PartitionMetrics {
                mount_point: "/security",
                label: "Bolum 3: Guvenlik Kasasi",
                fs_type: "KeyVault",
                start_lba: 4_327_424,
                sector_count: 524_288,
                total_mb: 256,
                used_mb: 12,
                readonly: true,
                online: true,
            },
            PartitionMetrics {
                mount_point: "/data",
                label: "Bolum 4: Kullanici Verisi",
                fs_type: "exFAT",
                start_lba: 4_851_712,
                sector_count: 2_097_152,
                total_mb: 1024,
                used_mb: data_used_mb,
                readonly: false,
                online: data_online,
            },
            PartitionMetrics {
                mount_point: "/ram",
                label: "Gecici Bellek Diski",
                fs_type: "MemFS",
                start_lba: 0,
                sector_count: 65_536,
                total_mb: 32,
                used_mb: 4,
                readonly: false,
                online: true,
            },
        ]
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
                    if let Ok(mut res) = fs.list_dir(rel) {
                        if rel.trim_matches('/') == "app" && !res.iter().any(|e| e.name == "files.ntzc") {
                            res.insert(
                                0,
                                DirEntry {
                                    name: String::from("files.ntzc"),
                                    file_type: FileType::RegularFile,
                                    size: 256,
                                    id: 90,
                                },
                            );
                            res.insert(
                                1,
                                DirEntry {
                                    name: String::from("browser.ntzc"),
                                    file_type: FileType::RegularFile,
                                    size: 256,
                                    id: 91,
                                },
                            );
                        }
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
            "data" => {
                if let Some(ref mut fs) = self.data_fs {
                    fs.write_file(rel, content)
                } else {
                    self.ram_fs.write_file(rel, content)
                }
            }
            "ram" => self.ram_fs.write_file(rel, content),
            "system" => Err(FsError::ReadOnly),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn create_dir_path(&mut self, path: &str) -> Result<(), FsError> {
        let (mount, rel) = self.split_mount_path(path);
        match mount {
            "data" => {
                if let Some(ref mut fs) = self.data_fs {
                    fs.create_dir(rel)
                } else {
                    self.ram_fs.create_dir(rel)
                }
            }
            "ram" => self.ram_fs.create_dir(rel),
            "system" => Err(FsError::ReadOnly),
            _ => Err(FsError::NotFound),
        }
    }

    pub fn remove_path(&mut self, path: &str) -> Result<(), FsError> {
        let (mount, rel) = self.split_mount_path(path);
        match mount {
            "data" => {
                if let Some(ref mut fs) = self.data_fs {
                    fs.remove(rel)
                } else {
                    self.ram_fs.remove(rel)
                }
            }
            "ram" => self.ram_fs.remove(rel),
            "system" => Err(FsError::ReadOnly),
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
            "data" => {
                if let Some(ref mut fs) = self.data_fs {
                    fs.file_stat(rel)
                } else {
                    Ok(FileStat {
                        size: self.read_file_path(path).map(|v| v.len() as u64).unwrap_or(0),
                        is_dir: self.list_dir_path(path).is_ok(),
                        readonly: false,
                    })
                }
            }
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
                    name: String::from("documents"),
                    file_type: FileType::Directory,
                    size: 0,
                    id: 3,
                });
            } else {
                list.push(DirEntry {
                    name: String::from("build.prop"),
                    file_type: FileType::RegularFile,
                    size: 214,
                    id: 4,
                });
            }
        } else if trimmed == "app" {
            if target == PartitionTarget::System {
                for (idx, name) in [
                    "files.ntzc",
                    "browser.ntzc",
                    "settings.ntzc",
                    "system.ntzc",
                    "stopwatch.ntzc",
                ]
                .iter()
                .enumerate()
                {
                    list.push(DirEntry {
                        name: String::from(*name),
                        file_type: FileType::RegularFile,
                        size: 512,
                        id: 10 + idx as u64,
                    });
                }
            } else {
                list.push(DirEntry {
                    name: String::from("calculator.ntzc"),
                    file_type: FileType::RegularFile,
                    size: 512,
                    id: 20,
                });
            }
        } else if trimmed == "media" {
            list.push(DirEntry {
                name: String::from("aurora_cam.png"),
                file_type: FileType::RegularFile,
                size: 512,
                id: 30,
            });
            list.push(DirEntry {
                name: String::from("aero_tanitim.mp4"),
                file_type: FileType::RegularFile,
                size: 512,
                id: 31,
            });
        } else if trimmed == "documents" {
            list.push(DirEntry {
                name: String::from("hosgeldiniz.txt"),
                file_type: FileType::RegularFile,
                size: 248,
                id: 40,
            });
        } else {
            return Err(FsError::NotFound);
        }
        Ok(list)
    }

    fn fallback_read(_target: PartitionTarget, path: &str) -> Result<Vec<u8>, FsError> {
        let trimmed = path.trim_matches('/');
        if trimmed == "build.prop" {
            return Ok(b"ro.netizen.os.name=NetizenCore\nro.netizen.os.version=1.2.0\nro.netizen.arch=x86_64\nro.netizen.ui.style=EMUI-Porcelain-Aero\nro.netizen.vfs.mounts=/system,/data,/ram\n".to_vec());
        }
        if trimmed.ends_with(".png") {
            return Ok(make_sample_png(0).to_vec());
        }
        if trimmed.ends_with(".mp4") {
            return Ok(make_sample_mp4(0, 18).to_vec());
        }
        if trimmed.ends_with(".txt") {
            return Ok(b"NetizenCore Dosya Sistemi\nKalici /data (exFAT) ve /system (EROFS) bolumleri aktif.\n".to_vec());
        }
        if trimmed.ends_with(".ntzc") {
            let pkg = if trimmed.contains("files") {
                make_ntzc_pkg("nc.files", "Dosyalar", "generic", 46, 30, 1000, 7)
            } else if trimmed.contains("browser") {
                make_ntzc_pkg("nc.browser", "Tarayici", "monitor", 44, 28, 1000, 5)
            } else if trimmed.contains("settings") {
                make_ntzc_pkg("nc.settings", "Ayarlar", "sliders", 38, 24, 1000, 3)
            } else if trimmed.contains("system") {
                make_ntzc_pkg("nc.system", "Sistem", "monitor", 32, 24, 1000, 1)
            } else if trimmed.contains("stopwatch") {
                make_ntzc_pkg("nc.stopwatch", "Kronometre", "stopwatch", 26, 16, 1000, 0)
            } else {
                make_ntzc_pkg("nc.calculator", "Hesap Makinesi", "generic", 28, 22, 10001, 4)
            };
            return Ok(pkg.to_vec());
        }
        Err(FsError::NotFound)
    }
}

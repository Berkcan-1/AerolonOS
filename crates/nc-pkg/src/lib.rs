#![no_std]
extern crate alloc;

pub mod manifest;

use alloc::vec::Vec;
pub use manifest::PackageManifest;

pub const NTZC_MAGIC: [u8; 4] = *b"NTZC";
pub const PERM_SYSTEM_SETTINGS: u32 = 1 << 0;
pub const PERM_POWER_CONTROL: u32 = 1 << 1;
pub const PERM_STORAGE_FULL: u32 = 1 << 2;

#[derive(Clone, Debug)]
pub struct NtzcPackage {
    pub uid: u32,
    pub permissions: u32,
    pub manifest: PackageManifest,
    pub payload: Vec<u8>,
}

impl NtzcPackage {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 28 || &bytes[0..4] != &NTZC_MAGIC {
            return None;
        }

        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != 1 {
            return None;
        }

        let uid = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let permissions = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let manifest_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
        let payload_offset = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;
        let payload_len = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as usize;

        if bytes.len() < 28 + manifest_len || bytes.len() < payload_offset + payload_len {
            return None;
        }

        let manifest_text = core::str::from_utf8(&bytes[28..28 + manifest_len]).ok()?;
        let manifest = PackageManifest::parse(manifest_text)?;

        let mut payload = Vec::with_capacity(payload_len);
        payload.extend_from_slice(&bytes[payload_offset..payload_offset + payload_len]);

        Some(NtzcPackage {
            uid,
            permissions,
            manifest,
            payload,
        })
    }

    pub fn is_system(&self) -> bool {
        self.uid == 1000
    }

    pub fn role_label(&self) -> &'static str {
        if self.is_system() {
            "Sistem Uygulamasi"
        } else {
            "Kullanici Uygulamasi"
        }
    }

    pub fn permission_labels(&self) -> Vec<&'static str> {
        let mut list = Vec::new();
        if (self.permissions & PERM_SYSTEM_SETTINGS) != 0 {
            list.push("Gorunum ve Sistem Ayarlarini Degistirme");
        }
        if (self.permissions & PERM_POWER_CONTROL) != 0 {
            list.push("Cihazi Kapatma ve Yeniden Baslatma");
        }
        if (self.permissions & PERM_STORAGE_FULL) != 0 {
            list.push("Depolama Alanindaki Dosyalara Erisim");
        }
        if list.is_empty() {
            list.push("Temel Uygulama Izinleri (Korumali)");
        }
        list
    }
}

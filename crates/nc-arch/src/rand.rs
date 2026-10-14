use core::arch::asm;
use core::arch::x86_64::__cpuid;

pub struct HardwareRng;

static mut FALLBACK_STATE: u64 = 0;

fn has_rdrand() -> bool {
    (__cpuid(1).ecx & (1 << 30)) != 0
}

impl HardwareRng {
    pub fn next_u64() -> u64 {
        if has_rdrand() {
            for _ in 0..10 {
                let mut val: u64 = 0;
                let mut success: u8;
                unsafe {
                    asm!(
                        "rdrand {0}",
                        "setc {1}",
                        out(reg) val,
                        out(reg_byte) success,
                        options(nomem, nostack)
                    );
                }
                if success == 1 {
                    return val;
                }
            }
        }
        unsafe {
            let tsc = crate::time::read_tsc();
            FALLBACK_STATE = FALLBACK_STATE.wrapping_add(0x9E37_79B9_7F4A_7C15).wrapping_add(tsc);
            let mut z = FALLBACK_STATE;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    pub fn fill_bytes(dest: &mut [u8]) {
        let mut chunks = dest.chunks_exact_mut(8);
        for chunk in chunks.by_ref() {
            chunk.copy_from_slice(&Self::next_u64().to_le_bytes());
        }
        let remainder = chunks.into_remainder();
        if !remainder.is_empty() {
            let bytes = Self::next_u64().to_le_bytes();
            let len = remainder.len();
            remainder.copy_from_slice(&bytes[..len]);
        }
    }
}

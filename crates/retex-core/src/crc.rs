//! CRC-32 variants used by TexMod.
//!
//! TexMod's texture hash is the standard reflected CRC-32 with initial value
//! `0xFFFFFFFF` but **without the final XOR**. The ZIP container uses the
//! standard variant (with the final XOR).

pub const POLY: u32 = 0xEDB8_8320;
const INIT: u32 = 0xFFFF_FFFF;

const fn make_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { POLY ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

pub static TABLE: [u32; 256] = make_table();

/// CRC as computed by TexMod's `GetCRC32` (no final XOR).
pub fn texmod_crc(data: &[u8]) -> u32 {
    let mut crc = INIT;
    for &b in data {
        crc = (crc >> 8) ^ TABLE[((crc ^ b as u32) & 0xFF) as usize];
    }
    crc
}

/// Standard CRC-32 (as used by ZIP entries).
pub fn zip_crc(data: &[u8]) -> u32 {
    texmod_crc(data) ^ 0xFFFF_FFFF
}

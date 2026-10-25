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

/// Slicing-by-16 tables: `TABLES[k][b]` is the CRC contribution of byte `b`
/// followed by `k` zero bytes, so 16 input bytes fold in with 16 lookups and
/// no loop-carried dependency per byte.
const fn make_tables() -> [[u32; 256]; 16] {
    let mut tables = [[0u32; 256]; 16];
    tables[0] = make_table();
    let mut i = 0usize;
    while i < 256 {
        let mut k = 1;
        while k < 16 {
            let previous = tables[k - 1][i];
            tables[k][i] = (previous >> 8) ^ tables[0][(previous & 0xFF) as usize];
            k += 1;
        }
        i += 1;
    }
    tables
}

static TABLES: [[u32; 256]; 16] = make_tables();

/// CRC as computed by TexMod's `GetCRC32` (no final XOR).
///
/// Same result as the bytewise loop TexMod uses, several times faster: the
/// texture hash runs on the game's render thread, so its speed is frame time.
pub fn texmod_crc(data: &[u8]) -> u32 {
    let t = &TABLES;
    let mut crc = INIT;
    let (chunks, rest) = data.as_chunks::<16>();
    for c in chunks {
        let a = crc ^ u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        crc = t[15][(a & 0xFF) as usize]
            ^ t[14][((a >> 8) & 0xFF) as usize]
            ^ t[13][((a >> 16) & 0xFF) as usize]
            ^ t[12][(a >> 24) as usize]
            ^ t[11][c[4] as usize]
            ^ t[10][c[5] as usize]
            ^ t[9][c[6] as usize]
            ^ t[8][c[7] as usize]
            ^ t[7][c[8] as usize]
            ^ t[6][c[9] as usize]
            ^ t[5][c[10] as usize]
            ^ t[4][c[11] as usize]
            ^ t[3][c[12] as usize]
            ^ t[2][c[13] as usize]
            ^ t[1][c[14] as usize]
            ^ t[0][c[15] as usize];
    }
    for &b in rest {
        crc = (crc >> 8) ^ t[0][((crc ^ b as u32) & 0xFF) as usize];
    }
    crc
}

/// Standard CRC-32 (as used by ZIP entries).
pub fn zip_crc(data: &[u8]) -> u32 {
    texmod_crc(data) ^ 0xFFFF_FFFF
}

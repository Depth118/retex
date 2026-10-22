//! TexMod's repeating 32-bit XOR mask.

pub const XOR_KEY: u32 = 0x3FA4_3FA4;

pub fn apply(data: &mut [u8]) {
    apply_at(data, 0);
}

/// Apply the mask at an absolute file offset (the 4-byte phase matters).
pub fn apply_at(data: &mut [u8], offset: usize) {
    let mask = XOR_KEY.to_le_bytes();
    for (i, b) in data.iter_mut().enumerate() {
        *b ^= mask[(offset + i) & 3];
    }
}

pub fn apply_vec(mut data: Vec<u8>) -> Vec<u8> {
    apply(&mut data);
    data
}

//! TexMod's repeating 32-bit XOR mask.

pub const XOR_KEY: u32 = 0x3FA4_3FA4;

pub fn apply(data: &mut [u8]) {
    apply_at(data, 0);
}

/// Apply the mask at an absolute file offset (the 4-byte phase matters).
pub fn apply_at(data: &mut [u8], offset: usize) {
    let mask = XOR_KEY.to_le_bytes();
    // Eight bytes at a time: the mask repeats every four bytes, so a fixed
    // 8-byte pattern in the right phase covers every full chunk.
    let mut pattern = [0u8; 8];
    for (j, byte) in pattern.iter_mut().enumerate() {
        *byte = mask[(offset + j) & 3];
    }
    let word = u64::from_ne_bytes(pattern);
    let (chunks, rest) = data.as_chunks_mut::<8>();
    let done = chunks.len() * 8;
    for chunk in chunks {
        *chunk = (u64::from_ne_bytes(*chunk) ^ word).to_ne_bytes();
    }
    for (i, b) in rest.iter_mut().enumerate() {
        *b ^= mask[(offset + done + i) & 3];
    }
}

pub fn apply_vec(mut data: Vec<u8>) -> Vec<u8> {
    apply(&mut data);
    data
}

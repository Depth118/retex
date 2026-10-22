//! Traditional PKWARE ("ZipCrypto") encryption as used by TexMod packages.

use crate::crc::TABLE;
use rand::rngs::OsRng;
use rand::RngCore;

/// Fixed 42-byte binary password used by every TexMod package.
pub const PASSWORD: [u8; 42] = [
    0x73, 0x2A, 0x63, 0x7D, 0x5F, 0x0A, 0xA6, 0xBD, 0x7D, 0x65, 0x7E, 0x67, 0x61, 0x2A, 0x7F, 0x7F,
    0x74, 0x61, 0x67, 0x5B, 0x60, 0x70, 0x45, 0x74, 0x5C, 0x22, 0x74, 0x5D, 0x6E, 0x6A, 0x73, 0x41,
    0x77, 0x6E, 0x46, 0x47, 0x77, 0x49, 0x0C, 0x4B, 0x46, 0x6F,
];

const KEY0: u32 = 0x1234_5678;
const KEY1: u32 = 0x2345_6789;
const KEY2: u32 = 0x3456_7890;

#[derive(Clone, Copy)]
pub struct Keys {
    k0: u32,
    k1: u32,
    k2: u32,
}

impl Keys {
    pub fn new(password: &[u8]) -> Self {
        let mut keys = Keys {
            k0: KEY0,
            k1: KEY1,
            k2: KEY2,
        };
        for &b in password {
            keys.update(b);
        }
        keys
    }

    #[inline]
    fn update(&mut self, b: u8) {
        self.k0 = (self.k0 >> 8) ^ TABLE[((self.k0 ^ b as u32) & 0xFF) as usize];
        self.k1 = (self.k1.wrapping_add(self.k0 & 0xFF))
            .wrapping_mul(134_775_813)
            .wrapping_add(1);
        self.k2 = (self.k2 >> 8) ^ TABLE[((self.k2 ^ (self.k1 >> 24)) & 0xFF) as usize];
    }

    #[inline]
    fn stream_byte(&self) -> u8 {
        let t = (self.k2 | 2) & 0xFFFF;
        ((t.wrapping_mul(t ^ 1)) >> 8) as u8
    }
}

/// Encrypt `data`, returning the 12-byte header followed by the body.
pub fn encrypt(data: &[u8], password: &[u8], check_byte: u8) -> Vec<u8> {
    let mut keys = Keys::new(password);
    let mut header = [0u8; 12];
    OsRng.fill_bytes(&mut header[..11]);
    header[11] = check_byte;

    let mut out = Vec::with_capacity(data.len() + 12);
    for &b in header.iter() {
        out.push(b ^ keys.stream_byte());
        keys.update(b);
    }
    for &b in data {
        out.push(b ^ keys.stream_byte());
        keys.update(b);
    }
    out
}

/// Decrypt a ZipCrypto stream and verify the header check byte.
///
/// The check byte is the high byte of the entry CRC (from the central
/// directory, so it also works for entries with a data descriptor).
pub fn decrypt(encrypted: &[u8], password: &[u8], expected_check: u8) -> Option<Vec<u8>> {
    if encrypted.len() < 12 {
        return None;
    }
    let mut keys = Keys::new(password);
    let mut last = 0u8;
    for &b in &encrypted[..12] {
        let p = b ^ keys.stream_byte();
        keys.update(p);
        last = p;
    }
    if last != expected_check {
        return None;
    }
    let mut out = Vec::with_capacity(encrypted.len() - 12);
    for &b in &encrypted[12..] {
        let p = b ^ keys.stream_byte();
        keys.update(p);
        out.push(p);
    }
    Some(out)
}

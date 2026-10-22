//! Bytes-per-pixel table used by TexMod's texture hash.
//!
//! Values were verified by executing TexMod's logging DLL at `+0x8A50`.
//! Compressed formats use FourCC constants (`D3DFMT_DXT1` = `0x31545844`, ...).

use crate::crc::texmod_crc;

pub const DEFAULT_BPP: f32 = 0.5;

/// (D3DFORMAT, bytes per pixel) pairs that differ from the 0.5 fallback.
pub const BPP_TABLE: &[(u32, f32)] = &[
    (0x14, 3.0), // D3DFMT_R8G8B8
    (0x15, 4.0), // D3DFMT_A8R8G8B8
    (0x16, 4.0), // D3DFMT_X8R8G8B8
    (0x17, 2.0), // D3DFMT_R5G6B5
    (0x18, 2.0), // D3DFMT_X1R5G5B5
    (0x19, 2.0), // D3DFMT_A1R5G5B5
    (0x1A, 2.0), // D3DFMT_A4R4G4B4
    (0x1B, 1.0), // D3DFMT_R3G3B2
    (0x1C, 1.0), // D3DFMT_A8
    (0x1D, 2.0), // D3DFMT_A8R3G3B2
    (0x1E, 2.0), // D3DFMT_X4R4G4B4
    (0x1F, 4.0), // D3DFMT_A2B10G10R10
    (0x20, 4.0), // D3DFMT_A8B8G8R8
    (0x21, 4.0), // D3DFMT_X8B8G8R8
    (0x22, 4.0), // D3DFMT_G16R16
    (0x23, 4.0),
    (0x24, 8.0),
    (0x28, 1.0),
    (0x29, 1.0),
    (0x32, 1.0), // D3DFMT_L16
    (0x33, 2.0), // D3DFMT_A8L8
    (0x34, 1.0), // D3DFMT_A4L4
    (0x3C, 2.0), // D3DFMT_R16F
    (0x3D, 2.0), // D3DFMT_G16R16F
    (0x3E, 4.0), // D3DFMT_A16B16G16R16F
    (0x3F, 4.0), // D3DFMT_R32F
    (0x40, 4.0), // D3DFMT_G32R32F
    (0x43, 4.0), // D3DFMT_YUY2
    (0x51, 2.0),
    (0x6E, 8.0),
    (0x6F, 2.0),
    (0x70, 4.0),
    (0x71, 8.0),
    (0x72, 4.0),
    (0x73, 8.0),
    (0x74, 16.0),
    (0x75, 2.0),
    // FourCC compressed formats
    (0x3154_5844, 0.5), // D3DFMT_DXT1
    (0x3254_5844, 1.0), // D3DFMT_DXT2
    (0x3354_5844, 1.0), // D3DFMT_DXT3
    (0x3454_5844, 1.0), // D3DFMT_DXT4
    (0x3554_5844, 1.0), // D3DFMT_DXT5
];

pub fn bytes_per_pixel(format: u32) -> f32 {
    BPP_TABLE
        .iter()
        .find(|(f, _)| *f == format)
        .map(|(_, b)| *b)
        .unwrap_or(DEFAULT_BPP)
}

/// Replicate TexMod's texture checksum.
///
/// `surface` must be the raw locked-surface data. Only the first
/// `width * height * bpp` bytes are hashed; pitch is deliberately ignored,
/// matching TexMod (including its row-padding bug).
pub fn texture_hash(surface: &[u8], width: u32, height: u32, format: u32) -> u32 {
    let len = (width as f64 * height as f64 * bytes_per_pixel(format) as f64) as usize;
    let end = len.min(surface.len());
    texmod_crc(&surface[..end])
}

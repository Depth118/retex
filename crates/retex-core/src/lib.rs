//! TexMod `.tpf` package support and texture hashing.
//!
//! Clean-room implementation based on reverse engineering of TexMod 0.9b.
//! See `docs/texmod-format.md` for the full specification.

pub mod bpp;
pub mod container;
pub mod crc;
pub mod error;
pub mod index;
pub mod tpf;
pub mod xor;
pub mod zipcrypto;

pub use bpp::{bytes_per_pixel, texture_hash, BPP_TABLE};
pub use crc::{texmod_crc, zip_crc};
pub use error::{Error, Result};
pub use index::{EntryMeta, PackageIndex};
pub use tpf::{parse_hash, Package, PackageEntry};
pub use zipcrypto::PASSWORD;

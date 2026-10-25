//! Lazy `.tpf` index: parses the ZIP directory without decrypting entries and
//! reads/decrypts individual entries on demand.
//!
//! This is what the 32-bit hook uses so a large pack does not have to be held
//! in the game's address space.

use crate::container;
use crate::crc::zip_crc;
use crate::error::{Error, Result};
use crate::tpf::parse_hash;
use crate::xor;
use crate::zipcrypto::{self, PASSWORD};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const SIG_LOCAL: u32 = 0x0403_4B50;
const SIG_CENTRAL: u32 = 0x0201_4B50;
const SIG_EOCD: u32 = 0x0605_4B50;

#[derive(Debug, Clone)]
pub struct EntryMeta {
    pub name: String,
    pub hash: u32,
    /// Offset of the encrypted entry data in the raw (XORed) file.
    pub data_offset: u64,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub flags: u16,
    pub method: u16,
    pub dos_time: u16,
    pub crc: u32,
}

#[derive(Debug)]
pub struct PackageIndex {
    pub path: PathBuf,
    pub comment: Vec<u8>,
    pub entries: Vec<EntryMeta>,
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16> {
    bytes
        .get(offset..offset + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| Error::Zip("truncated".into()))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| Error::Zip("truncated".into()))
}

fn find_eocd(bytes: &[u8]) -> Result<usize> {
    if bytes.len() < 22 {
        return Err(Error::Zip("smaller than EOCD".into()));
    }
    let mut pos = bytes.len() - 22;
    loop {
        if u32_at(bytes, pos)? == SIG_EOCD {
            let comment_len = u16_at(bytes, pos + 20)? as usize;
            if pos + 22 + comment_len <= bytes.len() {
                return Ok(pos);
            }
        }
        if pos == 0 {
            break;
        }
        pos -= 1;
    }
    Err(Error::Zip("end of central directory not found".into()))
}

impl PackageIndex {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let length = file.metadata()?.len();
        if length < 22 {
            return Err(Error::Zip("file too small".into()));
        }

        // Read enough of the tail to find the EOCD and the central directory.
        let tail_length = std::cmp::min(length, 22 + 0xFFFF + 0xFFFF) as usize;
        let tail_start = length - tail_length as u64;
        let mut tail = vec![0u8; tail_length];
        file.seek(SeekFrom::Start(tail_start))?;
        file.read_exact(&mut tail)?;
        xor::apply_at(&mut tail, tail_start as usize);

        let eocd = find_eocd(&tail)?;
        let total = u16_at(&tail, eocd + 10)? as usize;
        let directory_size = u32_at(&tail, eocd + 12)? as usize;
        let directory_offset = u32_at(&tail, eocd + 16)? as u64;
        let comment_len = u16_at(&tail, eocd + 20)? as usize;
        let comment = tail
            .get(eocd + 22..eocd + 22 + comment_len)
            .ok_or_else(|| Error::Zip("truncated comment".into()))?
            .to_vec();

        // Bound every size by the file length before allocating: a corrupt
        // pack must not make the 32-bit hook attempt a multi-GiB allocation.
        if directory_offset + directory_size as u64 > length {
            return Err(Error::Zip("central directory out of range".into()));
        }
        let mut directory = vec![0u8; directory_size];
        file.seek(SeekFrom::Start(directory_offset))?;
        file.read_exact(&mut directory)?;
        xor::apply_at(&mut directory, directory_offset as usize);

        let mut entries = Vec::with_capacity(total);
        let mut p = 0usize;
        for _ in 0..total {
            if u32_at(&directory, p)? != SIG_CENTRAL {
                return Err(Error::Zip("bad central directory signature".into()));
            }
            let flags = u16_at(&directory, p + 8)?;
            let method = u16_at(&directory, p + 10)?;
            let dos_time = u16_at(&directory, p + 12)?;
            let crc = u32_at(&directory, p + 16)?;
            let compressed_size = u32_at(&directory, p + 20)?;
            let uncompressed_size = u32_at(&directory, p + 24)?;
            let name_len = u16_at(&directory, p + 28)? as usize;
            let extra_len = u16_at(&directory, p + 30)? as usize;
            let entry_comment_len = u16_at(&directory, p + 32)? as usize;
            let local_offset = u32_at(&directory, p + 42)? as u64;
            let name_bytes = directory
                .get(p + 46..p + 46 + name_len)
                .ok_or_else(|| Error::Zip("truncated name".into()))?;
            let name = String::from_utf8_lossy(name_bytes).into_owned();

            // Read the local header to locate the entry data.
            let mut local = [0u8; 30];
            file.seek(SeekFrom::Start(local_offset))?;
            file.read_exact(&mut local)?;
            xor::apply_at(&mut local, local_offset as usize);
            if u32_at(&local, 0)? != SIG_LOCAL {
                return Err(Error::Zip(format!("bad local header for {name}")));
            }
            let local_name = u16_at(&local, 26)? as u64;
            let local_extra = u16_at(&local, 28)? as u64;
            let data_offset = local_offset + 30 + local_name + local_extra;
            if data_offset + compressed_size as u64 > length {
                return Err(Error::Zip(format!("truncated data for {name}")));
            }
            if uncompressed_size as u64 > container::MAX_ENTRY_SIZE {
                return Err(Error::Zip(format!("{name} exceeds the size limit")));
            }

            entries.push(EntryMeta {
                hash: parse_hash(&name).unwrap_or(0),
                name,
                data_offset,
                compressed_size,
                uncompressed_size,
                flags,
                method,
                dos_time,
                crc,
            });
            p += 46 + name_len + extra_len + entry_comment_len;
        }

        Ok(Self {
            path,
            comment,
            entries,
        })
    }

    pub fn read_entry(&self, meta: &EntryMeta) -> Result<Vec<u8>> {
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(meta.data_offset))?;
        let mut raw = vec![0u8; meta.compressed_size as usize];
        file.read_exact(&mut raw)?;
        xor::apply_at(&mut raw, meta.data_offset as usize);

        let start = if meta.flags & 1 != 0 {
            let check = if meta.flags & 8 != 0 {
                (meta.dos_time >> 8) as u8
            } else {
                (meta.crc >> 24) as u8
            };
            zipcrypto::decrypt_in_place(&mut raw, &PASSWORD, check).ok_or(Error::Decrypt)?
        } else {
            0
        };

        let data = match meta.method {
            0 => {
                raw.drain(..start);
                raw
            }
            8 => container::inflate(&raw[start..])?,
            method => {
                return Err(Error::Zip(format!(
                    "unsupported method {method} for {}",
                    meta.name
                )))
            }
        };

        if zip_crc(&data) != meta.crc {
            return Err(Error::Crc {
                name: meta.name.clone(),
            });
        }
        Ok(data)
    }
}

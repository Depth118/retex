//! Minimal ZIP container reader/writer for TexMod packages.
//!
//! Supports the subset used by `.tpf` files: deflate or stored entries,
//! optional ZipCrypto, archive comments. No ZIP64, no multi-disk.

use crate::crc::zip_crc;
use crate::error::{Error, Result};
use crate::zipcrypto::{self, PASSWORD};
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::{Read, Write};

const SIG_LOCAL: u32 = 0x0403_4B50;
const SIG_CENTRAL: u32 = 0x0201_4B50;
const SIG_EOCD: u32 = 0x0605_4B50;

const DOS_TIME: u16 = 0;
const DOS_DATE: u16 = ((2024 - 1980) << 9) | (1 << 5) | 1;

/// Upper bound for one decompressed entry (an 8192x8192 32-bit texture).
pub(crate) const MAX_ENTRY_SIZE: u64 = 256 * 1024 * 1024;

pub struct ParsedEntry {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct ParsedZip {
    pub comment: Vec<u8>,
    pub entries: Vec<ParsedEntry>,
}

pub struct NewEntry {
    pub name: String,
    pub data: Vec<u8>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| Error::Zip("truncated".into()))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| Error::Zip("truncated".into()))
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn find_eocd(bytes: &[u8]) -> Result<usize> {
    if bytes.len() < 22 {
        return Err(Error::Zip("smaller than EOCD".into()));
    }
    let min = bytes.len().saturating_sub(22 + 0xFFFF);
    let mut pos = bytes.len() - 22;
    loop {
        if u32_at(bytes, pos)? == SIG_EOCD {
            // validate that the comment length matches the remaining bytes
            let comment_len = u16_at(bytes, pos + 20)? as usize;
            if pos + 22 + comment_len <= bytes.len() {
                return Ok(pos);
            }
        }
        if pos == min {
            break;
        }
        pos -= 1;
    }
    Err(Error::Zip("end of central directory not found".into()))
}

/// Inflate, refusing output larger than [`MAX_ENTRY_SIZE`].
pub(crate) fn inflate(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    DeflateDecoder::new(data)
        .take(MAX_ENTRY_SIZE + 1)
        .read_to_end(&mut out)
        .map_err(Error::Io)?;
    if out.len() as u64 > MAX_ENTRY_SIZE {
        return Err(Error::Zip(
            "entry exceeds the decompressed size limit".into(),
        ));
    }
    Ok(out)
}

fn deflate(data: &[u8]) -> Result<Vec<u8>> {
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).map_err(Error::Io)?;
    enc.finish().map_err(Error::Io)
}

pub fn parse(bytes: &[u8]) -> Result<ParsedZip> {
    let eocd = find_eocd(bytes)?;
    let total = u16_at(bytes, eocd + 10)? as usize;
    let cd_size = u32_at(bytes, eocd + 12)? as usize;
    let cd_offset = u32_at(bytes, eocd + 16)? as usize;
    let comment_len = u16_at(bytes, eocd + 20)? as usize;
    let comment = bytes
        .get(eocd + 22..eocd + 22 + comment_len)
        .ok_or_else(|| Error::Zip("truncated comment".into()))?
        .to_vec();

    if cd_offset
        .checked_add(cd_size)
        .is_none_or(|end| end > bytes.len())
    {
        return Err(Error::Zip("central directory out of range".into()));
    }

    let mut entries = Vec::with_capacity(total);
    let mut p = cd_offset;
    for _ in 0..total {
        if u32_at(bytes, p)? != SIG_CENTRAL {
            return Err(Error::Zip("bad central directory signature".into()));
        }
        let flags = u16_at(bytes, p + 8)?;
        let method = u16_at(bytes, p + 10)?;
        let dos_time = u16_at(bytes, p + 12)?;
        let crc = u32_at(bytes, p + 16)?;
        let csize = u32_at(bytes, p + 20)? as usize;
        let name_len = u16_at(bytes, p + 28)? as usize;
        let extra_len = u16_at(bytes, p + 30)? as usize;
        let entry_comment_len = u16_at(bytes, p + 32)? as usize;
        let local_offset = u32_at(bytes, p + 42)? as usize;
        let name_bytes = bytes
            .get(p + 46..p + 46 + name_len)
            .ok_or_else(|| Error::Zip("truncated name".into()))?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();

        // Local header
        if u32_at(bytes, local_offset)? != SIG_LOCAL {
            return Err(Error::Zip(format!("bad local signature for {name}")));
        }
        let l_name = u16_at(bytes, local_offset + 26)? as usize;
        let l_extra = u16_at(bytes, local_offset + 28)? as usize;
        let raw = local_offset
            .checked_add(30 + l_name + l_extra)
            .and_then(|start| Some(start..start.checked_add(csize)?))
            .and_then(|range| bytes.get(range))
            .ok_or_else(|| Error::Zip(format!("truncated data for {name}")))?;

        let compressed = if flags & 1 != 0 {
            // Traditional PKZIP: with the data-descriptor bit set the header
            // check byte is the high byte of the DOS time, otherwise the high
            // byte of the CRC.
            let check = if flags & 8 != 0 {
                (dos_time >> 8) as u8
            } else {
                (crc >> 24) as u8
            };
            zipcrypto::decrypt(raw, &PASSWORD, check).ok_or(Error::Decrypt)?
        } else {
            raw.to_vec()
        };

        let data = match method {
            0 => compressed,
            8 => inflate(&compressed)?,
            m => return Err(Error::Zip(format!("unsupported method {m} for {name}"))),
        };

        if zip_crc(&data) != crc {
            return Err(Error::Crc { name });
        }

        entries.push(ParsedEntry { name, data });
        p += 46 + name_len + extra_len + entry_comment_len;
    }

    Ok(ParsedZip { comment, entries })
}

fn fits_u32(value: usize, what: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| Error::Zip(format!("{what} exceeds 4 GiB (no ZIP64)")))
}

fn fits_u16(value: usize, what: &str) -> Result<u16> {
    u16::try_from(value).map_err(|_| Error::Zip(format!("{what} exceeds 65535 (no ZIP64)")))
}

pub fn build(entries: &[NewEntry], comment: &[u8]) -> Result<Vec<u8>> {
    let count = fits_u16(entries.len(), "entry count")?;
    let comment_len = fits_u16(comment.len(), "comment length")?;
    let mut local = Vec::new();
    let mut central = Vec::new();
    // (offset, crc, compressed size, uncompressed size) per entry.
    let mut records = Vec::with_capacity(entries.len());

    for entry in entries {
        let name = entry.name.as_bytes();
        let name_len = fits_u16(name.len(), "entry name length")?;
        let compressed = deflate(&entry.data)?;
        let crc = zip_crc(&entry.data);
        let encrypted = zipcrypto::encrypt(&compressed, &PASSWORD, (crc >> 24) as u8);
        let csize = fits_u32(encrypted.len(), "entry size")?;
        let usize = fits_u32(entry.data.len(), "entry size")?;
        let offset = fits_u32(local.len(), "archive size")?;

        records.push((offset, crc, csize, usize, name_len));
        push_u32(&mut local, SIG_LOCAL);
        push_u16(&mut local, 20);
        push_u16(&mut local, 0x0001); // encrypted
        push_u16(&mut local, 8); // deflate
        push_u16(&mut local, DOS_TIME);
        push_u16(&mut local, DOS_DATE);
        push_u32(&mut local, crc);
        push_u32(&mut local, csize);
        push_u32(&mut local, usize);
        push_u16(&mut local, name_len);
        push_u16(&mut local, 0);
        local.extend_from_slice(name);
        local.extend_from_slice(&encrypted);
    }

    let cd_offset = fits_u32(local.len(), "archive size")?;
    for (entry, &(offset, crc, csize, usize, name_len)) in entries.iter().zip(records.iter()) {
        let name = entry.name.as_bytes();

        push_u32(&mut central, SIG_CENTRAL);
        push_u16(&mut central, 20);
        push_u16(&mut central, 20);
        push_u16(&mut central, 0x0001);
        push_u16(&mut central, 8);
        push_u16(&mut central, DOS_TIME);
        push_u16(&mut central, DOS_DATE);
        push_u32(&mut central, crc);
        push_u32(&mut central, csize);
        push_u32(&mut central, usize);
        push_u16(&mut central, name_len);
        push_u16(&mut central, 0); // extra
        push_u16(&mut central, 0); // comment
        push_u16(&mut central, 0); // disk
        push_u16(&mut central, 0); // internal attrs
        push_u32(&mut central, 0); // external attrs
        push_u32(&mut central, offset);
        central.extend_from_slice(name);
    }

    let mut out = local;
    out.extend_from_slice(&central);
    push_u32(&mut out, SIG_EOCD);
    push_u16(&mut out, 0);
    push_u16(&mut out, 0);
    push_u16(&mut out, count);
    push_u16(&mut out, count);
    push_u32(&mut out, fits_u32(central.len(), "central directory size")?);
    push_u32(&mut out, cd_offset);
    push_u16(&mut out, comment_len);
    out.extend_from_slice(comment);
    Ok(out)
}

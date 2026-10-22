//! `.tpf` package model.

use crate::container::{self, NewEntry};
use crate::error::{Error, Result};
use crate::xor;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub name: String,
    pub hash: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct Package {
    pub comment: Vec<u8>,
    pub entries: Vec<PackageEntry>,
}

impl Package {
    pub fn comment_text(&self) -> String {
        String::from_utf8_lossy(&self.comment).into_owned()
    }

    pub fn by_hash(&self, hash: u32) -> Option<&PackageEntry> {
        self.entries.iter().find(|e| e.hash == hash)
    }
}

/// Read a `.tpf` file (XOR -> ZIP -> ZipCrypto).
pub fn read(path: impl AsRef<Path>) -> Result<Package> {
    let mut bytes = std::fs::read(path)?;
    xor::apply(&mut bytes);
    let parsed = container::parse(&bytes)?;
    let entries = parsed
        .entries
        .into_iter()
        .map(|e| {
            let hash = parse_hash(&e.name).unwrap_or(0);
            PackageEntry {
                name: e.name,
                hash,
                data: e.data,
            }
        })
        .collect();
    Ok(Package {
        comment: parsed.comment,
        entries,
    })
}

/// Write a `.tpf` file (ZIP -> ZipCrypto -> XOR).
pub fn write(path: impl AsRef<Path>, package: &Package) -> Result<()> {
    let new_entries: Vec<NewEntry> = package
        .entries
        .iter()
        .map(|e| NewEntry {
            name: e.name.clone(),
            data: e.data.clone(),
        })
        .collect();
    let mut bytes = container::build(&new_entries, &package.comment)?;
    xor::apply(&mut bytes);
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Extract the checksum from `<EXE>_0x<8 hex>.<ext>`.
pub fn parse_hash(name: &str) -> Option<u32> {
    let base = name.rsplit(['/', '\\']).next()?;
    let stem = match base.rsplit_once('.') {
        Some((s, _)) => s,
        None => base,
    };
    let lower = stem.to_ascii_lowercase();
    let idx = lower.rfind("_0x")?;
    u32::from_str_radix(&stem[idx + 3..], 16).ok()
}

/// Build the canonical entry name.
pub fn entry_name(exe: &str, hash: u32, ext: &str) -> String {
    let ext = if ext.starts_with('.') {
        ext.to_string()
    } else {
        format!(".{ext}")
    };
    format!("{exe}_0x{hash:08X}{ext}")
}

/// Parse a TexMod definition file (`0xHASH|path`, `#` comments).
pub fn parse_definition(text: &str) -> Result<Vec<(u32, String)>> {
    let mut out = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        // Some packs terminate the definition file with a NUL byte.
        let line = raw.trim_matches(|c: char| c.is_whitespace() || c == '\0');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (hash_s, file_s) = line
            .split_once('|')
            .ok_or_else(|| Error::Package(format!("line {}: missing '|'", lineno + 1)))?;
        let hash = u32::from_str_radix(hash_s.trim().trim_start_matches("0x"), 16)
            .map_err(|_| Error::Package(format!("line {}: bad checksum", lineno + 1)))?;
        let file = file_s.trim();
        if file.is_empty() {
            return Err(Error::Package(format!("line {}: empty path", lineno + 1)));
        }
        out.push((hash, file.to_string()));
    }
    Ok(out)
}

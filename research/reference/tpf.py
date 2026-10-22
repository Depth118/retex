"""TexMod `.tpf` package reader/writer and texture hash.

Clean-room implementation derived from static and dynamic analysis of
TexMod 0.9b (build 2006-04-21).  No TexMod code is used.

Format
------
A `.tpf` file is a ZIP archive obfuscated with a repeating 32-bit XOR mask and
using traditional PKWARE ("ZipCrypto") encryption with a fixed 42-byte binary
password.  Entries are deflate-compressed.

    1. XOR every 4 bytes with 0x3FA43FA4  (byte pattern A4 3F A4 3F)
    2. Read as ZIP; each entry is ZipCrypto-encrypted with PASSWORD
    3. Entry names: "<TARGET.EXE>_0x<8 hex checksum>.<bmp|dds>"
    4. Optional "texmod.def" entry: one "<0xHASH>|<filename>" line per texture
    5. The ZIP archive comment is the package comment shown by TexMod

Texture hash
------------
CRC-32 (reflected, polynomial 0xEDB88320, init 0xFFFFFFFF) with **no final
XOR**.  TexMod feeds it `bytes_per_pixel * width * height` bytes taken from
the locked D3D9 surface, ignoring the surface pitch.  That is a bug in TexMod
and must be reproduced for hash compatibility.  `bytes_per_pixel` is a float
table (DXT1 = 0.5, DXT2..DXT5 = 1.0, A8R8G8B8 = 4.0, ...); see BPP_BY_FORMAT.
"""

from __future__ import annotations

import io
import os
import struct
import sys
import time
import zipfile
import zlib

# ---------------------------------------------------------------------------
# Constants recovered from TexMod 0.9b
# ---------------------------------------------------------------------------

XOR_KEY = 0x3FA43FA4  # little-endian bytes: A4 3F A4 3F

PASSWORD = bytes(
    [
        0x73, 0x2A, 0x63, 0x7D, 0x5F, 0x0A, 0xA6, 0xBD, 0x7D, 0x65,
        0x7E, 0x67, 0x61, 0x2A, 0x7F, 0x7F, 0x74, 0x61, 0x67, 0x5B,
        0x60, 0x70, 0x45, 0x74, 0x5C, 0x22, 0x74, 0x5D, 0x6E, 0x6A,
        0x73, 0x41, 0x77, 0x6E, 0x46, 0x47, 0x77, 0x49, 0x0C, 0x4B,
        0x46, 0x6F,
    ]
)

CRC_POLY = 0xEDB88320

# D3DFMT -> bytes per pixel, as used by TexMod's checksum (float values).
# Table verified by executing TexMod's own function (DLL B + 0x8A50); any
# format not listed falls back to 0.5.
BPP_BY_FORMAT = {
    0x14: 3.0,   # D3DFMT_R8G8B8
    0x15: 4.0,   # D3DFMT_A8R8G8B8
    0x16: 4.0,   # D3DFMT_X8R8G8B8
    0x17: 2.0,   # D3DFMT_R5G6B5
    0x18: 2.0,   # D3DFMT_X1R5G5B5
    0x19: 2.0,   # D3DFMT_A1R5G5B5
    0x1A: 2.0,   # D3DFMT_A4R4G4B4
    0x1B: 1.0,   # D3DFMT_R3G3B2
    0x1C: 1.0,   # D3DFMT_A8
    0x1D: 2.0,   # D3DFMT_A8R3G3B2
    0x1E: 2.0,   # D3DFMT_X4R4G4B4
    0x1F: 4.0,   # D3DFMT_A2B10G10R10
    0x20: 4.0,   # D3DFMT_A8B8G8R8
    0x21: 4.0,   # D3DFMT_X8B8G8R8
    0x22: 4.0,   # D3DFMT_G16R16
    0x23: 4.0,
    0x24: 8.0,
    0x28: 1.0,
    0x29: 1.0,
    0x32: 1.0,   # D3DFMT_L16
    0x33: 2.0,   # D3DFMT_A8L8
    0x34: 1.0,   # D3DFMT_A4L4
    0x3C: 2.0,   # D3DFMT_R16F
    0x3D: 2.0,   # D3DFMT_G16R16F
    0x3E: 4.0,   # D3DFMT_A16B16G16R16F
    0x3F: 4.0,   # D3DFMT_R32F
    0x40: 4.0,   # D3DFMT_G32R32F
    0x43: 4.0,   # D3DFMT_YUY2
    0x51: 2.0,
    0x6E: 8.0,
    0x6F: 2.0,
    0x70: 4.0,
    0x71: 8.0,
    0x72: 4.0,
    0x73: 8.0,
    0x74: 16.0,
    0x75: 2.0,
    # FourCC formats (real D3DFMT values for compressed textures)
    0x31545844: 0.5,  # D3DFMT_DXT1
    0x32545844: 1.0,  # D3DFMT_DXT2
    0x33545844: 1.0,  # D3DFMT_DXT3
    0x34545844: 1.0,  # D3DFMT_DXT4
    0x35545844: 1.0,  # D3DFMT_DXT5
}
DEFAULT_BPP = 0.5  # TexMod falls back to 0.5 for unknown formats


# ---------------------------------------------------------------------------
# XOR obfuscation
# ---------------------------------------------------------------------------

def xor_tpf(data: bytes) -> bytes:
    """Apply (or undo) TexMod's repeating 32-bit XOR mask."""
    n = len(data)
    if n == 0:
        return b""
    pattern = struct.pack("<I", XOR_KEY)
    mask = (pattern * (n // 4 + 1))[:n]
    # big-int XOR runs in C and is orders of magnitude faster than a loop
    return (
        int.from_bytes(data, "little") ^ int.from_bytes(mask, "little")
    ).to_bytes(n, "little")


# ---------------------------------------------------------------------------
# CRC-32 variant used by TexMod (no final XOR)
# ---------------------------------------------------------------------------

def _make_crc_table() -> list[int]:
    table = []
    for i in range(256):
        c = i
        for _ in range(8):
            c = (c >> 1) ^ CRC_POLY if c & 1 else c >> 1
        table.append(c & 0xFFFFFFFF)
    return table


_CRC_TABLE = _make_crc_table()


def texmod_crc(data: bytes) -> int:
    """CRC-32 as computed by TexMod: init 0xFFFFFFFF, no final XOR."""
    crc = 0xFFFFFFFF
    for b in data:
        crc = (crc >> 8) ^ _CRC_TABLE[(crc ^ b) & 0xFF]
    return crc & 0xFFFFFFFF


def bytes_per_pixel(fmt: int) -> float:
    """TexMod's bytes-per-pixel for a D3DFORMAT value."""
    return BPP_BY_FORMAT.get(fmt, DEFAULT_BPP)


def texture_hash(surface: bytes, width: int, height: int, fmt: int) -> int:
    """Replicate TexMod's texture checksum.

    `surface` must be the raw locked-surface data (as uploaded to D3D9).
    Only the first int(width * height * bpp) bytes are hashed; pitch is
    deliberately ignored, matching TexMod.
    """
    length = int(width * height * bytes_per_pixel(fmt))
    return texmod_crc(surface[:length])


# ---------------------------------------------------------------------------
# ZipCrypto (traditional PKWARE encryption)
# ---------------------------------------------------------------------------

def _zipcrypto_keys(password: bytes) -> tuple[int, int, int]:
    k0, k1, k2 = 0x12345678, 0x23456789, 0x34567890
    for b in password:
        k0 = (k0 >> 8) ^ _CRC_TABLE[(k0 ^ b) & 0xFF]
        k1 = ((k1 + (k0 & 0xFF)) * 134775813 + 1) & 0xFFFFFFFF
        k2 = (k2 >> 8) ^ _CRC_TABLE[(k2 ^ (k1 >> 24)) & 0xFF]
    return k0, k1, k2


def _zipcrypto_update(keys: list[int], b: int) -> None:
    k0, k1, k2 = keys
    k0 = (k0 >> 8) ^ _CRC_TABLE[(k0 ^ b) & 0xFF]
    k1 = ((k1 + (k0 & 0xFF)) * 134775813 + 1) & 0xFFFFFFFF
    k2 = (k2 >> 8) ^ _CRC_TABLE[(k2 ^ (k1 >> 24)) & 0xFF]
    keys[0], keys[1], keys[2] = k0, k1, k2


def _zipcrypto_byte(keys: list[int]) -> int:
    t = (keys[2] | 2) & 0xFFFF
    return ((t * (t ^ 1)) >> 8) & 0xFF


def zipcrypto_encrypt(data: bytes, password: bytes, check_byte: int) -> bytes:
    """Encrypt `data` with ZipCrypto, returning the 12-byte header + body."""
    keys = list(_zipcrypto_keys(password))
    header = bytearray(os.urandom(11))
    header.append(check_byte)
    out = bytearray()
    for b in header:
        out.append(b ^ _zipcrypto_byte(keys))
        _zipcrypto_update(keys, b)
    for b in data:
        out.append(b ^ _zipcrypto_byte(keys))
        _zipcrypto_update(keys, b)
    return bytes(out)


def zipcrypto_decrypt(data: bytes, password: bytes) -> bytes:
    """Decrypt a ZipCrypto stream (header included), verifying the check byte
    against `data`'s caller-supplied expectation is left to the caller."""
    keys = list(_zipcrypto_keys(password))
    out = bytearray()
    for b in data:
        p = b ^ _zipcrypto_byte(keys)
        out.append(p)
        _zipcrypto_update(keys, p)
    return bytes(out[12:])


# ---------------------------------------------------------------------------
# Reading
# ---------------------------------------------------------------------------

class TpfEntry:
    __slots__ = ("name", "hash", "data")

    def __init__(self, name: str, hash_value: int, data: bytes):
        self.name = name
        self.hash = hash_value
        self.data = data

    def __repr__(self) -> str:
        return f"TpfEntry({self.name!r}, 0x{self.hash:08X}, {len(self.data)} bytes)"


class Tpf:
    def __init__(self, comment: bytes, entries: list[TpfEntry]):
        self.comment = comment
        self.entries = entries

    @property
    def comment_text(self) -> str:
        return self.comment.decode("latin1", "replace")

    def by_hash(self, hash_value: int) -> TpfEntry | None:
        for e in self.entries:
            if e.hash == hash_value:
                return e
        return None


def parse_entry_name(name: str) -> int | None:
    """Extract the checksum from '<EXE>_0x<8 hex>.<ext>'."""
    base = os.path.splitext(os.path.basename(name))[0]
    marker = base.rfind("_0x")
    if marker < 0:
        marker = base.rfind("_0X")
    if marker < 0:
        return None
    try:
        return int(base[marker + 3:], 16)
    except ValueError:
        return None


def read_tpf(path: str, load_data: bool = True) -> Tpf:
    raw = open(path, "rb").read()
    data = xor_tpf(raw)
    zf = zipfile.ZipFile(io.BytesIO(data))
    entries = []
    for info in zf.infolist():
        if info.filename.endswith(".def"):
            continue
        payload = zf.read(info, pwd=PASSWORD) if load_data else b""
        hv = parse_entry_name(info.filename)
        entries.append(TpfEntry(info.filename, hv if hv is not None else 0, payload))
    return Tpf(zf.comment, entries)


# ---------------------------------------------------------------------------
# Writing
# ---------------------------------------------------------------------------

def _dos_time(t: time.struct_time | None = None) -> tuple[int, int]:
    t = t or time.localtime()
    dtime = (t.tm_hour << 11) | (t.tm_min << 5) | (t.tm_sec // 2)
    ddate = ((t.tm_year - 1980) << 9) | (t.tm_mon << 5) | t.tm_mday
    return dtime, ddate


def _deflate(data: bytes) -> bytes:
    c = zlib.compressobj(9, zlib.DEFLATED, -15)
    return c.compress(data) + c.flush()


def build_zip(entries: list[tuple[str, bytes]], comment: bytes = b"") -> bytes:
    """Create a ZipCrypto-encrypted ZIP in memory (the TexMod dialect)."""
    dtime, ddate = _dos_time()
    local = bytearray()
    central = bytearray()
    offsets = []

    for name, payload in entries:
        name_b = name.encode("latin1")
        comp = _deflate(payload)
        crc = zlib.crc32(payload) & 0xFFFFFFFF
        enc = zipcrypto_encrypt(comp, PASSWORD, (crc >> 24) & 0xFF)
        csize = len(enc)
        usize = len(payload)

        offsets.append(len(local))
        local += struct.pack(
            "<IHHHHHIIIHH",
            0x04034B50, 20, 0x0001, 8, dtime, ddate,
            crc, csize, usize, len(name_b), 0,
        )
        local += name_b
        local += enc

    cd_start = len(local)
    for (name, payload), off in zip(entries, offsets):
        name_b = name.encode("latin1")
        comp = _deflate(payload)
        crc = zlib.crc32(payload) & 0xFFFFFFFF
        csize = len(comp) + 12
        central += struct.pack(
            "<IHHHHHHIIIHHHHHII",
            0x02014B50, 20, 20, 0x0001, 8, dtime, ddate,
            crc, csize, len(payload), len(name_b), 0, 0, 0, 0, 0, off,
        )
        central += name_b

    eocd = struct.pack(
        "<IHHHHIIH",
        0x06054B50, 0, 0, len(entries), len(entries),
        len(central), cd_start, len(comment),
    ) + comment

    return bytes(local + central + eocd)


def write_tpf(path: str, entries: list[tuple[str, bytes]], comment: bytes = b"") -> None:
    """Write a .tpf package. `entries` is [(name, data), ...]."""
    zip_bytes = build_zip(entries, comment)
    open(path, "wb").write(xor_tpf(zip_bytes))


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _cmd_list(args: list[str]) -> int:
    tpf = read_tpf(args[0], load_data=False)
    print(f"entries: {len(tpf.entries)}")
    print(f"comment: {tpf.comment_text!r}")
    for e in tpf.entries[: int(args[1]) if len(args) > 1 else 20]:
        print(f"  0x{e.hash:08X}  {e.name}")
    return 0


def _cmd_extract(args: list[str]) -> int:
    tpf = read_tpf(args[0])
    out = args[1]
    limit = int(args[2]) if len(args) > 2 else len(tpf.entries)
    os.makedirs(out, exist_ok=True)
    for e in tpf.entries[:limit]:
        open(os.path.join(out, e.name), "wb").write(e.data)
    print(f"extracted {min(limit, len(tpf.entries))} file(s) to {out}")
    return 0


def _cmd_build(args: list[str]) -> int:
    tpf_path, def_path = args[0], args[1]
    exe_name = args[2] if len(args) > 2 else "TEXMOD.EXE"
    comment = args[3].encode("latin1") if len(args) > 3 else b""
    entries = []
    lines = []
    for line in open(def_path, encoding="latin1"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        hash_s, file_s = line.split("|", 1)
        hv = int(hash_s, 16)
        name = f"{exe_name}_0x{hv:08X}{os.path.splitext(file_s)[1].lower()}"
        entries.append((name, open(file_s, "rb").read()))
        lines.append(f"0x{hv:08X}|{name}")
    entries.append(("texmod.def", ("\r\n".join(lines) + "\r\n").encode("latin1")))
    write_tpf(tpf_path, entries, comment)
    print(f"wrote {tpf_path} ({len(entries)} entries)")
    return 0


def _cmd_hash(args: list[str]) -> int:
    from struct import unpack_from

    d = open(args[0], "rb").read()
    if d[:2] != b"BM":
        print("not a BMP")
        return 2
    offbits = unpack_from("<I", d, 10)[0]
    w, h = unpack_from("<ii", d, 18)
    bpp = unpack_from("<H", d, 28)[0]
    pixels = d[offbits:offbits + abs(w * h) * (bpp // 8)]
    print(f"0x{texmod_crc(pixels):08X}")
    return 0


def _cmd_verify(args: list[str]) -> int:
    """Validate a package without keeping all payloads in memory."""
    raw = xor_tpf(open(args[0], "rb").read())
    zf = zipfile.ZipFile(io.BytesIO(raw))
    names = [i.filename for i in zf.infolist()]

    def_map: dict[str, int] = {}
    if "texmod.def" in names:
        text = zf.read("texmod.def", pwd=PASSWORD).decode("latin1")
        for line in text.splitlines():
            if "|" in line:
                h, f = line.split("|", 1)
                try:
                    def_map[f.strip()] = int(h.strip(), 16)
                except ValueError:
                    pass

    ok = bad = mismatched = 0
    total = 0
    for info in zf.infolist():
        if info.filename.endswith(".def"):
            continue
        data = zf.read(info, pwd=PASSWORD)
        total += len(data)
        if data[:2] != b"BM" or len(data) < 54:
            bad += 1
            continue
        bmp_off = struct.unpack_from("<I", data, 10)[0]
        w, h = struct.unpack_from("<ii", data, 18)
        need = bmp_off + abs(w) * abs(h) * 4
        if need > len(data):
            bad += 1
            continue
        ok += 1
        parsed = parse_entry_name(info.filename)
        if info.filename in def_map and def_map[info.filename] != parsed:
            mismatched += 1

    missing = [n for n in def_map if n not in names]
    print(f"entries:        {ok + bad}")
    print(f"valid bitmaps:  {ok}")
    print(f"invalid:        {bad}")
    print(f"payload bytes:  {total:,}")
    print(f"texmod.def:     {'yes' if def_map else 'no'} ({len(def_map)} entries)")
    print(f"def mismatches: {mismatched}")
    print(f"def references missing files: {len(missing)}")
    print(f"comment:        {zf.comment.decode('latin1', 'replace')!r}")
    return 0 if bad == 0 and mismatched == 0 and not missing else 1


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 0
    cmd, rest = argv[1], argv[2:]
    if cmd == "list":
        return _cmd_list(rest)
    if cmd == "extract":
        return _cmd_extract(rest)
    if cmd == "build":
        return _cmd_build(rest)
    if cmd == "hash":
        return _cmd_hash(rest)
    if cmd == "verify":
        return _cmd_verify(rest)
    print(f"unknown command: {cmd}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

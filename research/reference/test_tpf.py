"""Self-test for tools/tpf.py. Run from the repository root:

    python tests/test_tpf.py
"""

import io
import os
import struct
import sys
import tempfile
import zipfile
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import tpf  # noqa: E402


def make_bmp(width: int, height: int, pixel: bytes = b"\x20\x60\xA0\xFF") -> bytes:
    """Minimal 32-bit BMP with the same layout TexMod writes."""
    pixels = pixel * (width * height)
    size = 54 + len(pixels) + 2
    header = struct.pack("<2sIHHI", b"BM", size, 0, 0, 54)
    dib = struct.pack("<IiiHHIIiiII", 40, width, height, 1, 32, 0, len(pixels) + 2, 0, 0, 0, 0)
    return header + dib + pixels + b"\x00\x00"


def main() -> int:
    # CRC variant check value (complement of standard CRC-32)
    assert tpf.texmod_crc(b"123456789") == 0x340BC6D9, "CRC variant mismatch"

    # XOR is an involution
    blob = os.urandom(1001)
    assert tpf.xor_tpf(tpf.xor_tpf(blob)) == blob

    # bpp table (values verified by executing TexMod's own function)
    assert tpf.bytes_per_pixel(0x31545844) == 0.5   # DXT1
    assert tpf.bytes_per_pixel(0x33545844) == 1.0   # DXT3
    assert tpf.bytes_per_pixel(0x15) == 4.0         # A8R8G8B8
    assert tpf.bytes_per_pixel(0xFFFF) == 0.5       # unknown fallback

    # hash length ignores pitch: 4x2 A8R8G8B8 = 32 bytes
    surface = bytes(range(64))
    assert tpf.texture_hash(surface, 4, 2, 0x15) == tpf.texmod_crc(surface[:32])

    bmp = make_bmp(8, 8)
    name = "TEST.EXE_0x802F681B.bmp"
    comment = b"unit test package"

    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "test.tpf")
        tpf.write_tpf(path, [(name, bmp)], comment)

        # our reader
        pkg = tpf.read_tpf(path)
        assert pkg.comment == comment
        assert len(pkg.entries) == 1
        entry = pkg.entries[0]
        assert entry.name == name
        assert entry.hash == 0x802F681B
        assert entry.data == bmp

        # stock zipfile must accept it with the password
        z = zipfile.ZipFile(io.BytesIO(tpf.xor_tpf(open(path, "rb").read())))
        assert z.read(name, pwd=tpf.PASSWORD) == bmp
        assert z.getinfo(name).CRC == zlib.crc32(bmp) & 0xFFFFFFFF

        # definition-file build path
        tex_path = os.path.join(tmp, "tex.bmp")
        def_path = os.path.join(tmp, "pkg.def")
        open(tex_path, "wb").write(bmp)
        open(def_path, "w").write("0x802F681B|%s\n" % tex_path)
        built = os.path.join(tmp, "built.tpf")
        tpf._cmd_build([built, def_path, "TEST.EXE", "built by test"])
        pkg2 = tpf.read_tpf(built)
        assert len(pkg2.entries) == 1
        assert pkg2.entries[0].hash == 0x802F681B
        assert pkg2.entries[0].data == bmp
        assert pkg2.comment == b"built by test"

    print("ok: CRC, XOR, bpp table, reader, writer, stock-zipfile interop")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


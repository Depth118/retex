import struct, zipfile, binascii

ZIP = r'work\redux_decrypted.zip'

# CRC32 table
tab = []
for i in range(256):
    c = i
    for _ in range(8):
        c = (c >> 1) ^ 0xEDB88320 if c & 1 else c >> 1
    tab.append(c)

def crc32_byte(crc, b):
    return (crc >> 8) ^ tab[(crc ^ b) & 0xFF]

class ZC:
    def __init__(self, pwd=b''):
        self.k0, self.k1, self.k2 = 0x12345678, 0x23456789, 0x34567890
        for b in pwd:
            self.update(b)
    def update(self, b):
        self.k0 = crc32_byte(self.k0, b)
        self.k1 = ((self.k1 + (self.k0 & 0xFF)) * 134775813 + 1) & 0xFFFFFFFF
        self.k2 = crc32_byte(self.k2, (self.k1 >> 24) & 0xFF)
    def byte(self):
        t = (self.k2 | 2) & 0xFFFF
        return ((t * (t ^ 1)) >> 8) & 0xFF
    def decrypt(self, data):
        out = bytearray(len(data))
        for i, c in enumerate(data):
            p = c ^ self.byte()
            out[i] = p
            self.update(p)
        return bytes(out)

z = zipfile.ZipFile(ZIP)
info = z.infolist()[0]
print('entry:', info.filename, 'flags=0x%X crc=0x%08X' % (info.flag_bits, info.CRC))

# read raw local header + encrypted data
with open(ZIP,'rb') as f:
    f.seek(info.header_offset)
    lh = f.read(30)
    sig, ver, flags, method, mt, md, crc, csz, usz, nl, el = struct.unpack('<IHHHHHIIIHH', lh)
    name = f.read(nl); f.read(el)
    enc = f.read(12)
print('local flags=0x%X crc=0x%08X name=%r' % (flags, crc, name))

def check(pwd):
    zc = ZC(pwd)
    hdr = zc.decrypt(enc)
    return hdr[11], hdr

exp = (info.CRC >> 24) & 0xFF
print('expected check byte 0x%02X' % exp)
print('empty password ->', check(b''))

# test candidates
cands = [b'TexMod', b'texmod', b'TEXMOD', b'TRUEEXE.EXE', b'Speed', b'speed']
for c in cands:
    b, hdr = check(c)
    print('%-14r -> 0x%02X %s' % (c, b, 'MATCH' if b == exp else ''))

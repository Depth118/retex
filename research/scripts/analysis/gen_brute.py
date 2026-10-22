import struct, zipfile, os

ZIP = r'work\redux_decrypted.zip'
out = r'work\brute_input.txt'

z = zipfile.ZipFile(ZIP)
infos = z.infolist()[:2]
lines = []
with open(ZIP,'rb') as f:
    for info in infos:
        f.seek(info.header_offset)
        lh = f.read(30)
        sig, ver, flags, method, mt, md, crc, csz, usz, nl, el = struct.unpack('<IHHHHHIIIHH', lh)
        assert flags & 1, 'not encrypted'
        f.seek(info.header_offset + 30 + nl + el)
        enc = f.read(12)
        lines.append('%s %02X' % (enc.hex(), (info.CRC >> 24) & 0xFF))
open(out,'w').write('\n'.join(lines))
print(open(out).read())

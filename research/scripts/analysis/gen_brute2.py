import struct, zipfile

ZIP = r'work\redux_decrypted.zip'
out = r'work\brute_input.txt'

z = zipfile.ZipFile(ZIP)
lines = []
with open(ZIP,'rb') as f:
    for info in z.infolist()[:2]:
        f.seek(info.header_offset)
        sig, ver, flags, method, mt, md, crc, csz, usz, nl, el = struct.unpack('<IHHHHHIIIHH', f.read(30))
        data_off = info.header_offset + 30 + nl + el
        f.seek(data_off)
        enc = f.read(12)
        lines.append('%s %02X %d %d %08X' % (enc.hex(), (info.CRC >> 24) & 0xFF, data_off, csz, info.CRC))
open(out,'w').write('\n'.join(lines))
print(open(out).read())

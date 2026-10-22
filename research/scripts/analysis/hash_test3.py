import zipfile, zlib, struct

ZIP = r'work\redux_decrypted.zip'
pwd = bytes([0x73,0x2A,0x63,0x7D,0x5F,0x0A,0xA6,0xBD,0x7D,0x65,0x7E,0x67,0x61,0x2A,0x7F,0x7F,0x74,0x61,0x67,0x5B,0x60,0x70,0x45,0x74,0x5C,0x22,0x74,0x5D,0x6E,0x6A,0x73,0x41,0x77,0x6E,0x46,0x47,0x77,0x49,0x0C,0x4B,0x46,0x6F])

def texmod_crc(b):
    return (zlib.crc32(b) ^ 0xFFFFFFFF) & 0xFFFFFFFF

# sanity check: CRC-32 check value for "123456789" is 0xCBF43926 (with xorout); ours should be the complement
print('sanity 123456789 -> 0x%08X (expect 0x340BC6D9)' % texmod_crc(b'123456789'))

z = zipfile.ZipFile(ZIP)
infos = [i for i in z.infolist() if i.filename.endswith('.bmp')]
print('bmp entries:', len(infos))
matches = 0
for i in infos[:40]:
    target = int(i.filename.rsplit('_0x',1)[1].split('.')[0], 16)
    d = z.read(i.filename, pwd=pwd)
    off = struct.unpack_from('<I', d, 10)[0]
    bi, w, h, p, bpp, comp, isz = struct.unpack_from('<IiiHHII', d, 14)
    pix = d[off:off + w*h*4]
    c = texmod_crc(pix)
    ok = c == target
    if ok: matches += 1
    print('%-45s w=%-5d h=%-5d target=0x%08X crc=0x%08X %s' % (i.filename, w, h, target, c, 'MATCH' if ok else ''))
print('matches:', matches, '/', min(40, len(infos)))

print('\n--- texmod.def (first 12 lines) ---')
try:
    defn = z.read('texmod.def', pwd=pwd).decode('latin1', 'replace')
    for line in defn.splitlines()[:12]:
        print('  ', line)
    print('  ... total lines:', len(defn.splitlines()))
except Exception as e:
    print('err', e)

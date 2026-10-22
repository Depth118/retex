import zipfile, struct, zlib

ZIP = r'work\redux_decrypted.zip'
pwd = bytes([0x73,0x2A,0x63,0x7D,0x5F,0x0A,0xA6,0xBD,0x7D,0x65,0x7E,0x67,0x61,0x2A,0x7F,0x7F,0x74,0x61,0x67,0x5B,0x60,0x70,0x45,0x74,0x5C,0x22,0x74,0x5D,0x6E,0x6A,0x73,0x41,0x77,0x6E,0x46,0x47,0x77,0x49,0x0C,0x4B,0x46,0x6F])

z = zipfile.ZipFile(ZIP)
info = z.infolist()[0]
target = int(info.filename.rsplit('_0x',1)[1].split('.')[0], 16)
d = z.read(info.filename, pwd=pwd)
offbits = struct.unpack_from('<I', d, 10)[0]
biSize, w, h, planes, bpp, comp, imgsize = struct.unpack_from('<IiiHHII', d, 14)
pix = d[offbits:offbits + w*h*4]
print('w=%d h=%d bpp=%d pixlen=%d target=0x%08X' % (w, h, bpp, len(pix), target))

def crc(b, init=0):
    return zlib.crc32(b, init) & 0xFFFFFFFF

def adler(b):
    return zlib.adler32(b) & 0xFFFFFFFF

def flip_rows(b, w, h):
    stride = w*4
    out = bytearray(len(b))
    for y in range(h):
        out[(h-1-y)*stride:(h-y)*stride] = b[y*stride:(y+1)*stride]
    return bytes(out)

def swap_rb(b):
    out = bytearray(b)
    out[0::4], out[2::4] = b[2::4], b[0::4]
    return bytes(out)

variants = {
    'pixels': pix,
    'pixels flip': flip_rows(pix, w, h),
    'pixels swapRB': swap_rb(pix),
    'pixels flip+swap': swap_rb(flip_rows(pix, w, h)),
    'pixels + 2 extra': d[offbits:],
    'whole file': d,
}
for name, v in variants.items():
    print('%-20s crc=0x%08X adler=0x%08X %s' % (name, crc(v), adler(v), 'MATCH-CRC' if crc(v)==target else ('MATCH-ADLER' if adler(v)==target else '')))

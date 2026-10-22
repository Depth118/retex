import zipfile, struct, zlib, binascii

ZIP = r'work\redux_decrypted.zip'
pwd = bytes([0x73,0x2A,0x63,0x7D,0x5F,0x0A,0xA6,0xBD,0x7D,0x65,0x7E,0x67,0x61,0x2A,0x7F,0x7F,0x74,0x61,0x67,0x5B,0x60,0x70,0x45,0x74,0x5C,0x22,0x74,0x5D,0x6E,0x6A,0x73,0x41,0x77,0x6E,0x46,0x47,0x77,0x49,0x0C,0x4B,0x46,0x6F])

z = zipfile.ZipFile(ZIP)
info = z.infolist()[0]
name = info.filename
target = int(name.rsplit('_0x',1)[1].split('.')[0], 16)
print('entry', name, 'target hash 0x%08X' % target)
d = z.read(name, pwd=pwd)

# BMP file header
sig, bfsize, r1, r2, offbits = struct.unpack_from('<2sIHHI', d, 0)
print('BMP sig=%r bfsize=%d offbits=%d' % (sig, bfsize, offbits))
biSize, w, h, planes, bpp, comp, imgsize = struct.unpack_from('<IiiHHII', d, 14)
print('DIB size=%d w=%d h=%d planes=%d bpp=%d comp=%d imgsize=%d' % (biSize, w, h, planes, bpp, comp, imgsize))
print('file len', len(d), 'offbits+imgsize', offbits + imgsize)

pix = d[offbits:]
print('pixels', len(pix), 'expected', abs(w*h*bpp//8))
variants = {
  'whole file': d,
  'from offbits': pix,
  'from 54': d[54:],
  'from 56': d[56:],
  'rgb only from offbits': b''.join(pix[i:i+3] for i in range(0, len(pix), 4)),
  'first row': pix[:w*4],
}
for k, v in variants.items():
    c = zlib.crc32(v) & 0xFFFFFFFF
    print('%-24s crc32=0x%08X %s' % (k, c, 'MATCH' if c == target else ''))

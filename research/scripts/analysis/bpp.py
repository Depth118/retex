import struct

d = bytes(c ^ 0xAA for c in open(r'work\tm_b_unpacked.xor','rb').read())
BASE = 0x10000000
def off(va): return va - BASE
def u32(va): return struct.unpack_from('<I', d, off(va))[0]
def f32(va): return struct.unpack_from('<f', d, off(va))[0]

float_addrs = {
    0x10008B3C: 0x10074618,
    0x10008B44: 0x10074614,
    0x10008B4C: 0x10074610,
    0x10008B54: 0x1007460C,
    0x10008B5C: 0x10074608,
    0x10008B64: 0x100744E4,
    0x10008B6C: 0x100744D8,
    0x10008B74: 0x100744D8,
}
print('float constants:')
for a, fa in float_addrs.items():
    print('  0x%08X -> 0x%08X = %g' % (a, fa, f32(fa)))

def target_for(fmt):
    if 0x14 <= fmt <= 0x24:
        idx = fmt - 0x14
        b = d[off(0x10008B97) + idx]
        return u32(0x10008B83 + b*4)
    if 0x32 <= fmt <= 0x72:
        idx = fmt - 0x32
        b = d[off(0x10008BBC) + idx]
        return u32(0x10008BA8 + b*4)
    special = {
        0x31545844: 0x10008B6C,  # DXT1
        0x32545844: 0x10008B64,  # DXT2
        0x33545844: 0x10008B64,  # DXT3
        0x34545844: 0x10008B64,  # DXT4
        0x35545844: 0x10008B64,  # DXT5
        0x42475247: 0x10008B5C,  # GRGB
        0x47424752: 0x10008B5C,  # RGBG
        0x73: 0x10008B44,
        0x74: 0x10008B3C,
        0x75: 0x10008B5C,
    }
    return special.get(fmt, 0x10008B74)

names = {
 1:'R8G8B8',2:'A8R8G8B8',3:'X8R8G8B8',4:'R5G6B5',5:'X1R5G5B5',6:'A1R5G5B5',7:'A4R4G4B4',
 8:'R3G3B2',9:'A8',10:'A8R3G3B2',11:'X4R4G4B4',12:'A4B4G4R4',13:'X4B4G4R4',
 20:'R8G8B8',21:'A8R8G8B8',22:'X8R8G8B8',23:'R5G6B5',24:'X1R5G5B5',25:'A1R5G5B5',
 26:'A4R4G4B4',27:'R3G3B2',28:'A8',29:'A8R3G3B2',30:'X4R4G4B4',31:'A4B4G4R4',32:'X4B4G4R4',
 50:'L16',51:'A8L8',52:'A4L4',53:'V8U8',54:'L6V5U5',55:'X8L8V8U8',56:'Q8W8V8U8',
 57:'V16U16',58:'A2W10V10U10',59:'Q16W16V16U16',60:'R16F',61:'G16R16F',62:'A16B16G16R16F',
 63:'R32F',64:'G32R32F',65:'A32B32G32R32F',66:'UYVY',67:'YUY2',68:'DXT1',69:'DXT2',
 70:'DXT3',71:'DXT4',72:'DXT5',73:'D16_LOCKABLE',74:'D32',75:'D15S1',76:'D24S8',77:'D16',
 78:'D32F_LOCKABLE',79:'D24X8',80:'D24X4S4',81:'D16_LOCKABLE',82:'D32F_LOCKABLE',
 83:'D24FS8',84:'D32_LOCKABLE',85:'S8_LOCKABLE',86:'A8',87:'L8',88:'A1',89:'A4L4',
 90:'DXT1'
}

print('\n D3DFMT  name              bpp   target')
for fmt in list(range(0x14, 0x25)) + list(range(0x32, 0x73)) + [0x31545844,0x32545844,0x33545844,0x34545844,0x35545844,0x73,0x74,0x75,0x42475247,0x47424752]:
    t = target_for(fmt)
    bpp = f32(float_addrs.get(t, 0x100744D8))
    label = names.get(fmt if fmt < 0x100 else 0, hex(fmt))
    if fmt >= 0x100:
        label = {0x31545844:'DXT1',0x32545844:'DXT2',0x33545844:'DXT3',0x34545844:'DXT4',0x35545844:'DXT5',0x42475247:'GRGB',0x47424752:'RGBG'}.get(fmt, hex(fmt))
    print(' 0x%02X   %-16s %-6g 0x%08X' % (fmt if fmt < 0x100 else fmt & 0xFF, label, bpp, t))

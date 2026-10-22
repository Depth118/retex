import struct, re
from capstone import *
from capstone.x86 import *

d = bytes(c ^ 0xAA for c in open(r'work\tm_b_unpacked.xor','rb').read())
BASE = 0x10000000
print('len', hex(len(d)))

def find_str(s):
    b = s.encode('latin1')
    i = d.find(b)
    return i

for s in ['checksum: 0x%.8X', 'Textures loaded: %i', 'Selected Texture', 'ReplaceTexture', 'd3d9.dll', 'Direct3DCreate9']:
    off = find_str(s)
    print('%-28r off=0x%X va=0x%X' % (s, off, BASE+off if off>=0 else 0))

# find CRC poly constant as immediate bytes
for pat, name in [(b'\x20\x83\xb8\xed', 'poly ED B8 83 20 (little endian)'),
                  (b'\x20\x83\xb8\xed', 'poly')]:
    offs = []
    pos = 0
    while True:
        i = d.find(pat, pos)
        if i < 0: break
        offs.append(i); pos = i+1
    print(name, [hex(o) for o in offs][:10])

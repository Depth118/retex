import re
from capstone import *
from capstone.x86 import *

d = bytes(c ^ 0xAA for c in open(r'work\tm_a_unpacked.xor','rb').read())
print('len', hex(len(d)))
# find key init
for pat, name in ((b'\x78\x56\x34\x12','k0'),(b'\x89\x67\x45\x23','k1'),(b'\x90\x78\x56\x34','k2')):
    offs=[]; pos=0
    while True:
        i=d.find(pat,pos)
        if i<0: break
        offs.append(i); pos=i+1
    print(name,[hex(o) for o in offs][:6])

# raw scan of the whole image for member offsets
for disp,name in ((b'\xC0\x01\x00\x00','+0x1C0'),(b'\xC4\x01\x00\x00','+0x1C4')):
    hits=[]
    for m in re.finditer(re.escape(disp), d):
        off=m.start()
        b=d[max(0,off-3):off+8]
        hits.append((off,b.hex(' ')))
    print(name, len(hits))
    for off,b in hits[:12]:
        print('   0x%06X: %s' % (0x10000000+off, b))

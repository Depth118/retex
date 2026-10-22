import struct,binascii
d=open(r'work\texmod_mem.bin','rb').read()
print('size',hex(len(d)))
# zero ratio per 16K
for off in range(0,len(d),0x4000):
    b=d[off:off+0x4000]
    z=b.count(0)/len(b)
    print('0x%06X zero=%.3f  %s' % (off,z,binascii.hexlify(b[:16]).decode()))

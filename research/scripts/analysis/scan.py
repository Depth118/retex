import struct, re, sys, os

d = open(r'work\texmod_mem.bin','rb').read()
print('len', hex(len(d)))

# candidate embedded PE images
for m in re.finditer(b'MZ', d):
    o = m.start()
    if o + 0x40 > len(d):
        continue
    lf = struct.unpack_from('<I', d, o+0x3C)[0]
    if lf > 0x400 or o+lf+24 > len(d):
        continue
    if d[o+lf:o+lf+4] == b'PE\x00\x00':
        mach, nsec = struct.unpack_from('<HH', d, o+lf+4)
        optsz = struct.unpack_from('<H', d, o+lf+20)[0]
        magic = struct.unpack_from('<H', d, o+lf+24)[0] if o+lf+26 < len(d) else 0
        print('candidate PE at 0x%X lfanew=0x%X machine=0x%X nsec=%d optsz=%d magic=0x%X' % (o, lf, mach, nsec, optsz, magic))

print('--- DOS stub occurrences ---')
for m in re.finditer(b'This program cannot be run in DOS mode', d):
    print('0x%X' % m.start())

print('--- .dll names ---')
seen = set()
for m in re.finditer(rb'[A-Za-z0-9_\-\.]{3,50}\.dll', d):
    s = m.group().decode('latin1')
    if s.lower() not in seen:
        seen.add(s.lower())
        print('0x%X %s' % (m.start(), s))

print('--- interesting wide strings ---')
for m in re.finditer(rb'(?:[\x20-\x7E]\x00){4,}', d):
    s = m.group().decode('utf-16le', 'ignore')
    if any(k in s.lower() for k in ('texmod','d3d','texture','package','inject','dll','hook')):
        print('0x%X %s' % (m.start(), s))

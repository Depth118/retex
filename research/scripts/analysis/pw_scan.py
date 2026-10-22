import struct

BASE = 0x10000000
d = bytes(c ^ 0xAA for c in open(r'work\tm_a_unpacked.xor','rb').read())

cands = set()
for o in range(0, len(d)-8, 4):
    p, n = struct.unpack_from('<II', d, o)
    if 0 < n <= 64 and BASE <= p < BASE + len(d):
        off = p - BASE
        if off + n <= len(d):
            s = d[off:off+n]
            if all(32 <= c < 127 for c in s):
                try:
                    cands.add(s.decode('latin1'))
                except Exception:
                    pass
print('pointer+length candidates:', len(cands))
for c in sorted(cands)[:40]:
    print('   %r' % c)

# also: null-terminated strings with length stored just before/after
cands2 = set()
for o in range(0, len(d)-4, 4):
    v = struct.unpack_from('<I', d, o)[0]
    if BASE <= v < BASE + len(d):
        off = v - BASE
        end = d.find(b'\0', off, min(off+80, len(d)))
        if end > off and 1 <= end-off <= 40:
            s = d[off:end]
            if all(32 <= c < 127 for c in s):
                cands2.add(s.decode('latin1'))
print('null-terminated ptr candidates:', len(cands2))
for c in sorted(cands2)[:40]:
    print('   %r' % c)

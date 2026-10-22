import math, struct, zlib, os, re, binascii

p = r'work\texmod_mem.bin'
d = open(p, 'rb').read()
W = 4096

def ent(b):
    if not b:
        return 0
    c = [0]*256
    for x in b:
        c[x] += 1
    n = len(b)
    return -sum((v/n)*math.log2(v/n) for v in c if v)

rows = []
for off in range(0, len(d), W):
    b = d[off:off+W]
    rows.append((ent(b), off, len(b)))

print('== highest-entropy 4K windows ==')
for e, off, n in sorted(rows, reverse=True)[:25]:
    print('0x%06X  ent=%.3f  first=%s' % (off, e, binascii.hexlify(d[off:off+8]).decode()))

print()
print('== zero-heavy boundaries ==')
run_start = None
for e, off, n in rows:
    z = d[off:off+n].count(0)/max(n,1)
    if z < 0.05 and run_start is None:
        run_start = off
    elif z >= 0.05 and run_start is not None:
        print('non-zero run 0x%06X..0x%06X (%d bytes)' % (run_start, off, off-run_start))
        run_start = None
if run_start is not None:
    print('non-zero run 0x%06X..0x%06X' % (run_start, len(d)))

print()
print('== zlib probes ==')
found = 0
for off in range(0, len(d)-2, 1):
    if d[off] == 0x78 and d[off+1] in (0x01, 0x9C, 0xDA):
        for wbits in (15, -15):
            try:
                o = zlib.decompressobj(wbits)
                out = o.decompress(d[off:off+4*1024*1024])
                if len(out) > 4096:
                    print('zlib hit 0x%X wbits=%d outlen=%d head=%s' % (off, wbits, len(out), binascii.hexlify(out[:8]).decode()))
                    found += 1
                    break
            except Exception:
                pass
        if found > 10:
            break
print('zlib hits:', found)

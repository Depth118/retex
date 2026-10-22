import struct, sys, re, os, json

p = r"work\texmod_mem.bin"
d = open(p, 'rb').read()
out = os.path.dirname(p)

e_lfanew = struct.unpack_from('<I', d, 0x3C)[0]
print('e_lfanew=0x%X' % e_lfanew, 'sig=', d[e_lfanew:e_lfanew+4])
machine, nsec, ts, _, _, optsz, chars = struct.unpack_from('<HHIIIHH', d, e_lfanew+4)
opt = e_lfanew + 24
magic = struct.unpack_from('<H', d, opt)[0]
entry, base = struct.unpack_from('<II', d, opt+16)[0], struct.unpack_from('<I', d, opt+28)[0]
salign, falign = struct.unpack_from('<II', d, opt+32)
sizeimg, sizehdr = struct.unpack_from('<II', d, opt+56)
ndd = struct.unpack_from('<I', d, opt+92)[0]
print('machine=0x%X nsec=%d ts=%d magic=0x%X entry=0x%X base=0x%X sizeimg=0x%X sizehdr=0x%X ndd=%d' % (
    machine, nsec, ts, magic, entry, base, sizeimg, sizehdr, ndd))
print('timestamp', __import__('datetime').datetime.utcfromtimestamp(ts))

st = opt + optsz
for i in range(nsec):
    o = st + 40*i
    name = d[o:o+8].rstrip(b'\0')
    vs, va, rs, rp = struct.unpack_from('<IIII', d, o+8)
    ch = struct.unpack_from('<I', d, o+36)[0]
    print('sec%-2d %-9s VA=0x%08X VS=0x%08X RAW=0x%08X RS=0x%08X CH=0x%08X' % (i, name.decode('latin1'), va, vs, rp, rs, ch))

dd = opt + 96
for i in range(min(ndd, 16)):
    rva, sz = struct.unpack_from('<II', d, dd+8*i)
    if rva or sz:
        print('dir[%d] rva=0x%08X size=0x%08X' % (i, rva, sz))

def ascii_strings(b, minlen=5):
    return [(m.start(), m.group().decode('latin1')) for m in re.finditer(rb'[\x20-\x7E]{%d,}' % minlen, b)]

def utf16_strings(b, minlen=4):
    res = []
    # naive: find runs of printable ascii chars interleaved with 0x00
    pat = re.compile((rb'(?:[\x20-\x7E]\x00){%d,}' % minlen))
    for m in pat.finditer(b):
        try:
            res.append((m.start(), m.group().decode('utf-16le')))
        except Exception:
            pass
    return res

a = ascii_strings(d)
u = utf16_strings(d)
print('ascii strings:', len(a), 'utf16 strings:', len(u))

with open(os.path.join(out, 'strings_ascii.txt'), 'w', encoding='utf-8') as f:
    for off, s in a:
        f.write('0x%08X\t%s\n' % (off, s))
with open(os.path.join(out, 'strings_utf16.txt'), 'w', encoding='utf-8') as f:
    for off, s in u:
        f.write('0x%08X\t%s\n' % (off, s))

keys = ['texmod', 'tpf', 'd3d9', 'ddraw', 'createtexture', 'settexture', 'msvbvm', 'tmrls', 'tmldr',
        'vb6', 'vba', 'crc', 'checksum', 'package', 'texture', 'logging', 'inject', 'hook',
        'kernel32', 'user32', 'advapi', 'oleaut', 'gdi32', 'shlwapi', 'version.dll', 'd3dx']
for k in keys:
    hits = [s for o, s in (a + u) if k in s.lower()]
    print('== %-12s %d' % (k, len(hits)))
    for h in hits[:8]:
        print('    ', h[:120])

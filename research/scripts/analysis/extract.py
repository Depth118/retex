import struct, re, os

d = open(r'work\texmod_mem.bin','rb').read()
out = r'work'

def extract(va, size, key, name):
    off = va - 0x400000
    blob = bytes(c ^ key for c in d[off:off+size])
    path = os.path.join(out, name)
    open(path,'wb').write(blob)
    lf = struct.unpack_from('<I', blob, 0x3C)[0]
    machine, nsec, ts = struct.unpack_from('<HHI', blob, lf+4)
    optsz = struct.unpack_from('<H', blob, lf+20)[0]
    chars = struct.unpack_from('<H', blob, lf+22)[0]
    opt = lf+24
    magic = struct.unpack_from('<H', blob, opt)[0]
    entry, base = struct.unpack_from('<II', blob, opt+16)[0], struct.unpack_from('<II', blob, opt+28)[0]
    import datetime
    print('\n=== %s ===' % name)
    print('size=%d machine=0x%X nsec=%d optmagic=0x%X entry=0x%X base=0x%X chars=0x%X dll=%s' % (
        len(blob), machine, nsec, magic, entry, base, chars, bool(chars & 0x2000)))
    print('timestamp', datetime.datetime.utcfromtimestamp(ts))
    st = opt + optsz
    for i in range(nsec):
        o = st + 40*i
        nm = blob[o:o+8].rstrip(b'\0').decode('latin1')
        vs, rva, rs, rp = struct.unpack_from('<IIII', blob, o+8)
        print('  sec %-8s VA=0x%06X VS=0x%06X RAW=0x%06X RS=0x%06X' % (nm,rva,vs,rp,rs))
    # imports
    dd = opt + 96
    imp_rva, imp_sz = struct.unpack_from('<II', blob, dd+8)
    def r2o(r):
        for i in range(nsec):
            o = st + 40*i
            vs, rva, rs, rp = struct.unpack_from('<IIII', blob, o+8)
            if rva <= r < rva + max(vs, rs):
                return rp + (r - rva)
        return None
    if imp_rva:
        p = r2o(imp_rva)
        print('imports:')
        while True:
            oft, tds, fc, name_rva, ft = struct.unpack_from('<IIIII', blob, p)
            if name_rva == 0:
                break
            no = r2o(name_rva)
            dll = blob[no:blob.index(b'\0', no)].decode('latin1')
            thunks = []
            t = r2o(oft or ft)
            while True:
                v = struct.unpack_from('<I', blob, t)[0]
                if v == 0:
                    break
                if not (v & 0x80000000):
                    nmo = r2o(v)
                    nm = blob[nmo+2:blob.index(b'\0', nmo+2)].decode('latin1')
                    thunks.append(nm)
                t += 4
            print('  %-16s %d funcs' % (dll, len(thunks)))
            if dll.lower() in ('d3d9.dll','d3dx9_43.dll','d3dx9_36.dll','d3dx9_30.dll','d3dx9_25.dll','d3dx9_26.dll','d3dx9_27.dll'):
                print('     ', ', '.join(thunks[:40]))
            p += 20
    return blob

a = extract(0x430050, 0x3ACF7, 0x91, 'tm_payload_a.dll')
b = extract(0x46AD48, 0x3A240, 0xC8, 'tm_payload_b.dll')

for name, blob in (('A', a), ('B', b)):
    txt = re.findall(rb'[\x20-\x7E]{6,}', blob)
    keys = [s.decode('latin1') for s in txt if re.search(rb'\.tpf|texmod|Texture|d3d9|CreateTexture|SetTexture|CRC|crc|package|Logging|Replace|Hash|checksum', s, re.I)]
    print('\n--- %s interesting strings (%d) ---' % (name, len(keys)))
    seen=set()
    for s in keys:
        if s not in seen:
            seen.add(s); print('   ', s[:110])

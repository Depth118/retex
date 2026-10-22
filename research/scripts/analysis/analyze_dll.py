import struct, re, os

out = r'work'
for tag, fn in (('A','tm_a_unpacked.bin'), ('B','tm_b_unpacked.bin')):
    d = open(os.path.join(out, fn),'rb').read()
    print('\n' + '='*70)
    print('%s  %s  (%d bytes)' % (tag, fn, len(d)))
    txt = re.findall(rb'[\x20-\x7E]{5,}', d)
    strs = [s.decode('latin1') for s in txt]
    print('ascii strings:', len(strs))
    pats = ['d3d9','Direct3D','CreateTexture','SetTexture','Present','EndScene','tpf','TexMod','texmod',
            'CRC','crc','Checksum','checksum','Replace','Package','Logging','Dump','Texture']
    for p in pats:
        hits = [s for s in strs if p in s]
        if hits:
            print('  [%s] %d hits' % (p, len(hits)))
            seen=set()
            for h in hits[:6]:
                if h not in seen:
                    seen.add(h); print('     ', h[:100])
    # wide strings
    wide = re.findall(rb'(?:[\x20-\x7E]\x00){5,}', d)
    if wide:
        print('utf16 strings:', len(wide))
        for w in wide[:10]:
            print('   ', w.decode('utf-16le','ignore')[:100])
    # file paths / format strings
    fmts = [s for s in strs if '%' in s]
    print('  format strings sample:')
    for s in fmts[:25]:
        print('     ', s[:90])

import re, sys
out = r'work'
for tag, fn in (('A','tm_a_unpacked.xor'),('B','tm_b_unpacked.xor')):
    raw = open(fn,'rb').read()
    d = bytes(c ^ 0xAA for c in raw)
    s = [x.decode('latin1') for x in re.findall(rb'[\x20-\x7E]{5,}', d)]
    print('='*70)
    print(tag, 'strings', len(s))
    seen = set()
    for x in s:
        if x in seen: continue
        seen.add(x)
        if (' ' in x or '%' in x) and len(x) < 130 and not x.startswith('h:\\'):
            print('  ', x)

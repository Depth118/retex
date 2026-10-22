import zlib, struct

p = r'D:\Games\Need For Speed - Underground 1\Mods\TexMod\Texmod\redux1.0.8.tpf'
data = open(p,'rb').read(2*1024*1024)
print('head:', data[:32].hex())
print('as u32:', [hex(struct.unpack_from('<I',data,i)[0]) for i in range(0,32,4)])

def try_zlib(buf):
    try:
        o = zlib.decompressobj(15)
        out = o.decompress(buf)
        return out
    except Exception:
        return None

# plain zlib at offset 0?
out = try_zlib(data)
print('plain zlib@0:', None if out is None else (len(out), out[:16].hex()))

# scan XOR keys and offsets
found = 0
for key in range(256):
    for off in range(0, 4096):
        if (data[off]^key) == 0x78 and (data[off+1]^key) in (0x01,0x9c,0xda):
            dec = bytes(b^key for b in data[off:off+65536])
            out = try_zlib(dec)
            if out and len(out) > 100:
                print('HIT key=0x%02X off=0x%X outlen=%d head=%s' % (key,off,len(out),out[:24].hex()))
                found += 1
                if found > 5: raise SystemExit
print('done, hits', found)

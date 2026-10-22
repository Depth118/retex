import zipfile, os, struct, collections

src = r'D:\Games\Need For Speed - Underground 1\Mods\TexMod\Texmod\redux1.0.8.tpf'
dst = r'work\redux_decrypted.zip'

if not os.path.exists(dst):
    with open(src,'rb') as fi, open(dst,'wb') as fo:
        i = 0
        while True:
            chunk = fi.read(1 << 22)
            if not chunk:
                break
            out = bytearray(len(chunk))
            for j, b in enumerate(chunk):
                out[j] = b ^ (0xA4 if ((i + j) & 1) == 0 else 0x3F)
            fo.write(out)
            i += len(chunk)
    print('decrypted ->', dst, os.path.getsize(dst))

z = zipfile.ZipFile(dst)
infos = z.infolist()
print('entries:', len(infos))
print('total uncompressed: %.1f MB' % (sum(i.file_size for i in infos)/1e6))
meth = collections.Counter(i.compress_type for i in infos)
print('methods:', dict(meth))
exts = collections.Counter(os.path.splitext(i.filename)[1].lower() for i in infos)
print('extensions:', dict(exts))
print('\nfirst 15 entries:')
for i in infos[:15]:
    print('  %-60s csize=%9d usize=%9d' % (i.filename, i.compress_size, i.file_size))
print('\nlast 5 entries:')
for i in infos[-5:]:
    print('  %-60s csize=%9d usize=%9d' % (i.filename, i.compress_size, i.file_size))
# unique target prefixes
pref = collections.Counter(i.filename.rsplit('_0x',1)[0] for i in infos if '_0x' in i.filename)
print('\ntarget prefixes:', dict(list(pref.items())[:10]))
# crc test on first entry
name = infos[0].filename
data = z.read(name)
print('\nfirst entry read ok:', name, len(data), data[:8].hex())

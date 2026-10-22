import struct
from capstone import *
from capstone.x86 import *

d = bytes(c ^ 0xAA for c in open(r'work\tm_a_unpacked.xor','rb').read())
md = Cs(CS_ARCH_X86, CS_MODE_32); md.detail = True

# Scan every offset for a plausible instruction that references +0x1C0/+0x1C4
hits = []
for off in range(len(d) - 8):
    # look for disp32
    if d[off:off+4] in (b'\xC0\x01\x00\x00', b'\xC4\x01\x00\x00'):
        disp = struct.unpack_from('<I', d, off)[0]
        start = max(0, off - 8)
        for ins in md.disasm(d[start:off+4], 0x10000000 + start):
            if ins.address + ins.size == 0x10000000 + off + 4:
                # instruction ends at the disp32 -> it references it
                for op in ins.operands:
                    if op.type == X86_OP_MEM and op.mem.disp == disp:
                        hits.append((ins.address, ins.mnemonic, ins.op_str, op.access))
                break
seen = set()
for a, m, o, acc in hits:
    key = (a, m, o)
    if key in seen: continue
    seen.add(key)
    accs = 'w' if acc & 2 else ('r' if acc & 1 else '')
    print('0x%08X  %-8s %-45s %s' % (a, m, o, accs))

import struct, re
from capstone import *
from capstone.x86 import *

p = r'work\texmod_mem.bin'
d = open(p,'rb').read()
IMG = 0x400000
# file offset -> VA for the unpacked image (identity: offset == RVA)
def va(off): return IMG + off

md = Cs(CS_ARCH_X86, CS_MODE_32)
md.detail = True

# linear sweep over code region
code_start, code_end = 0x1000, 0x2A000
hits = []
insns = list(md.disasm(d[code_start:code_end], va(code_start)))
print('disassembled', len(insns), 'instructions')

# collect immediate/data references into the blob region 0x430000..0x4A4000
target_lo, target_hi = 0x430000, 0x4A4000
for ins in insns:
    for op in ins.operands:
        val = None
        if op.type == X86_OP_IMM:
            val = op.imm & 0xFFFFFFFF
        elif op.type == X86_OP_MEM and op.mem.base == X86_REG_INVALID and op.mem.index == X86_REG_INVALID:
            val = op.mem.disp & 0xFFFFFFFF
        if val is not None and target_lo <= val < target_hi:
            hits.append((ins.address, ins.mnemonic, ins.op_str, val))

print('blob references:', len(hits))
for a,m,o,v in hits[:60]:
    print('0x%06X  %-8s %-40s ; -> 0x%06X' % (a,m,o,v))

# also references to interesting strings
def find_string(s):
    b = s.encode('latin1')
    i = d.find(b)
    return (IMG + i, i) if i >= 0 else (None,None)
for s in [b'SeDebugPrivilege', b'CreateRemoteThread', b'Speed', b'.tpf', b'Software\\TexMod']:
    print(s, '->', find_string(s.decode('latin1')))


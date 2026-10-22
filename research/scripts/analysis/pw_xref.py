from capstone import *
from capstone.x86 import *

d = open(r'work\texmod_mem.bin','rb').read()
md = Cs(CS_ARCH_X86, CS_MODE_32); md.detail = True

seen = []
for ins in md.disasm(d[0x1000:0x2A000], 0x401000):
    for op in ins.operands:
        if op.type == X86_OP_MEM and op.mem.disp in (0x1C0, 0x1C4):
            seen.append((ins.address, ins.mnemonic, ins.op_str, op.access))
print('refs to +0x1C0/+0x1C4:', len(seen))
for a, m, o, acc in seen:
    print('0x%06X  %-8s %-40s access=%s' % (a, m, o, acc))

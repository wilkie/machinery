"""MUL/IMUL/DIV timing against operand values (register forms).

    python3 -I mul.py F7.4 16
"""
import collections
import os
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from traces import vectors, measured, prefixes  # noqa: E402

R16 = ['eax', 'ecx', 'edx', 'ebx', 'esp', 'ebp', 'esi', 'edi']


def reg(v, n, w):
    if w == 8:
        r = v['regs'][R16[n & 3]]
        return (r >> 8) & 0xFF if n >= 4 else r & 0xFF
    return v['regs'][R16[n]] & ((1 << w) - 1)


def bits(x, w, signed):
    if signed and x >> (w - 1):
        x = (~x) & ((1 << w) - 1)
    return x.bit_length()


name, w = sys.argv[1], int(sys.argv[2])
signed = len(sys.argv) > 3
g_rm = collections.defaultdict(collections.Counter)
g_acc = collections.defaultdict(collections.Counter)
for v in vectors(name):
    if 'exception' in v:
        continue
    b = v['bytes']
    p = prefixes(v)
    m = b[p + 1]
    if m >> 6 != 3:
        continue
    src = reg(v, m & 7, w)
    acc = reg(v, 0, w)
    t = measured(v, 1)
    g_rm[bits(src, w, signed)][t] += 1
    g_acc[bits(acc, w, signed)][t] += 1
for label, g in (('by r/m bit length', g_rm), ('by accumulator bit length', g_acc)):
    print(label)
    for k in sorted(g):
        c = g[k]
        print('  %2d  mode %d  %s' % (k, c.most_common(1)[0][0], ' '.join('%dx%d' % kv for kv in sorted(c.items()))))

"""Measured cycles (less the rig's clock) grouped by an expression.

    python3 -I by.py "r['ecx'] & 0x1F" D3.4 6D3.4
    names in scope: v (the vector), r (initial registers), b (bytes),
    p (prefix count), m (ModRM or None), data (data bus cycles), rep
"""
import collections
import os
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from traces import vectors, measured, prefixes, bus_cycles  # noqa: E402

expr = sys.argv[1]
for name in sys.argv[2:]:
    g = collections.defaultdict(collections.Counter)
    for v in vectors(name):
        if 'exception' in v:
            continue
        b = v['bytes']
        p = prefixes(v)
        r = v['regs']
        rep = any(x in (0xF2, 0xF3) for x in b[:p])
        two = b[p] == 0x0F
        m = b[p + 1 + two] if p + 1 + two < len(b) else None
        data = sum(1 for x in bus_cycles(v) if x[1] in ('MEMR', 'MEMW', 'IOR', 'IOW'))
        g[eval(expr)][measured(v, 1)] += 1
    print('==', name)
    for k in sorted(g, key=str):
        c = g[k]
        n = sum(c.values())
        print('  %-20s n=%-5d mode %-3d %s' % (k, n, c.most_common(1)[0][0], ' '.join('%dx%d' % kv for kv in sorted(c.items()))))

"""First-element and per-element cycles of repeated string instructions,
from vectors whose pointers (and port) are aligned so no access splits.

    python3 -I strings.py A4 66A5 ...
"""
import collections
import os
import statistics
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from traces import vectors, measured, prefixes  # noqa: E402

for name in sys.argv[1:]:
    w = 4 if '66' in name[:4] and name.endswith(('5', '7', 'B', 'D', 'F')) else (2 if name[-1] in '57BDF' else 1)
    first = collections.Counter()
    per = []
    single = collections.Counter()
    for v in vectors(name):
        if 'exception' in v:
            continue
        b = v['bytes']
        p = prefixes(v)
        r = v['regs']
        rep = any(x in (0xF2, 0xF3) for x in b[:p])
        if (r['esi'] % w) or (r['edi'] % w) or (r['edx'] % w):
            continue
        t = measured(v, 1)
        if not rep:
            single[t] += 1
            continue
        n = (r['ecx'] - v['final'].get('ecx', r['ecx'])) & 0xFFFF
        if n == 1:
            first[t] += 1
        elif n >= 8:
            per.append((t, n))
    f = first.most_common(1)[0][0] if first else None
    rs = [(t - f) / (n - 1) for t, n in per] if f is not None else []
    print('%-8s single %s  first %s  per-element %s  (n=%d)' % (
        name, single.most_common(1)[0][0] if single else '-', f,
        '%.2f' % statistics.median(rs) if rs else '-', len(rs)))

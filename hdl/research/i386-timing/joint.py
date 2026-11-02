"""Measured cycles by a joint key of features.

    python3 -I joint.py ea,data 8B 89
"""
import collections
import os
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from traces import vectors, measured  # noqa: E402
from features import feats  # noqa: E402

keys = sys.argv[1].split(',')
for name in sys.argv[2:]:
    g = collections.defaultdict(collections.Counter)
    for v in vectors(name):
        f = feats(v)
        if f['exc']:
            continue
        g[tuple(f.get(k) for k in keys)][measured(v)] += 1
    print('==', name)
    for k in sorted(g, key=str):
        c = g[k]
        n = sum(c.values())
        mode = c.most_common(1)[0]
        print('  %-30s n=%-5d mode %d (%2.0f%%)  %s' % (k, n, mode[0], 100 * mode[1] / n,
              ' '.join('%dx%d' % kv for kv in sorted(c.items()))))

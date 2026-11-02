"""Which features of a vector explain its measured cycles?

    python3 -I features.py 8B 89 6601
"""
import collections
import os
import sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from traces import vectors, bus_cycles, measured, prefixes


def feats(v):
    b = v['bytes']
    p = prefixes(v)
    a32 = 0x67 in b[:p]
    op = b[p]
    two = op == 0x0F
    m = b[p + 1 + two] if p + 1 + two < len(b) else None
    f = {}
    f['prefixes'] = p
    f['segpfx'] = sum(1 for x in b[:p] if x in (0x26, 0x2E, 0x36, 0x3E, 0x64, 0x65))
    f['len'] = len(b) - 1
    f['odd'] = (len(b) - 1) & 1
    f['eip&1'] = v['regs']['eip'] & 1
    f['eip&3'] = v['regs']['eip'] & 3
    bus = bus_cycles(v)
    f['data'] = sum(1 for x in bus if x[1] in ('MEMR', 'MEMW', 'IOR', 'IOW'))
    f['code'] = sum(1 for x in bus if x[1] == 'CODE')
    if m is not None:
        mod, rm = m >> 6, m & 7
        if mod == 3:
            f['ea'] = 'reg'
        elif not a32:
            f['ea'] = {0: 'bx+si', 1: 'bx+di', 2: 'bp+si', 3: 'bp+di', 4: 'si', 5: 'di', 6: 'bp', 7: 'bx'}[rm]
            if rm < 4:
                f['ea'] = 'base+index'
            elif rm == 6 and mod == 0:
                f['ea'] = 'disp'
            else:
                f['ea'] = 'base'
            f['ea'] += '/mod%d' % mod
        else:
            if rm == 4:
                sib = b[p + 2 + two]
                f['ea'] = 'sib' + ('+index' if (sib >> 3) & 7 != 4 else '') + '/mod%d' % mod
            elif rm == 5 and mod == 0:
                f['ea'] = 'disp32'
            else:
                f['ea'] = 'base32/mod%d' % mod
    f['exc'] = 'exception' in v
    da = [x[2] for x in bus if x[1] in ('MEMR', 'MEMW')]
    if da:
        f['addr&3'] = da[0] & 3
    return f


def main(names):
    for name in names:
        vs = vectors(name)
        print('==', name, vs[0]['name'].split()[0], len(vs))
        rows = [(feats(v), measured(v)) for v in vs]
        for key in ['prefixes', 'segpfx', 'len', 'eip&1', 'eip&3', 'data', 'code', 'ea', 'exc']:
            g = collections.defaultdict(collections.Counter)
            for f, m in rows:
                if key in f:
                    g[f[key]][m] += 1
            if len(g) < 2:
                continue
            print('  by', key)
            for k in sorted(g, key=str):
                c = g[k]
                n = sum(c.values())
                mode = c.most_common(1)[0]
                print('    %-16s n=%-5d mode %d (%2.0f%%)  %s' % (k, n, mode[0], 100 * mode[1] / n,
                      ' '.join('%dx%d' % kv for kv in sorted(c.items()))))


if __name__ == "__main__":
    main(sys.argv[1:])

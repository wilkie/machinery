"""Turn `machine vectors --fit` output into timing-table entries.

    python3 -I table.py fit.log > entries.machine

For each instruction: the register forms' mode, and for memory forms the
mode of (cycles - the operand's `ea`) pooled over addressing forms, giving
`if X.is_mem { M + X.ea } else { R }`. The value chosen is the one
with the most vectors within a cycle of it (the traces carry a +-1 clock
that depends on how the next instruction falls in the decoder); entries
that explain less than 80% of their vectors within a cycle are marked for a
hand-written formula.
"""
import collections
import re
import sys

hist = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
faults = {}
name = None
for line in open(sys.argv[1]):
    m = re.match(r'// (\S+)$', line.strip())
    if m and not line.startswith('//   '):
        name = m.group(1)
        continue
    m = re.match(r'//   \[(.*)\] \S+ \(.*?\)(?:  <- data-dependent\?)?\s+(.*)$', line.rstrip())
    if m and name:
        feats = dict(kv.split('=') for kv in m.group(1).split()) if m.group(1) else {}
        for vc in m.group(2).split():
            v, c = vc.split('x')
            hist[name][tuple(sorted(feats.items()))][int(v)] += int(c)
        continue
    m = re.match(r'// fault Some\((\d+)\): exceeds the table by (-?\d+) most often \((\d+)% of (\d+)\)', line)
    if m:
        faults[int(m.group(1))] = (int(m.group(2)), int(m.group(3)), int(m.group(4)))


def mode(c):
    """The value with the most vectors within one cycle of it, and that share."""
    n = sum(c.values())
    best = max(c, key=lambda v: (c[v - 1] + c[v] + c[v + 1], c[v]))
    return best, 100.0 * (c[best - 1] + c[best] + c[best + 1]) / n, n


out = []
for name in sorted(hist):
    groups = hist[name]
    reg = collections.Counter()
    mem = collections.Counter()
    memop = None
    for feats, c in groups.items():
        f = dict(feats)
        ops = [k[:-len('.is_mem')] for k, v in f.items() if k.endswith('.is_mem') and v == '1']
        if ops:
            memop = ops[0]
            e = int(f.get(memop + '.ea', 0))
            for v, n in c.items():
                mem[v - e] += n
        else:
            reg.update(c)
    parts = []
    notes = []
    if mem and reg:
        mv, ms, mn = mode(mem)
        rv, rs, rn = mode(reg)
        text = 'if %s.is_mem { %d + %s.ea } else { %d }' % (memop, max(mv, 0), memop, max(rv, 0))
        notes = ['mem %.0f%% of %d within 1' % (ms, mn), 'reg %.0f%% of %d within 1' % (rs, rn)]
        low = min(ms, rs)
    elif mem:
        mv, ms, mn = mode(mem)
        text = '%d + %s.ea' % (max(mv, 0), memop)
        notes = ['%.0f%% of %d within 1' % (ms, mn)]
        low = ms
    else:
        rv, rs, rn = mode(reg)
        text = '%d' % max(rv, 0)
        notes = ['%.0f%% of %d within 1' % (rs, rn)]
        low = rs
    flag = '  // TODO data-dependent' if low < 80 else ''
    out.append('    %s => %s;%s  // %s' % (name, text, flag, ', '.join(notes)))
print('\n'.join(out))
if faults:
    arms = ', '.join('%d => %d' % (v, max(e, 0)) for v, (e, s, n) in sorted(faults.items()))
    print('    fault => match vector { %s, _ => 40 };' % arms)
    for v, (e, s, n) in sorted(faults.items()):
        print('    // fault %d: %d (%d%% of %d)' % (v, e, s, n))

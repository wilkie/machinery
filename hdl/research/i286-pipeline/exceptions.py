"""Fit exception delivery: INT n, INT3 and INTO, and faults (#DE, #BR, #UD).

    python3 -I exceptions.py u CD CC        # the decoder stops (a `jump`)
    python3 -I exceptions.py b F6.6 F7.6    # it had carried on (a `branch`)

Every delivery is three pushes, two reads of the vector table and the
transfer; what differs is the work before it, which for a fault is the
faulting instruction's own schedule and a few clocks more.
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import fit  # noqa: E402
from traces import vectors, accesses, jump_target, start_address, bus_cycles  # noqa: E402
from pipeline import simulate  # noqa: E402
from fit import form, sign_extended  # noqa: E402

UNCOND = True
def run(t, P, partial=False):
    L = len(t['bytes']) - 1
    hold = {i: 'stall' if (start_address(t) + i) % 2 == 0 else 'late' for i in sign_extended(t)}
    seq = list(accesses(t)); tgt = jump_target(t)
    seq.insert(P['jpos'], ('J',))
    sim = simulate(L, start_address(t), seq, P['pre'], P['posts'][:len(seq)], tgt, {}, uncond=UNCOND, hold=hold)
    tr = bus_cycles(t)
    if not partial: return sim == tr
    m = 0
    for a, b in zip(sim, tr):
        if a != b: break
        m += 1
    return m + (100 if sim == tr else 0)
fit.run = run

for f in sys.argv[2:]:
    UNCOND = sys.argv[1] == 'u'
    groups = collections.defaultdict(list)
    for t in vectors(f, 600, keep_exceptions=True):
        if 'exception' not in t or jump_target(t) is None: continue
        _, _, ea, mod = form(t)
        groups[(t['exception'] if f not in ('CD',) else 'n', tuple(k for k, _ in accesses(t)), ea, mod)].append(t)
    for key, g in sorted(groups.items(), key=lambda kv: -len(kv[1]))[:4]:
        if len(g) < 6: continue
        ok, P = fit.fit_group(g, top=40)
        print(f, sys.argv[1], key[0], key[1], key[2], key[3], f'{ok}/{len(g)}', P['pre'], P['jpos'], P['posts'], flush=True)

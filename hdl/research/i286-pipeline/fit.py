"""Fit each instruction form's execution-unit schedule to the traces.

    python3 -I fit.py 89 01 E8 ...      # opcode files of the corpus

Vectors are grouped by form: their data accesses and whether they jump,
the addressing mode, and the prefix count. Each group gets the schedule
(execution cycles before the first step and after each, the jump's place
among the accesses, and the decoder's extra cycle after a ModRM with a
displacement) that reproduces the most traces exactly, cycle for cycle.
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from traces import vectors, start_address, bus_cycles, accesses, jump_target, PREFIXES  # noqa: E402
from pipeline import simulate  # noqa: E402

MODRM = (set(range(0x00, 0x40)) - {x for x in range(0x00, 0x40) if x & 7 in (4, 5, 6, 7)}) | {
    0x62, 0x69, 0x6B, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B,
    0x8C, 0x8D, 0x8E, 0x8F, 0xC0, 0xC1, 0xC4, 0xC5, 0xC6, 0xC7, 0xD0, 0xD1, 0xD2, 0xD3, 0xF6,
    0xF7, 0xFE, 0xFF}
CONDITIONAL = set(range(0x70, 0x80)) | {0xE0, 0xE1, 0xE2, 0xE3, 0xCE}


def form(t):
    """(prefix count, ModRM index or None, addressing mode, mod)."""
    b = t['bytes']
    p = 0
    while b[p] in PREFIXES:
        p += 1
    if b[p] not in MODRM or len(b) <= p + 1:
        return p, None, None, None
    m = b[p + 1]
    mod, rm = m >> 6, m & 7
    if mod == 3:
        return p, p + 1, 'reg', None
    ea = 'bi' if rm < 4 else ('bp' if rm == 6 and mod else ('d16' if rm == 6 else 'b'))
    return p, p + 1, ea, mod


def run(t, P, partial=False):
    L = len(t['bytes']) - 1
    p, mpos, ea, mod = form(t)
    extra = {}
    # An 8-bit displacement that starts a word stays a clock longer in the
    # queue (the address work itself is in the schedule).
    hold = {}
    if mpos is not None and mod == 1:
        hold = {mpos + 1: 'stall' if (start_address(t) + mpos + 1) % 2 == 0 else 'late'}
    seq = list(accesses(t))
    tgt = jump_target(t)
    if tgt is not None:
        seq.insert(P['jpos'], ('J',))
    op = t['bytes'][p]
    sim = simulate(L, start_address(t), seq, P['pre'], P['posts'][:len(seq)], tgt, extra,
                   uncond=tgt is not None and op not in CONDITIONAL, hold=hold)
    tr = bus_cycles(t)
    if not partial:
        return sim == tr
    m = 0
    for a, b in zip(sim, tr):
        if a != b:
            break
        m += 1
    return m + (100 if sim == tr else 0)


def fit_group(vs, rounds=3, top=26):
    nacc = len(accesses(vs[0]))
    jumps = jump_target(vs[0]) is not None
    n = nacc + (1 if jumps else 0)
    score = lambda P: sum(run(t, P, partial=True) for t in vs)  # noqa: E731
    best = P = None
    for pre0 in (2, 3, 4, 5, 20):
        Q = {'pre': pre0, 'jpos': nacc, 'posts': [0] * n, 'x': 0}
        sc = score(Q)
        if best is None or sc > best:
            best, P = sc, Q
    keys = [('pre', None)] + ([('jpos', None)] if jumps else []) + [('posts', i) for i in range(n)]
    for _ in range(rounds):
        improved = False
        for k, i in keys:
            rng = {'pre': range(top), 'x': range(3), 'jpos': range(nacc + 1)}.get(k, range(top))
            for v in rng:
                Q = {'pre': P['pre'], 'jpos': P['jpos'], 'posts': list(P['posts']), 'x': P['x']}
                if i is None:
                    Q[k] = v
                else:
                    Q['posts'][i] = v
                sc = score(Q)
                if sc > best:
                    best, P, improved = sc, Q, True
        if not improved:
            break
    return sum(run(t, P) for t in vs), P


def main(files, per_file=400, min_group=12):
    total = good = 0
    for f in files:
        groups = collections.defaultdict(list)
        for t in vectors(f, per_file):
            p, _, ea, mod = form(t)
            key = (tuple(k for k, _ in accesses(t)), jump_target(t) is not None, ea, mod, p)
            groups[key].append(t)
        for key, g in sorted(groups.items(), key=lambda kv: -len(kv[1])):
            if len(g) < min_group:
                continue
            ok, P = fit_group(g)
            total += len(g)
            good += ok
            print(f"{f:6} {str(key)[:58]:<58} {ok:4}/{len(g):<4} {P}", flush=True)
    if total:
        print(f"overall {good}/{total} = {100 * good / total:.1f}%", flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])

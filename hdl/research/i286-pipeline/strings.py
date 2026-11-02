"""Fit repeated string instructions: the first iteration's lead-in, the work
after each access of an iteration, and the last iteration's tail.

    python3 -I strings.py A4 AA AC      # opcode files of the corpus

A vector runs the whole repetition, so its trace is the lead-in followed by
CX iterations; this searches the three for the most exact traces, and the
lead-in alone for CX = 0. Word strings depend on alignment (a split store
lengthens the tail), which is best seen by grouping on SI and DI parity.
"""
import itertools
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from traces import vectors, start_address, bus_cycles, PREFIXES  # noqa: E402
from pipeline import simulate  # noqa: E402

def rep_of(t):
    b = t['bytes']; p = 0; r = None
    while b[p] in PREFIXES:
        if b[p] in (0xF2, 0xF3): r = b[p]
        p += 1
    return r

WORD = {0xA5, 0xA7, 0xAB, 0xAD, 0xAF, 0x6D, 0x6F}

def str_accesses(t):
    """A byte string's cycles are each an access; a word string's split
    (odd address) word is two cycles of one access."""
    b = t['bytes']; p = 0
    while b[p] in PREFIXES: p += 1
    cyc = [c for c in t['bus'] if c[1] not in ('CODE', 'HALT')]
    if b[p] not in WORD:
        return [(k, 1) for _, k, _, _ in cyc]
    out = []; i = 0
    while i < len(cyc):
        _, k, a, _ = cyc[i]
        n = 2 if a & 1 and i + 1 < len(cyc) and cyc[i + 1][1] == k else 1
        out.append((k, n)); i += n
    return out

accesses = str_accesses

def run(t, pre, mid, last, k):
    acc = accesses(t)
    n = len(acc) // k if k else 0
    posts = []
    for i in range(n):
        posts += list(mid[:-1]) + [mid[-1] if i < n - 1 else last]
    if not acc:
        return simulate(len(t['bytes']) - 1, start_address(t), [], pre, [], None) == bus_cycles(t)
    return simulate(len(t['bytes']) - 1, start_address(t), acc, pre, posts, None) == bus_cycles(t)

for f in sys.argv[1:]:
    vs = [t for t in vectors(f, 1000) if rep_of(t)]
    zero = [t for t in vs if not accesses(t)]
    some = [t for t in vs if accesses(t)]
    # CX == 0: nothing but the lead-in.
    best = max((sum(run(t, pre, (), 0, 0) for t in zero), pre) for pre in range(0, 16)) if zero else None
    print(f, 'cx0', best, len(zero))
    k = {'A4': 2, 'A5': 2, 'A6': 2, 'A7': 2, 'AA': 1, 'AB': 1, 'AC': 1, 'AD': 1, 'AE': 1, 'AF': 1, '6C': 2, '6D': 2, '6E': 2, '6F': 2}[f]
    sample = some[:60]
    res = []
    for pre in range(0, 12):
        for mid in itertools.product(range(0, 8), repeat=k):
            for last in range(0, 10):
                ok = sum(run(t, pre, mid, last, k) for t in sample)
                res.append((ok, pre, mid, last))
    res.sort(reverse=True)
    ok, pre, mid, last = res[0]
    full = sum(run(t, pre, mid, last, k) for t in some)
    print(f, 'iter', res[:3], 'all', full, len(some))

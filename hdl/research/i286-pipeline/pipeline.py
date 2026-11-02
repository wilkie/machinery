"""A cycle-level model of the 80286's bus unit, decoder and execution unit,
fitted against the SingleStepTests cycle traces (see README.md).

Time is in processor clocks from the jump that starts a vector. One vector
is an instruction followed by an injected HLT.
"""

def simulate(L, start, seq, pre, posts, target=None, extra=None, Q=6, hc=3, lat=2, jl=1, limit=600, uncond=False):
    """The bus cycles [(start, kind)] of one vector.

    L: instruction length (prefixes included); start: its linear address.
    seq: the execution unit's steps in order, each a data access
      ('MEMR'|'MEMW'|'IOR'|'IOW', bus cycles) or a jump ('J',).
    pre: execution cycles before the first step; posts: after each step.
    target: the jump's target address. extra: {byte index: extra decoder
      cycles after that byte} (a ModRM followed by a displacement).
    uncond: the decoder stops after this instruction (an unconditional
      transfer).
    Q: queue bytes; hc: HLT's cycles before its HALT request; lat: cycles
    from an execution-unit request to its bus cycle; jl: from a jump to the
    target's fetch decision.
    """
    extra = extra or {}
    cycles = []
    bus_free = 0
    fp = start
    seg_len = L + 1                # bytes the decoder will take from this stream
    arrivals = []; inflight = []   # inflight: [arrive, nbytes, live]
    proc = []; last_proc = -1
    ins_ready = hlt_ready = None
    jumped = False
    eu_state = 'idle'; eu_t = None; si = 0
    pending = []; data_ready = None
    hlt_start = halt_req = None
    fetch_block_until = -1
    has_jump = any(st[0] == 'J' for st in seq)
    t = -1
    def real_count(pr):
        return sum(1 for _, d in pr if not d)
    def start_step():
        nonlocal eu_state, eu_t, si
        nonlocal fp, arrivals, proc, last_proc, hlt_ready, seg_len, jumped, fetch_block_until
        st = seq[si]
        if st[0] == 'J':
            # Flush: in-flight fetches are discarded, decoding restarts at the target.
            for f in inflight: f[2] = False
            arrivals = []; proc = []; last_proc = t; hlt_ready = None
            fp = target; seg_len = 1; jumped = True
            fetch_block_until = t + jl
            eu_state = 'post'; eu_t = t + posts[si]
        else:
            k, n = st
            w = k in ('MEMR', 'IOR')
            pending.append([t, k, n, w])
            eu_state = 'waitdata' if w else 'post'
            eu_t = None if w else t + posts[si]
    while t < limit:
        for f in list(inflight):
            if f[0] <= t:
                if f[2]:
                    # An odd fetch's word slot holds one dead byte.
                    if f[1] == 1: arrivals.append((f[0], True))
                    arrivals.extend([(f[0], False)] * f[1])
                inflight.remove(f)
        # Decoder.
        limit_here = L if (uncond and not jumped) else seg_len
        while len(proc) < len(arrivals) and real_count(proc) < limit_here:
            k = len(proc)
            at, dead = arrivals[k]
            if dead:
                # Freed with the byte beside it.
                if k + 1 >= len(arrivals): break
                p = max(last_proc + 1, arrivals[k + 1][0] + 1)
                if p > t: break
                proc.append((p, True)); continue
            p = max(last_proc + 1, at + 1)
            if not jumped and k - 1 in extra:
                p += extra[k - 1]
            if p > t: break
            proc.append((p, False)); last_proc = p
            r = real_count(proc)
            if not jumped and r == L: ins_ready = p
            if r == seg_len: hlt_ready = p
        # A dead byte holds its place until the next flush.
        consumed = sum(1 for p, d in proc if p < t and not d)
        # Execution unit.
        if eu_state == 'idle' and ins_ready is not None and t >= ins_ready:
            eu_state = 'pre'; eu_t = t + pre
        for _ in range(8):
            before = (eu_state, si)
            if eu_state == 'pre' and t >= eu_t:
                if si < len(seq): start_step()
                else: eu_state = 'done'
            if eu_state == 'waitdata' and data_ready is not None and t >= data_ready:
                data_ready = None; eu_state = 'post'; eu_t = t + posts[si]
            if eu_state == 'post' and t >= eu_t:
                si += 1
                if si < len(seq): start_step()
                else: eu_state = 'done'
            if (eu_state, si) == before: break
        if eu_state == 'done' and hlt_start is None and hlt_ready is not None and t >= hlt_ready:
            hlt_start = t; halt_req = t + hc
        s = t + 1
        if s >= bus_free:
            if pending and s >= pending[0][0] + lat:
                p = pending[0]
                cycles.append((s, p[1])); p[2] -= 1; bus_free = s + 2
                if p[2] == 0:
                    pending.pop(0)
                    if p[3]: data_ready = s
            elif not pending and halt_req is not None and s >= halt_req + lat:
                cycles.append((s, 'HALT')); return cycles
            elif s >= fetch_block_until and not (hlt_start is not None and t > hlt_start):
                q = len(arrivals) + sum(2 for f in inflight if f[2]) - consumed
                n = 1 if fp & 1 else 2
                if q + n <= Q:
                    cycles.append((s, 'CODE')); inflight.append([s + 2, n, True]); fp += n; bus_free = s + 2
        t += 1
    return cycles

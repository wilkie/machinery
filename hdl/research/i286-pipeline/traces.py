"""Reading SingleStepTests 80286 MOO files: each vector's bytes, initial
registers, exception, and bus cycles as (start cycle, kind, address, BHE)."""
import gzip
import os
import struct

CORPUS = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'simulators', 'test',
                      'i286', 'SingleStepTests', 'v1_real_mode')
# One CYCL record: pins, address, segment, memory and I/O status, BHE,
# data, bus status, T-state, queue operation, queue byte.
CYCLE = struct.Struct('<B I B B B B H B B B B')
# The 286 bus status (COD/INTA, M/IO, S1, S0), as the corpus tool decodes it.
STATUS = {0x0: 'INTA', 0x4: 'HALT', 0x5: 'MEMR', 0x6: 'MEMW', 0x9: 'IOR', 0xA: 'IOW', 0xD: 'CODE'}
REGS = ["ax", "bx", "cx", "dx", "cs", "ss", "ds", "es", "sp", "bp", "si", "di", "ip", "flags"]
PREFIXES = (0x26, 0x2E, 0x36, 0x3E, 0xF0, 0xF2, 0xF3)


def chunks(b, off, end):
    while off < end:
        tag = b[off:off + 4].decode('latin1')
        n = struct.unpack_from('<I', b, off + 4)[0]
        yield tag, off + 8, n
        off += 8 + n


def vectors(name, limit=5000, keep_exceptions=False):
    """Vectors of one opcode file (`'89'`, `'F7.4'`)."""
    b = gzip.open(os.path.join(CORPUS, name + '.MOO.gz')).read()
    out = []
    for tag, off, ln in chunks(b, 0, len(b)):
        if tag != 'TEST':
            continue
        t = {'file': name}
        for st, so, sn in chunks(b, off + 4, off + ln):
            if st == 'BYTS':
                n = struct.unpack_from('<I', b, so)[0]
                t['bytes'] = b[so + 4:so + 4 + n]
            elif st == 'NAME':
                n = struct.unpack_from('<I', b, so)[0]
                t['name'] = b[so + 4:so + 4 + n].decode()
            elif st == 'EXCP':
                t['exception'] = b[so]
            elif st == 'INIT':
                for t2, o2, _ in chunks(b, so, so + sn):
                    if t2 == 'REGS':
                        mask = struct.unpack_from('<H', b, o2)[0]
                        p = o2 + 2
                        regs = {}
                        for i, r in enumerate(REGS):
                            if mask & (1 << i):
                                regs[r] = struct.unpack_from('<H', b, p)[0]
                                p += 2
                        t['regs'] = regs
            elif st == 'CYCL':
                cnt = struct.unpack_from('<I', b, so)[0]
                bus = []
                for k in range(cnt):
                    f = CYCLE.unpack_from(b, so + 4 + k * CYCLE.size)
                    if f[0] & 1:  # ALE: a bus cycle starts
                        bus.append((k, STATUS.get(f[7] & 0xF, '?'), f[1], (f[0] >> 1) & 1))
                t['bus'] = bus
                t['cycles'] = cnt
        if 'exception' in t and not keep_exceptions:
            continue
        out.append(t)
        if len(out) >= limit:
            break
    return out


def start_address(t):
    return ((t['regs']['cs'] << 4) + t['regs']['ip']) & 0xFFFFFF


def bus_cycles(t):
    """[(start, kind)]: what the model has to reproduce."""
    return [(s, k) for s, k, _, _ in t['bus']]


def accesses(t):
    """The data accesses the execution unit made, a split word as one:
    [(kind, bus cycles)]."""
    out = []
    cyc = [c for c in t['bus'] if c[1] not in ('CODE', 'HALT')]
    i = 0
    while i < len(cyc):
        _, k, a, _ = cyc[i]
        n = 2 if a & 1 and i + 1 < len(cyc) and cyc[i + 1][1] == k and cyc[i + 1][2] == a + 1 else 1
        out.append((k, n))
        i += n
    return out


def jump_target(t):
    """The first code fetch off the sequential path, or None."""
    fp = start_address(t)
    for _, k, a, _ in t['bus']:
        if k != 'CODE':
            continue
        if a != fp:
            return a
        fp += 1 if fp & 1 else 2
    return None

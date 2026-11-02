"""Read the SingleStepTests 80386 corpus (MOO files): each vector's bytes,
initial registers and its cycle trace, decoded with the 386's status lines.

    python3 -I traces.py 8B 3                 # print vector 3 of 8B.MOO.gz
"""
import gzip
import os
import struct
import sys

CORPUS = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'simulators', 'test', 'i386',
                      'SingleStepTests', 'v1_ex_real_mode')
REGS32 = ["cr0", "cr3", "eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp", "cs", "ds", "es",
          "fs", "gs", "ss", "eip", "eflags", "dr6", "dr7"]
# M/IO, D/C, W/R
STATUS = {0: 'INTA', 2: 'IOR', 3: 'IOW', 4: 'CODE', 5: 'HALT', 6: 'MEMR', 7: 'MEMW'}
PREFIXES = {0x26, 0x2E, 0x36, 0x3E, 0x64, 0x65, 0x66, 0x67, 0xF0, 0xF2, 0xF3}


def _chunks(b):
    p = 0
    while p < len(b):
        t = b[p:p + 4]
        n = struct.unpack('<I', b[p + 4:p + 8])[0]
        yield t, b[p + 8:p + 8 + n]
        p += 8 + n


def _state(b):
    regs, ram = {}, {}
    for t, c in _chunks(b):
        if t == b'RG32':
            m = struct.unpack('<I', c[:4])[0]
            p = 4
            for i, nm in enumerate(REGS32):
                if m >> i & 1:
                    regs[nm] = struct.unpack('<I', c[p:p + 4])[0]
                    p += 4
        elif t == b'RAM ':
            n = struct.unpack('<I', c[:4])[0]
            for i in range(n):
                a, v = struct.unpack('<IB', c[4 + 5 * i:9 + 5 * i])
                ram[a] = v
    return regs, ram


def vectors(name):
    """The vectors of one file (`8B`, `66F7.6`), as dicts."""
    data = gzip.open(os.path.join(CORPUS, name + '.MOO.gz')).read()
    out = []
    for t, body in _chunks(data):
        if t != b'TEST':
            continue
        v = {'index': struct.unpack('<I', body[:4])[0], 'file': name}
        for tt, c in _chunks(body[4:]):
            if tt == b'NAME':
                v['name'] = c[4:].decode()
            elif tt == b'BYTS':
                v['bytes'] = list(c[4:])
            elif tt == b'INIT':
                v['regs'], v['ram'] = _state(c)
            elif tt == b'FINA':
                v['final'], v['final_ram'] = _state(c)
            elif tt == b'EXCP':
                v['exception'] = c[0]
            elif tt == b'CYCL':
                n = struct.unpack('<I', c[:4])[0]
                sz = (len(c) - 4) // n
                cyc = []
                for k in range(n):
                    r = c[4 + k * sz:4 + (k + 1) * sz]
                    cyc.append({'ads': r[0] & 1, 'addr': struct.unpack('<I', r[1:5])[0],
                                'status': r[11] & 0x0F, 't': r[12], 'data': struct.unpack('<H', r[9:11])[0]})
                v['cycles'] = cyc
        out.append(v)
    return out


def bus_cycles(v):
    """[(start cycle, kind, address)] of each bus cycle."""
    return [(k, STATUS.get(c['status'], '?%d' % c['status']), c['addr'])
            for k, c in enumerate(v['cycles']) if c['ads']]


def measured(v, harness=0):
    """Cycles outside bus cycles (2 clocks each), less the rig's."""
    bus = [b for b in bus_cycles(v) if b[1] not in ('HALT', '?1')]
    return len(v['cycles']) - 2 * len(bus) - harness


def prefixes(v):
    b = v['bytes']
    p = 0
    while p < len(b) and b[p] in PREFIXES:
        p += 1
    return p


if __name__ == '__main__':
    v = vectors(sys.argv[1])[int(sys.argv[2])]
    print(v['name'], ' '.join('%02x' % x for x in v['bytes']), 'cs:eip=%04x:%04x' % (v['regs']['cs'], v['regs']['eip']))
    for k, c in enumerate(v['cycles']):
        print('%3d %s %-5s T%d %06x' % (k, 'A' if c['ads'] else ' ', STATUS.get(c['status'], '?'), c['t'], c['addr']))
    print('measured', measured(v))

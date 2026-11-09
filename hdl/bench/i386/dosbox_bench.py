#!/usr/bin/env python3
"""Runs a disk image in the patched DOSBox (build_dosbox.sh) for a set
emulated time and measures it as ours is measured: by host instructions
(perf stat), not wall time alone.

    dosbox_bench.py [options] IMG T1 [T2]

boots IMG (a floppy; with --hd, a 615/4/17 fixed disk as drive C, the
geometry build_omf.sh makes) and stops at T1 emulated seconds; with T2,
runs again to T2 and reports the window between them (host instructions an
emulated second, the speed, the cycles given), cancelling boot and startup.

The patch keeps fast forward on, so emulated time is the cycles alone:
`--cycles N` is N cycles an emulated millisecond however fast the host is
(4300 is about what our 386 runs a game at). Keys are pressed at emulated
times, given as play_disk's PLAY_KEYS are (`seconds:code`, a set-1 make
code in hex), held 150 ms. Sound is mixed and discarded (nosound).
"""
import argparse, os, re, resource, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT = os.path.join(HERE, "..", "..", "target", "dosbox-bench", "dosbox")

p = argparse.ArgumentParser()
p.add_argument("img")
p.add_argument("t1", type=float)
p.add_argument("t2", type=float, nargs="?")
p.add_argument("--hd")
p.add_argument("--core", default="normal", choices=["normal", "dynamic"])
p.add_argument("--cycles", type=int, default=4300)
p.add_argument("--memsize", type=int, default=16)
p.add_argument("--keys", default=os.environ.get("PLAY_KEYS", ""))
p.add_argument("--frameskip", type=int, default=0, help="DOSBox draws one frame in this many plus one")
p.add_argument("--every", type=float, help="also report every this many emulated seconds")
p.add_argument("--shots", help="a directory: a screenshot at each --every report")
p.add_argument("--dosbox", default=os.environ.get("DOSBOX_BENCH", DEFAULT))
a = p.parse_args()

def keys_ms(keys):
    out = []
    for k in filter(None, keys.split(",")):
        t, code = k.split(":")
        out.append(f"{round(float(t) * 1000)}:{code}")
    return ",".join(out)

def conf():
    boot = []
    if a.hd:
        boot.append(f"imgmount 2 {os.path.abspath(a.hd)} -size 512,17,4,615 -t hdd -fs none")
    boot.append(f"boot {os.path.abspath(a.img)} -l a")
    captures = f"captures={os.path.abspath(a.shots)}\n" if a.shots else ""
    return f"""[sdl]
output=surface
[dosbox]
{captures}machine=vgaonly
memsize={a.memsize}
[render]
frameskip={a.frameskip}
[cpu]
core={a.core}
cputype=386
cycles=fixed {a.cycles}
[mixer]
nosound=true
[sblaster]
sbtype=sb2
sbbase=220
irq=5
dma=1
oplmode=opl2
[autoexec]
""" + "\n".join(boot) + "\n"

def limit():
    resource.setrlimit(resource.RLIMIT_AS, (8_000_000_000, 8_000_000_000))

def run(seconds):
    with tempfile.TemporaryDirectory() as d:
        path, stat = os.path.join(d, "bench.conf"), os.path.join(d, "perf.csv")
        open(path, "w").write(conf())
        env = dict(os.environ, SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy", TZ="UTC",
                   DOSBOX_BENCH_MS=str(round(seconds * 1000)), DOSBOX_BENCH_KEYS=keys_ms(a.keys))
        if a.every:
            env["DOSBOX_BENCH_EVERY"] = str(round(a.every * 1000))
        if a.shots:
            os.makedirs(a.shots, exist_ok=True)
            env["DOSBOX_BENCH_SHOTS"] = "1"
        r = subprocess.run(["perf", "stat", "-x,", "-e", "instructions:u", "-o", stat,
                            a.dosbox, "-conf", path],
                           env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                           timeout=3600, preexec_fn=limit)
        m = re.search(r"DOSBOX_BENCH end ms (\d+) cycles (\d+) instructions (\d+) wall ([\d.]+) tick (\w+) ram (\w+)", r.stdout)
        if not m:
            sys.exit(f"dosbox stopped without a report:\n{r.stdout[-3000:]}")
        for l in r.stdout.splitlines():
            if l.startswith("DOSBOX_BENCH at"):
                print(l)
        host = next(int(l.split(",")[0]) for l in open(stat) if "instructions" in l)
        ms, cycles, instr, wall = int(m[1]), int(m[2]), int(m[3]), float(m[4])
        print(f"DOSBOX {a.core} cycles/ms {a.cycles} to {ms / 1000:.3f} s: host {host / 1e9:.2f}G wall {wall:.3f} "
              f"speed {ms / 1000 / wall:.2f}x cycles {cycles} normal_instructions {instr} tick {m[5]} ram {m[6]}")
        return ms, host, wall, cycles, instr

r1 = run(a.t1)
if a.t2 is not None:
    r2 = run(a.t2)
    s = (r2[0] - r1[0]) / 1000
    print(f"DOSBOX {a.core} window {a.t1}-{a.t2} s: {(r2[1] - r1[1]) / s / 1e9:.3f}G host instructions an emulated second, "
          f"{(r2[3] - r1[3]) / s / 1e6:.2f} M cycles an emulated second"
          + (f", {(r2[4] - r1[4]) / s / 1e6:.2f} M instructions (normal core)" if a.core == "normal" else ""))

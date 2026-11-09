#!/usr/bin/env python3
"""Runs the 386 comparison (after build.sh): each tool, each workload, at N
and 0 passes, REPS times interleaved; prints marginal MIPS (median) per tool.

Ours reports the time between the program's markers (bench_compare in
targets/i386/at386_test.rs, on the at386vga crates the generate tests
write); the others' marginal time (N passes less 0) cancels their startup
and DOS boot. Guest instructions are ours, counted exactly (a REP iteration
counts as one). Tools: QEMU (TCG), Bochs (legacy BIOS, sync=none), DOSBox
(normal and dynamic cores, cycles=max). TOOLS=a,b runs some.
"""
import os, re, statistics, subprocess, sys, time

HDL = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
B = os.path.join(HDL, "target", "bench386")
SCR = os.path.join(HDL, "target", "bench386")
REPS = int(os.environ.get("REPS", "3"))
WORK = ["chk", "mix", "snd"]

def timed(cmd, env=None, cwd=None):
    t = time.perf_counter()
    r = subprocess.run(cmd, env=env, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=3600)
    return time.perf_counter() - t, r.stdout

def qemu(v):
    return timed(["qemu-system-i386", "-accel", "tcg", "-m", "16", "-drive", f"file={B}/{v}.img,format=raw,if=floppy",
                  "-boot", "a", "-display", "none", "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-no-reboot",
                  "-audiodev", "none,id=snd0", "-device", "sb16,audiodev=snd0"])[0]

def bochs(v):
    rc = open(f"{B}/bochsrc.tmpl").read().replace("IMG", f"{B}/{v}.img")
    path = f"{B}/bochsrc.{v}"
    open(path, "w").write(rc)
    return timed(["xvfb-run", "-a", "bochs", "-q", "-f", path, "-rc", f"{B}/bochs.cmds"])[0]

def dosbox(core):
    def run(v):
        conf = open(f"{B}/dosbox_{core}.conf").read().replace("DIR", f"{B}/dir_{v}")
        path = f"{B}/dosbox.{v}.conf"
        open(path, "w").write(conf)
        env = dict(os.environ, SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy")
        return timed(["dosbox", "-conf", path], env=env)[0]
    return run

# Ours: the time between the program's markers (no boot, no harness).
ours_counts = {}
def ours(profile):
    def run(v):
        env = dict(os.environ, BENCH_DISK=f"{B}/{v}.img", CARGO_TARGET_DIR=f"{SCR}/cargo-{profile}")
        _, out = timed(["cargo", "test", "-q", "--release", "--manifest-path", f"{HDL}/target/gen-tests/at386vga-{profile}/Cargo.toml",
                        "--test", "at386", "--", "--ignored", "--nocapture", "bench_compare"], env=env)
        m = re.search(r"run_instructions (\d+) run_seconds ([\d.]+)", out)
        if not m:
            sys.exit(f"ours {profile} {v}: {out[-2000:]}")
        ours_counts[v] = int(m.group(1))
        return float(m.group(2))
    return run

TOOLS = {
    "ours trace": ours("trace"),
    "ours jit": ours("jit"),
    "ours approx": ours("approx"),
    "ours blocks": ours("blocks"),
    "ours quantum": ours("quantum"),
    "bochs": bochs,
    "dosbox normal": dosbox("normal"),
    "dosbox dynamic": dosbox("dynamic"),
    "qemu tcg": qemu,
}
only = os.environ.get("TOOLS")
if only:
    TOOLS = {k: v for k, v in TOOLS.items() if k in only.split(",")}

times = {(t, w): [] for t in TOOLS for w in WORK}
for rep in range(REPS):
    for t, run in TOOLS.items():
        for w in WORK:
            if t.startswith("ours"):
                d = run(f"{w}n")
            else:
                d = run(f"{w}n") - run(f"{w}0")
            times[(t, w)].append(d)
            print(f"rep {rep} {t:15s} {w}: {d:.3f} s", flush=True)

counts = {w: ours_counts.get(f"{w}n") for w in WORK}
print("\ninstructions:", counts)
print(f"{'tool':16s}" + "".join(f"{w + ' MIPS':>12s}" for w in WORK))
for t in TOOLS:
    row = f"{t:16s}"
    for w in WORK:
        d = statistics.median(times[(t, w)])
        row += f"{(counts[w] or 0) / d / 1e6:12.1f}"
    print(row)

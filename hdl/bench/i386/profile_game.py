#!/usr/bin/env python3
"""profile_game.py PROFILE GAME: where ours spends host instructions over a
game's window (compare_games.py's windows and crates: run it first),
drawing none: `perf record` at the window's two ends, the samples sorted by
symbol into categories (compiled code, the JIT's glue, the interpreter, the
board's buses, devices, the VGA, sound, compiling), the end less the start.
Per guest instruction, as compare_games.py counts them."""
import os, re, subprocess, sys, collections
HERE = os.path.dirname(os.path.abspath(__file__))
H = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)
import perfbin
PERF = perfbin.find()
S = os.path.join(H, "target", "compare")
prof, game = sys.argv[1], sys.argv[2]
# compare_games.py's windows.
G = {"lemm": ("lemm.img", None, 40, 90, ""), "jetpack": ("jetpack.img", None, 60, 180, ""),
     "omf": ("omf.img", "omf_hd.img", 160, 240, "45:1c")}[game]
exe = max((os.path.join(d, f) for d in [f"{H}/target/compare/cargo-{prof}/release/deps"] for f in os.listdir(d)
           if f.startswith("at386-") and not f.endswith(".d")), key=os.path.getmtime)
PERIOD = 5_000_000
def cat(dso, sym):
    if dso.startswith("[JIT") or dso in ("[unknown]", "[anon]") or dso.startswith("perf-"):
        return "compiled code"
    if "cranelift" in sym or "regalloc" in sym or "machine_jit::" in sym or "target_lexicon" in sym:
        return "compiling"
    if "::Vga" in sym or "Vga::" in sym or "catch_up_vga" in sym:
        return "vga"
    if "Opl2" in sym or "SoundBlaster" in sym or "catch_up_opl" in sym or "catch_up_sb" in sym:
        return "sound"
    if "jit_burst" in sym or "jit_enter" in sym or "jit_follow" in sym or "jit_lookup" in sym or "jit_compile" in sym or "_c(" in sym or "jit_step_" in sym or "jit_" in sym:
        return "jit glue (entries, shims)"
    if "catch_up_" in sym or "board_sync" in sym or "after_step" in sym or "Pit" in sym or "Pic" in sym or "Dma" in sym or "Kbc" in sym or "Rtc" in sym or "Fdc" in sym or "Wd1003" in sym:
        return "devices"
    if "as machine_at386_vga::Bus>" in sym or "Ctx_" in sym or "Board::" in sym or "AtBus" in sym:
        return "board: buses"
    if "I386<B>" in sym or "machine_at386_vga::" in sym:
        return "interpreter"
    if dso.startswith("libc") or "memmove" in sym or "memcpy" in sym or "memset" in sym:
        return "libc (memcpy...)"
    return "other: " + (sym[:40] if sym else dso)
def run(t):
    env = dict(os.environ, PLAY_DISK=f"{H}/target/bench386/{G[0]}", PLAY_SECONDS=str(t), PLAY_EVERY="1000000",
               PLAY_DRAW="0", PLAY_KEYS=G[4])
    if G[1]: env["PLAY_HD"] = f"{H}/target/bench386/{G[1]}"
    data = f"{S}/prof-{prof}-{game}-{t}.data"
    r = subprocess.run([PERF, "record", "-q", "-e", "instructions:u", "-c", str(PERIOD), "-o", data, exe,
                        "--ignored", "--nocapture", "--exact", "play_disk"], env=env, capture_output=True, text=True)
    m = re.search(r"instructions (\d+)", r.stdout + r.stderr)
    rep = subprocess.run([PERF, "report", "-i", data, "--stdio", "--sort", "dso,sym", "-F", "sample,dso,sym", "-t", "|"],
                         capture_output=True, text=True).stdout
    c = collections.Counter()
    for l in rep.splitlines():
        f = l.split("|")
        if len(f) == 3 and f[0].strip().isdigit():
            c[cat(f[1].strip(), f[2].strip().removeprefix("[.] ").strip())] += int(f[0]) * PERIOD
    return c, int(m.group(1))
c1, i1 = run(G[2]); c2, i2 = run(G[3])
s = G[3] - G[2]; gi = (i2 - i1) / s
tot = sum(c2.values()) - sum(c1.values())
print(f"{prof} {game}: {tot / s / 1e9:.2f} G host instructions an emulated second, {gi / 1e6:.2f} M guest; {tot / s / gi:.0f} a guest instruction")
for k in sorted(set(c1) | set(c2), key=lambda k: -(c2[k] - c1[k])):
    d = c2[k] - c1[k]
    if abs(d) / tot > 0.005:
        print(f"  {d / tot * 100:5.1f}%  {d / s / gi:6.1f} a guest instr  {k}")

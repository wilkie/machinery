"""Finds a perf that runs, for the bench scripts' instruction counts.

$PERF if set; else `perf` on the PATH if it runs (Ubuntu's /usr/bin/perf is
a wrapper that refuses a kernel it has no tools for, as under WSL); else a
perf from any installed linux-tools (counting instructions works across
kernel versions: `apt install linux-tools-generic`).
"""
import glob, os, shutil, subprocess, sys


def _runs(path):
    try:
        return subprocess.run([path, "--version"], capture_output=True, timeout=30).returncode == 0
    except OSError:
        return False


def find():
    if os.environ.get("PERF"):
        return os.environ["PERF"]
    on_path = shutil.which("perf")
    if on_path and _runs(on_path):
        return on_path
    for path in sorted(glob.glob("/usr/lib/linux-tools*/*/perf") + glob.glob("/usr/lib/linux-tools*/*/bin/perf")
                       + glob.glob("/usr/lib/linux-tools-*/perf"), reverse=True):
        if _runs(path):
            return path
    sys.exit("no perf that runs: install it (Ubuntu: apt install linux-tools-generic) or set PERF")

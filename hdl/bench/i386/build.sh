#!/bin/sh
# Builds the 386 comparison's programs and boot disks into
# hdl/target/bench386 (FreeDOS 1.4's boot disk, from ~/roms or PC_ROMS, as
# fetch_games.sh fetches it: not in the repository). Needs nasm and mtools.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
. "$HERE/freedos.sh"
mkdir -p "$OUT"
cp "$HERE"/bochsrc.tmpl "$HERE"/dosbox_*.conf "$HERE"/bochs.cmds "$OUT"/
for k in chk mix snd; do
    n=8000; [ $k = mix ] && n=36000
    (cd "$HERE" && nasm -f bin -DN=0 -o "$OUT/${k}0.com" $k.asm && nasm -f bin -DN=$n -o "$OUT/${k}n.com" $k.asm)
done
for v in chk0 chkn mix0 mixn snd0 sndn; do
    freedos_floppy "$OUT/$v.img"
    printf 'BENCH\r\n' > "$OUT/autoexec.bat"
    mcopy -o -i "$OUT/$v.img" "$OUT/autoexec.bat" ::AUTOEXEC.BAT
    mcopy -o -i "$OUT/$v.img" "$OUT/$v.com" ::BENCH.COM
    mkdir -p "$OUT/dir_$v"
    cp "$OUT/$v.com" "$OUT/dir_$v/BENCH.COM"
done
echo "built into $OUT"

#!/bin/sh
# Builds the 386 comparison's programs and boot disks into
# hdl/target/bench386 (from MS-DOS 3.30's first disk in ~/roms or PC_ROMS:
# nothing of it is in the repository). Needs nasm and mtools.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
mkdir -p "$OUT"
cp "$HERE"/bochsrc.tmpl "$HERE"/dosbox_*.conf "$HERE"/bochs.cmds "$OUT"/
for k in chk mix snd; do
    n=8000; [ $k = mix ] && n=36000
    (cd "$HERE" && nasm -f bin -DN=0 -o "$OUT/${k}0.com" $k.asm && nasm -f bin -DN=$n -o "$OUT/${k}n.com" $k.asm)
done
for v in chk0 chkn mix0 mixn snd0 sndn; do
    cp "$ROMS/msdos330/DISK01.IMG" "$OUT/$v.img"
    chmod u+w "$OUT/$v.img"
    # DOS alone, so the program fits: the utilities go.
    for f in $(mdir -b -i "$OUT/$v.img" | sed 's|^::/||' | grep -viE '^(IO.SYS|MSDOS.SYS|COMMAND.COM)$'); do mdel -i "$OUT/$v.img" "::$f"; done
    printf 'BENCH\r\n' > "$OUT/autoexec.bat"
    mcopy -o -i "$OUT/$v.img" "$OUT/autoexec.bat" ::AUTOEXEC.BAT
    mcopy -o -i "$OUT/$v.img" "$OUT/$v.com" ::BENCH.COM
    mkdir -p "$OUT/dir_$v"
    cp "$OUT/$v.com" "$OUT/dir_$v/BENCH.COM"
done
echo "built into $OUT"

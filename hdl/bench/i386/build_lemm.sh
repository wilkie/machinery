#!/bin/sh
# Builds a boot floppy running Psygnosis' Xmas Lemmings 1992 demo (a free
# promotional release; https://archive.org/details/msdos_xmaslm92) into
# hdl/target/bench386/lemm.img, from ~/roms (or PC_ROMS): MS-DOS 3.30's
# first disk and the demo's xmaslm92.zip in xmaslm92/. Neither is in the
# repository. Needs mtools and unzip.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"
unzip -q "$ROMS/xmaslm92/xmaslm92.zip" -d "$TMP/files"
cp "$ROMS/msdos330/DISK01.IMG" "$OUT/lemm.img"
chmod u+w "$OUT/lemm.img"
for f in $(mdir -b -i "$OUT/lemm.img" | sed 's|^::/||' | grep -viE '^(IO.SYS|MSDOS.SYS|COMMAND.COM)$'); do mdel -i "$OUT/lemm.img" "::$f"; done
# The VGA version, as LEMMINGS.BAT runs it for a VGA.
printf 'VGAL -X -V -X\r\n' > "$TMP/autoexec.bat"
mcopy -o -i "$OUT/lemm.img" "$TMP/autoexec.bat" ::AUTOEXEC.BAT
for f in "$TMP"/files/*; do mcopy -o -i "$OUT/lemm.img" "$f" ::; done
echo "built $OUT/lemm.img"

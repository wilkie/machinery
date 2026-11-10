#!/bin/sh
# Builds a boot floppy running Psygnosis' Xmas Lemmings 1992 demo (a free
# promotional release; https://archive.org/details/msdos_xmaslm92) into
# hdl/target/bench386/lemm.img, from ~/roms (or PC_ROMS): FreeDOS 1.4's
# boot disk and the demo's xmaslm92.zip, both fetched by fetch_games.sh.
# Neither is in the repository. Needs mtools and unzip.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
. "$HERE/freedos.sh"
mkdir -p "$OUT"
unzip -q "$ROMS/xmaslm92/xmaslm92.zip" -d "$TMP/files"
freedos_floppy "$OUT/lemm.img"
# The VGA version, as LEMMINGS.BAT runs it for a VGA.
printf 'VGAL -X -V -X\r\n' > "$TMP/autoexec.bat"
mcopy -o -i "$OUT/lemm.img" "$TMP/autoexec.bat" ::AUTOEXEC.BAT
for f in "$TMP"/files/*; do mcopy -o -i "$OUT/lemm.img" "$f" ::; done
echo "built $OUT/lemm.img"

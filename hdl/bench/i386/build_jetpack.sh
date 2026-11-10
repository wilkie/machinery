#!/bin/sh
# Builds a 1.2 MB boot floppy running Adept Software's Jetpack 1.5
# (freeware since 1998; https://archive.org/details/Jetpack) into
# hdl/target/bench386/jetpack.img, from ~/roms (or PC_ROMS): FreeDOS 1.4's
# boot disk and jetpak15.zip, both fetched by fetch_games.sh. Neither is
# in the repository. Needs mtools and unzip.
#
# The game runs with its Sound Blaster samples (/S3) on the board's card
# (BLASTER: 220h, IRQ 5, DMA 1).
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
. "$HERE/freedos.sh"
mkdir -p "$OUT"
unzip -q "$ROMS/jetpack/jetpak15.zip" -d "$TMP/files"
freedos_floppy "$OUT/jetpack.img"
printf 'SET BLASTER=A220 I5 D1\r\nJETPACK /S3\r\n' > "$TMP/autoexec.bat"
mcopy -o -i "$OUT/jetpack.img" "$TMP/autoexec.bat" ::AUTOEXEC.BAT
for f in "$TMP"/files/jetpak15/*; do [ -f "$f" ] && mcopy -o -i "$OUT/jetpack.img" "$f" ::; done
echo "built $OUT/jetpack.img"

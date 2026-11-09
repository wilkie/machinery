#!/bin/sh
# Builds a 1.2 MB boot floppy running Adept Software's Jetpack 1.5
# (freeware since 1998; https://archive.org/details/Jetpack) into
# hdl/target/bench386/jetpack.img, from ~/roms (or PC_ROMS): MS-DOS 3.30's
# first disk (its boot sector and system files) and jetpak15.zip in
# jetpack/. Neither is in the repository. Needs mtools and unzip.
#
# The game runs with its Sound Blaster samples (/S3) on the board's card
# (BLASTER: 220h, IRQ 5, DMA 1).
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"
unzip -q "$ROMS/jetpack/jetpak15.zip" -d "$TMP/files"
DOS="$ROMS/msdos330/DISK01.IMG"
# DOS 3.30's boot sector, its geometry fields made a 1.2 MB disk's.
dd if="$DOS" of="$TMP/boot.bin" bs=512 count=1 2>/dev/null
rm -f "$OUT/jetpack.img"
mformat -C -f 1200 -B "$TMP/boot.bin" -i "$OUT/jetpack.img" ::
# (mformat writes a DOS 4 extended parameter block from 26h, where DOS
# 3.30's boot code is: its code back, from 1Eh, after the geometry.)
dd if="$TMP/boot.bin" of="$OUT/jetpack.img" bs=1 skip=30 seek=30 count=482 conv=notrunc 2>/dev/null
# The system files first (the boot sector loads IO.SYS from the first
# entries), then the shell.
for f in IO.SYS MSDOS.SYS COMMAND.COM; do mcopy -i "$DOS" "::$f" "$TMP/$f"; mcopy -i "$OUT/jetpack.img" "$TMP/$f" "::$f"; done
mattrib -i "$OUT/jetpack.img" +s +h ::IO.SYS ::MSDOS.SYS
printf 'SET BLASTER=A220 I5 D1\r\nJETPACK /S3\r\n' > "$TMP/autoexec.bat"
mcopy -o -i "$OUT/jetpack.img" "$TMP/autoexec.bat" ::AUTOEXEC.BAT
for f in "$TMP"/files/jetpak15/*; do [ -f "$f" ] && mcopy -o -i "$OUT/jetpack.img" "$f" ::; done
echo "built $OUT/jetpack.img"

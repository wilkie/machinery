#!/bin/sh
# Builds One Must Fall: 2097 (freeware since 1999;
# https://archive.org/details/OneMustFall2097) for the AT board into
# hdl/target/bench386: omf_hd.img, a 20 MB fixed disk (type 2: 615
# cylinders, 4 heads, 17 sectors) with one FAT16 partition holding the
# game in \OMF, and omf.img, MS-DOS 3.30's boot floppy starting it, from
# ~/roms (or PC_ROMS): msdos330/DISK01.IMG and OMF21cd-playable.ZIP in
# omf2097/. None of it is in the repository. Needs mtools, unzip, python3.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../../target/bench386"
ROMS=${PC_ROMS:-$HOME/roms}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"
unzip -q "$ROMS/omf2097/OMF21cd-playable.ZIP" -d "$TMP/omf"
# The disk: a master boot record with one partition (type 04h, FAT16 under
# 32 MB, as DOS 3.30 makes it) from the second track's... first head's
# second sector on: cylinder 0, head 1, sector 1 (LBA 17) to the end.
python3 - "$OUT/omf_hd.img" <<'PY'
import sys, struct
cyl, heads, spt = 615, 4, 17
total = cyl * heads * spt
start = spt
size = total - start
def chs(lba):
    c, r = divmod(lba, heads * spt)
    h, s = divmod(r, spt)
    return bytes([h, ((c >> 2) & 0xC0) | (s + 1), c & 0xFF])
mbr = bytearray(512)
mbr[446:462] = bytes([0x80]) + chs(start) + bytes([0x04]) + chs(total - 1) + struct.pack('<II', start, size)
mbr[510:512] = b'\x55\xaa'
with open(sys.argv[1], 'wb') as f:
    f.write(mbr)
    f.truncate(total * 512)
PY
# The partition, formatted as DOS 3.3 would: 2 KB clusters, 512 root
# entries.
mformat -i "$OUT/omf_hd.img@@8704" -T 41803 -h 4 -s 17 -H 17 -c 4 -r 32 -M 512 ::
mmd -i "$OUT/omf_hd.img@@8704" ::OMF
mcopy -s -i "$OUT/omf_hd.img@@8704" "$TMP"/omf/* ::OMF/
# The boot floppy: DOS alone, then C:\OMF.
cp "$ROMS/msdos330/DISK01.IMG" "$OUT/omf.img"
chmod u+w "$OUT/omf.img"
for f in $(mdir -b -i "$OUT/omf.img" | sed 's|^::/||' | grep -viE '^(IO.SYS|MSDOS.SYS|COMMAND.COM)$'); do mdel -i "$OUT/omf.img" "::$f"; done
printf 'C:\r\nCD \\OMF\r\nOMF\r\n' > "$TMP/autoexec.bat"
mcopy -o -i "$OUT/omf.img" "$TMP/autoexec.bat" ::AUTOEXEC.BAT
# DOS's defaults (8 files, 2 buffers) are too few: the game keeps files
# open, and loads megabytes through the FAT.
printf 'FILES=30\r\nBUFFERS=30\r\n' > "$TMP/config.sys"
mcopy -o -i "$OUT/omf.img" "$TMP/config.sys" ::CONFIG.SYS
echo "built $OUT/omf_hd.img and $OUT/omf.img"

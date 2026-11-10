#!/bin/sh
# Builds the free ROMs the 386 AT can boot instead of AMI's and IBM's, into
# ~/roms/free (or $PC_ROMS/free), from the Bochs project's 2.7 release
# (fetched and checked; not in the repository):
# - BIOS-bochs-legacy-at: its "legacy" system BIOS (LGPL; for ISA
#   machines, without PCI), patched (patches/rombios-2.7-at.patch) for a
#   real AT where it relied on Bochs's hardware: it seeks before reading,
#   writing or formatting a diskette (a real 765 does them where its head
#   is; Bochs's own controller moved it), and it gates A20 through the
#   keyboard controller (an AT has no port 92h, the PS/2's fast gate),
#   closing it before it boots, as an AT's POST does. Built as the release builds it, with its date fixed
#   (unpatched, this makes Ubuntu's bochsbios byte for byte, and the
#   release's binary but for the weekday in a version string).
# - VGABIOS-lgpl-latest: the LGPL VGABIOS 0.8a, as the release ships it.
# Use them with the AT tests' PC_BIOS and PC_VGABIOS. Needs bcc and as86
# (Ubuntu: bcc, bin86), gcc and perl.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
ROMS=${PC_ROMS:-$HOME/roms}
OUT="$ROMS/free"
mkdir -p "$OUT"
TAR="$OUT/bochs-2.7.tar.gz"
SUM=a010ab1bfdc72ac5a08d2e2412cd471c0febd66af1d9349bc0d796879de5b17a
if ! { [ -f "$TAR" ] && echo "$SUM  $TAR" | sha256sum -c --status -; }; then
  curl -sSL --fail -o "$TAR.part" "https://sourceforge.net/projects/bochs/files/bochs/2.7/bochs-2.7.tar.gz/download"
  echo "$SUM  $TAR.part" | sha256sum -c --quiet -
  mv "$TAR.part" "$TAR"
fi
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
tar -xzf "$TAR" -C "$TMP" bochs-2.7/bios
cd "$TMP/bochs-2.7"
patch -p1 < "$HERE/patches/rombios-2.7-at.patch"
# (configure would write this; the BIOS takes only the version from it.)
printf '#define VERSION "2.7"\n' > bxversion.h
cd bios
gcc -o biossums biossums.c
gcc '-DBIOS_BUILD_DATE="08/01/21"' -DLEGACY -E -P rombios.c > _rombiosl_.c
bcc -o rombiosl.s -C-c -D__i86__ -0 -S _rombiosl_.c
sed -e 's/^\.text//' -e 's/^\.data//' rombiosl.s > _rombiosl_.s
as86 _rombiosl_.s -b tmpl.bin -u- -w- -g -0 -j -O -l rombiosl.txt
./biossums tmpl.bin
cp tmpl.bin "$OUT/BIOS-bochs-legacy-at"
cp VGABIOS-lgpl-latest "$OUT/VGABIOS-lgpl-latest"
echo "built $OUT/BIOS-bochs-legacy-at and $OUT/VGABIOS-lgpl-latest"

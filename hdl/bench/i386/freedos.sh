# Sourced by the build scripts: boot floppies of FreeDOS 1.4 (GPL; its
# Floppy Edition's 1.2 MB boot disk, which fetch_games.sh fetches into
# $ROMS/freedos/120m/x86BOOT.img, and HimemX 3.39, FreeDOS 1.4's XMS
# driver, from its release, in $ROMS/freedos/HimemX339.zip).
#
# freedos_floppy IMG: a fresh 1.2 MB disk at IMG that boots FreeDOS's kernel
# (KERNEL.SYS, for an 8086 and up) and its shell (FreeCOM, COMMAND.COM), with
# FDCONFIG.SYS: no boot delay or F5/F8 prompt; HimemX, so FreeCOM swaps
# itself to XMS (without it FreeDOS leaves Jetpack 20 KB short of the memory
# it needs), but not DOS=HIGH: programs would load below 64 KB, where old
# packed executables (Lemmings' VGAL) wrap at 1 MB and need A20 off, which
# our direct-memory profiles do not model; 30 files and 30 buffers (the
# games keep files open and load megabytes through the FAT).
# The caller adds
# AUTOEXEC.BAT and its files. FreeDOS's boot sector reads the disk's
# parameter block and finds KERNEL.SYS by name, so mformat's block serves.
FD_BOOT="${ROMS:?}/freedos/120m/x86BOOT.img"
FD_HIMEMX="$ROMS/freedos/HimemX339.zip"

# The same disk on every build: files dated 1993-01-01 12:00 UTC (as the
# patched DOSBox's clock starts), a fixed volume serial.
SOURCE_DATE_EPOCH=725889600
TZ=UTC
export SOURCE_DATE_EPOCH TZ

freedos_floppy() {
  for f in "$FD_BOOT" "$FD_HIMEMX"; do
    if [ ! -f "$f" ]; then
      echo "no $f: run fetch_games.sh" >&2
      exit 1
    fi
  done
  fd_tmp=$(mktemp -d)
  dd if="$FD_BOOT" of="$fd_tmp/boot.bin" bs=512 count=1 2>/dev/null
  rm -f "$1"
  truncate -s 1228800 "$1"
  mformat -f 1200 -N 19930101 -B "$fd_tmp/boot.bin" -i "$1" ::
  mcopy -i "$FD_BOOT" ::KERNEL.SYS "$fd_tmp/KERNEL.SYS"
  mcopy -i "$FD_BOOT" ::freedos/bin/command.com "$fd_tmp/COMMAND.COM"
  unzip -p "$FD_HIMEMX" HimemX.exe > "$fd_tmp/HIMEMX.EXE"
  printf 'SWITCHES=/F /N\r\nDEVICE=HIMEMX.EXE\r\nFILES=30\r\nBUFFERS=30\r\n' > "$fd_tmp/FDCONFIG.SYS"
  mcopy -i "$1" "$fd_tmp/KERNEL.SYS" "$fd_tmp/COMMAND.COM" "$fd_tmp/HIMEMX.EXE" "$fd_tmp/FDCONFIG.SYS" ::
  rm -rf "$fd_tmp"
}

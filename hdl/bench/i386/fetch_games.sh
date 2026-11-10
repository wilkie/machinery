#!/bin/sh
# Fetches what the benchmarks' disks are made of (build.sh, build_lemm.sh,
# build_jetpack.sh, build_omf.sh) into ~/roms (or PC_ROMS), where those
# scripts read them, checking each file's SHA-256: FreeDOS 1.4 (GPL) from
# ibiblio, FreeDOS's own mirror, its XMS driver HimemX from its release on
# GitHub, and the freeware games from archive.org.
# Files already there and correct are kept. Not in the repository.
#
# The board's ROMs: AMI's AT BIOS (BIOS_5170_30APR89_U27/U47_AMI_27256.BIN)
# and IBM's VGA BIOS (ibm_vga.bin) are not free and are not fetched;
# build_bios.sh makes free ones (`compare_games.py --free-roms`).
set -e
ROMS=${PC_ROMS:-$HOME/roms}

get() { # url, file, directory under ROMS, sha256
  dest="$ROMS/$3/$2"
  if [ -f "$dest" ] && echo "$4  $dest" | sha256sum -c --status -; then
    echo "have $dest"
    return
  fi
  mkdir -p "$ROMS/$3"
  curl -sSL --fail -o "$dest.part" "$1"
  echo "$4  $dest.part" | sha256sum -c --quiet -
  mv "$dest.part" "$dest"
  echo "fetched $dest"
}
fetch() { # archive.org item, file, directory under ROMS, sha256
  get "https://archive.org/download/$1/$2" "$2" "$3" "$4"
}

# FreeDOS 1.4's Floppy Edition; its 1.2 MB boot disk is what the disks boot.
get https://www.ibiblio.org/pub/micro/pc-stuff/freedos/files/distributions/1.4/FD14-FloppyEdition.zip \
  FD14-FloppyEdition.zip freedos 45b1fa7c52dd996c3bfa5e352ffcd410781b952a6ad629f15a4c9ec4bbaefc5a
BOOT="$ROMS/freedos/120m/x86BOOT.img"
SUM=03df6088be016e57a6c44275f5bb9ab0244db71de1360957fd76ba83243b6a77
if ! { [ -f "$BOOT" ] && echo "$SUM  $BOOT" | sha256sum -c --status -; }; then
  mkdir -p "$ROMS/freedos/120m"
  unzip -p "$ROMS/freedos/FD14-FloppyEdition.zip" 120m/x86BOOT.img > "$BOOT.part"
  echo "$SUM  $BOOT.part" | sha256sum -c --quiet -
  mv "$BOOT.part" "$BOOT"
  echo "extracted $BOOT"
fi
# HimemX 3.39, FreeDOS 1.4's XMS driver (its HimemX.exe is the one FreeDOS
# 1.4 ships), from its author's release.
get https://github.com/Baron-von-Riedesel/HimemX/releases/download/v3.39/HimemX339.zip \
  HimemX339.zip freedos c898e55d17fc2b2bd2a3e21095a4a6958b4e43f18509210f0625532e1bfdf8a1

# Psygnosis' Xmas Lemmings 1992 demo (a free promotional release).
fetch msdos_xmaslm92 xmaslm92.zip xmaslm92 \
  558efcdbae165f624427ff2259a673178f5dc35f08acf85532dbae292bfb9690
# Adept Software's Jetpack 1.5 (freeware since 1998).
fetch Jetpack jetpak15.zip jetpack \
  db3f1a48ad21d7fcdd1956048b4cca9bc8f6ea21e78f981408399bee9a757b97
# Diversions Entertainment's One Must Fall: 2097 (freeware since 1999).
fetch OneMustFall2097 OMF21cd-playable.ZIP omf2097 \
  a59e0a7b834c7f2340370c065e48ae4a3b275b3770f75b8d1a5dd8ad88b98a74

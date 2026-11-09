#!/bin/sh
# DOSBox 0.74-3 patched for benchmarking (patches/dosbox-0.74-3.patch:
# emulated time from cycles alone, keys at emulated times, reports; see
# dosbox_bench.py), built into hdl/target/dosbox-bench (or $1). Its SDL 1.2
# is sdl12-compat 1.2.68, built there too over the system's SDL2 (needs
# SDL2's development files, cmake, autotools). Sources are fetched at
# pinned releases and checked; neither is kept in the repository.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${1:-$HERE/../../target/dosbox-bench}
mkdir -p "$OUT/dl" "$OUT/src"
OUT=$(cd "$OUT" && pwd)

fetch() { # url file sha256
  [ -f "$OUT/dl/$2" ] || curl -sSL -o "$OUT/dl/$2" "$1"
  echo "$3  $OUT/dl/$2" | sha256sum -c -
}
fetch "https://sourceforge.net/projects/dosbox/files/dosbox/0.74-3/dosbox-0.74-3.tar.gz/download" \
  dosbox-0.74-3.tar.gz c0d13dd7ed2ed363b68de615475781e891cd582e8162b5c3669137502222260a
fetch "https://github.com/libsdl-org/sdl12-compat/archive/refs/tags/release-1.2.68.tar.gz" \
  sdl12-compat-1.2.68.tar.gz 63c6e4dcc1154299e6f363c872900be7f3dcb3e42b9f8f57e05442ec3d89d02d

if [ ! -x "$OUT/sdl12/bin/sdl-config" ]; then
  rm -rf "$OUT/src/sdl12-compat-release-1.2.68"
  tar -xzf "$OUT/dl/sdl12-compat-1.2.68.tar.gz" -C "$OUT/src"
  cmake -S "$OUT/src/sdl12-compat-release-1.2.68" -B "$OUT/build-sdl12" \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$OUT/sdl12" -DSDL12TESTS=OFF
  cmake --build "$OUT/build-sdl12" -j"$(nproc)"
  cmake --install "$OUT/build-sdl12"
fi

rm -rf "$OUT/src/dosbox-0.74-3"
tar -xzf "$OUT/dl/dosbox-0.74-3.tar.gz" -C "$OUT/src"
cd "$OUT/src/dosbox-0.74-3"
patch -p1 < "$HERE/patches/dosbox-0.74-3.patch"
./configure --with-sdl-prefix="$OUT/sdl12" --disable-opengl LDFLAGS="-Wl,-rpath,$OUT/sdl12/lib"
make -j"$(nproc)"
cp src/dosbox "$OUT/dosbox"
echo "built $OUT/dosbox"

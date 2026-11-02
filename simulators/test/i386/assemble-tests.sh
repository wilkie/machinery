#!/bin/bash
# Assembles the 386 test programs with nasm: each .asm in a suite becomes
# bin/<name>.com (loaded at 0000:0100), with a hex dump and a disassembly.
cd "$(dirname "$0")"
for suite in protected; do
  (
    cd "$suite" || exit 1
    mkdir -p bin dsm
    for f in *.asm; do
      if [[ "${f}" -nt "./bin/${f%.asm}.com" || common.inc -nt "./bin/${f%.asm}.com" ]]; then
        echo "${suite}/${f} -> bin/${f%.asm}.com"
        nasm -I. "${f}" -f bin -o "./bin/${f%.asm}.com" || continue
        hexdump -C "./bin/${f%.asm}.com" > "./dsm/${f%.asm}.hex"
        objdump -D -mi386 -Maddr16,data16 -bbinary "./bin/${f%.asm}.com" > "./dsm/${f%.asm}.dsm"
      fi
    done
  )
done

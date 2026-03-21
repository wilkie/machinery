#!/bin/bash

# Loop through all .asm files and use nasm to generate binaries for the tests
cd simple
mkdir -p bin
mkdir -p dsm
for f in *.asm; do
  if [[ "${f}" -nt "./bin/${f%.asm}.com" ]]; then
    echo "${f} -> ${f%.asm}.com -> ${f%.asm}.hex, ${f%.asm}.dsm"
    nasm ${f} -f bin -o ./bin/${f%.asm}.com || continue
    hexdump -C ./bin/${f%.asm}.com > ./dsm/${f%.asm}.hex
    objdump -D -mi386 -Maddr16,data16 -bbinary ./bin/${f%.asm}.com > ./dsm/${f%.asm}.dsm
  fi
done
cd ..
cd complex
mkdir -p bin
mkdir -p dsm
for f in *.asm; do
  if [[ "${f}" -nt "./bin/${f%.asm}.com" ]]; then
    echo "${f} -> ${f%.asm}.com -> ${f%.asm}.hex, ${f%.asm}.dsm"
    nasm ${f} -f bin -o ./bin/${f%.asm}.com || continue
    hexdump -C ./bin/${f%.asm}.com > ./dsm/${f%.asm}.hex
    objdump -D -mi386 -Maddr16,data16 -bbinary ./bin/${f%.asm}.com > ./dsm/${f%.asm}.dsm
  fi
done
cd ..
cd protected
mkdir -p bin
mkdir -p dsm
for f in *.asm; do
  if [[ "${f}" -nt "./bin/${f%.asm}.com" ]]; then
    echo "${f} -> ${f%.asm}.com -> ${f%.asm}.hex, ${f%.asm}.dsm"
    nasm ${f} -f bin -o ./bin/${f%.asm}.com || continue
    hexdump -C ./bin/${f%.asm}.com > ./dsm/${f%.asm}.hex
    objdump -D -mi386 -Maddr16,data16 -bbinary ./bin/${f%.asm}.com > ./dsm/${f%.asm}.dsm
  fi
done
cd ..

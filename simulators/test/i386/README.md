# 386 tests

`SingleStepTests/` (not checked in) is the SingleStepTests 80386 hardware
corpus: real mode only.

`protected/` holds self-checking programs for what that corpus cannot
reach: 32-bit segments, gates and tasks, paging, and virtual-8086 mode.
They use the harness of the 286 tests (`../i286/README.md`): each is a
`.com` image loaded at `0000:0100`; `int 0x22` asserts `AL == AH`,
`int 0x23` asserts `AX == BX`, and, new here, `int 0x24` asserts
`EAX == EBX`; `int 0x21` with `AH = 0x4C` ends the program. The harness
handles those interrupts itself, in any mode, so a program that pages
must keep its code identity-mapped. `protected/common.inc` holds the
shared macros (descriptors, gates, checks).

Assemble with `./assemble-tests.sh` (nasm); the binaries are not checked in.

- `pm32.asm` - 32-bit code and stacks, page-granular limits, LSL/LAR,
  big expand-down segments, and the return to real mode ("unreal" mode).
- `gates32.asm` - ring 3 and back: 386 and 286 call gates (parameter
  copying), a 386 interrupt gate, IOPL and gate-privilege faults, data
  segments nulled on return.
- `task32.asm` - 386 task switches by CALL, JMP, IRET and a task gate,
  and a switch to a 286 TSS.
- `paging.asm` - page tables, accessed and dirty bits, #PF error codes and
  CR2, accesses across pages, user and supervisor protection.
- `tlb.asm` - the TLB: a stale translation until CR3 is reloaded, the
  dirty bit set on a write to a page cached by a read, and nothing
  cached for a page that faulted.
- `faults.asm` - the page-fault and contributory double-fault classes.
- `v86.asm` - virtual-8086 mode: entry, INT to protected mode, the I/O
  permission bitmap, IOPL-sensitive instructions.

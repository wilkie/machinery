import type { InstructionInfo } from '@machinery/core';

import { nop } from './instructions/nop';
import { halt } from './instructions/halt';
import { di } from './instructions/di';
import { ei } from './instructions/ei';
import { ld } from './instructions/ld';
import { add } from './instructions/add';
import { adc } from './instructions/adc';
import { sub } from './instructions/sub';
import { sbc } from './instructions/sbc';
import { and } from './instructions/and';
import { or } from './instructions/or';
import { xor } from './instructions/xor';
import { cp } from './instructions/cp';
import { inc } from './instructions/inc';
import { dec } from './instructions/dec';
import { jp } from './instructions/jp';
import { jr } from './instructions/jr';
import { djnz } from './instructions/djnz';
import { call } from './instructions/call';
import { ret, reti, retn } from './instructions/ret';
import { push } from './instructions/push';
import { pop } from './instructions/pop';
import { rst } from './instructions/rst';
import { ex_af, exx } from './instructions/ex';
import { rlca, rrca, rla, rra, rrd, rld } from './instructions/rotate';
import { daa, cpl, neg, scf, ccf, im } from './instructions/misc';
import { in_, out } from './instructions/io';
import {
  ldi,
  ldd,
  ldir,
  lddr,
  cpi,
  cpd,
  cpir,
  cpdr,
  ini,
  ind,
  inir,
  indr,
  outi,
  outd,
  otir,
  otdr,
} from './instructions/block';
import {
  rlc,
  rrc,
  rl,
  rr,
  sla,
  sra,
  srl,
  bit,
  res,
  set,
} from './instructions/bit';

export const instructions: InstructionInfo[] = [
  // Control
  nop,
  halt,
  di,
  ei,

  // Load
  ld,

  // Arithmetic
  add,
  adc,
  sub,
  sbc,
  inc,
  dec,

  // Logic
  and,
  or,
  xor,
  cp,

  // Jumps
  jp,
  jr,
  djnz,
  call,
  ret,
  reti,
  retn,

  // Stack
  push,
  pop,
  rst,

  // Exchange
  ex_af,
  exx,

  // Rotate/shift (unprefixed accumulator forms)
  rlca,
  rrca,
  rla,
  rra,
  rrd,
  rld,

  // Misc
  daa,
  cpl,
  neg,
  scf,
  ccf,
  im,

  // I/O
  in_,
  out,

  // Block operations
  ldi,
  ldd,
  ldir,
  lddr,
  cpi,
  cpd,
  cpir,
  cpdr,
  ini,
  ind,
  inir,
  indr,
  outi,
  outd,
  otir,
  otdr,

  // CB-prefix: rotate/shift/bit operations
  rlc,
  rrc,
  rl,
  rr,
  sla,
  sra,
  srl,
  bit,
  res,
  set,
];

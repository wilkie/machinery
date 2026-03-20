import type { InstructionInfo } from '@machinery/core';

import { aaa } from './instructions/aaa';
import { aad } from './instructions/aad';
import { aam } from './instructions/aam';
import { aas } from './instructions/aas';
import { adc } from './instructions/adc';
import { add } from './instructions/add';
import { and } from './instructions/and';
import { arpl } from './instructions/arpl';
import { bound } from './instructions/bound';
import { call } from './instructions/call';
import { cbw } from './instructions/cbw';
import { clc } from './instructions/clc';
import { cld } from './instructions/cld';
import { cli } from './instructions/cli';
import { clts } from './instructions/clts';
import { cmc } from './instructions/cmc';
import { cmp } from './instructions/cmp';
import { cmps } from './instructions/cmps';
import { cwd } from './instructions/cwd';
import { daa } from './instructions/daa';
import { das } from './instructions/das';
import { dec } from './instructions/dec';
import { div } from './instructions/div';
import { enter } from './instructions/enter';
import { esc } from './instructions/esc';
import { hlt } from './instructions/hlt';
import { idiv } from './instructions/idiv';
import { imul } from './instructions/imul';
import { in_ } from './instructions/in';
import { inc } from './instructions/inc';
import { ins } from './instructions/ins';
import { int } from './instructions/int';
import { iret } from './instructions/iret';
import { ja } from './instructions/ja';
import { jae } from './instructions/jae';
import { jb } from './instructions/jb';
import { jbe } from './instructions/jbe';
import { jcxz } from './instructions/jcxz';
import { je } from './instructions/je';
import { jg } from './instructions/jg';
import { jge } from './instructions/jge';
import { jl } from './instructions/jl';
import { jle } from './instructions/jle';
import { jmp } from './instructions/jmp';
import { jne } from './instructions/jne';
import { jno } from './instructions/jno';
import { jnp } from './instructions/jnp';
import { jns } from './instructions/jns';
import { jo } from './instructions/jo';
import { jp } from './instructions/jp';
import { js } from './instructions/js';
import { lahf } from './instructions/lahf';
import { lar } from './instructions/lar';
import { lds } from './instructions/lds';
import { lea } from './instructions/lea';
import { leave } from './instructions/leave';
import { les } from './instructions/les';
import { lgdt } from './instructions/lgdt';
import { lidt } from './instructions/lidt';
import { lldt } from './instructions/lldt';
import { lmsw } from './instructions/lmsw';
import { lock } from './instructions/lock';
import { lods } from './instructions/lods';
import { loop } from './instructions/loop';
import { loope } from './instructions/loope';
import { loopne } from './instructions/loopne';
import { lsl } from './instructions/lsl';
import { ltr } from './instructions/ltr';
import { mov } from './instructions/mov';
import { movs } from './instructions/movs';
import { mul } from './instructions/mul';
import { neg } from './instructions/neg';
import { nop } from './instructions/nop';
import { not } from './instructions/not';
import { or } from './instructions/or';
import { out } from './instructions/out';
import { outs } from './instructions/outs';
import { pop } from './instructions/pop';
import { popa } from './instructions/popa';
import { popf } from './instructions/popf';
import { push } from './instructions/push';
import { pusha } from './instructions/pusha';
import { pushf } from './instructions/pushf';
import { rcl } from './instructions/rcl';
import { rcr } from './instructions/rcr';
import { rep } from './instructions/rep';
import { repne } from './instructions/repne';
import { ret } from './instructions/ret';
import { rol } from './instructions/rol';
import { ror } from './instructions/ror';
import { sahf } from './instructions/sahf';
import { sal } from './instructions/sal';
import { salc } from './instructions/salc';
import { sar } from './instructions/sar';
import { sbb } from './instructions/sbb';
import { scas } from './instructions/scas';
import { seg } from './instructions/seg';
import { sgdt } from './instructions/sgdt';
import { shr } from './instructions/shr';
import { sidt } from './instructions/sidt';
import { sldt } from './instructions/sldt';
import { smsw } from './instructions/smsw';
import { stc } from './instructions/stc';
import { std } from './instructions/std';
import { sti } from './instructions/sti';
import { stos } from './instructions/stos';
import { str } from './instructions/str';
import { sub } from './instructions/sub';
import { test } from './instructions/test';
import { verr } from './instructions/verr';
import { verw } from './instructions/verw';
import { wait } from './instructions/wait';
import { xchg } from './instructions/xchg';
import { xlat } from './instructions/xlat';
import { xor } from './instructions/xor';

export const instructions: InstructionInfo[] = [
  aaa,
  aad,
  aam,
  aas,
  adc,
  add,
  and,
  arpl,
  bound,
  call,
  cbw,
  clc,
  cld,
  cli,
  clts,
  cmc,
  cmp,
  cmps,
  cwd,
  daa,
  das,
  dec,
  div,
  enter,
  esc,
  hlt,
  idiv,
  imul,
  in_,
  inc,
  ins,
  int,
  iret,
  ja,
  jae,
  jb,
  jbe,
  jcxz,
  je,
  jg,
  jge,
  jl,
  jle,
  jmp,
  jne,
  jno,
  jnp,
  jns,
  jo,
  jp,
  js,
  lahf,
  lar,
  lds,
  lea,
  leave,
  les,
  lgdt,
  lidt,
  lldt,
  lmsw,
  lock,
  lods,
  loop,
  loope,
  loopne,
  lsl,
  ltr,
  mov,
  movs,
  mul,
  neg,
  nop,
  not,
  or,
  out,
  outs,
  pop,
  popa,
  popf,
  push,
  pusha,
  pushf,
  rcl,
  rcr,
  rol,
  ror,
  rep,
  repne,
  ret,
  sahf,
  sal,
  salc,
  sar,
  shr,
  sbb,
  scas,
  seg,
  sgdt,
  sidt,
  sldt,
  smsw,
  stc,
  std,
  sti,
  stos,
  str,
  sub,
  test,
  verr,
  verw,
  wait,
  xchg,
  xlat,
  xor,
];

export default instructions;

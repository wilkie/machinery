import type { InstructionInfo } from '@machinery/core';

import { Opcodes, CBOpcodes } from '../opcodes';

// Helper to generate all 8 register forms for a CB-prefix instruction
function cbRegForms(
  baseOpcode: number,
  mnemonic: string,
  operation: (reg: string) => string[],
  cycles: number,
  memCycles: number,
) {
  const regs = ['B', 'C', 'D', 'E', 'H', 'L', '(HL)', 'A'];
  return regs.map((reg, i) => ({
    opcode: [Opcodes.CB_PREFIX, baseOpcode + i],
    operands: [reg],
    operandSize: 8 as const,
    operation: reg === '(HL)' ? operation('RAM:u8[HL]') : operation(reg),
    cycles: reg === '(HL)' ? memCycles : cycles,
  }));
}

// Helper for BIT/SET/RES which take a bit number and register
function cbBitRegForms(
  baseOpcode: number,
  bit: number,
  operation: (bit: number, reg: string) => string[],
  cycles: number,
  memCycles: number,
) {
  const regs = ['B', 'C', 'D', 'E', 'H', 'L', '(HL)', 'A'];
  return regs.map((reg, i) => ({
    opcode: [Opcodes.CB_PREFIX, baseOpcode + i],
    operands: [bit.toString(), reg],
    operandSize: 8 as const,
    operation:
      reg === '(HL)' ? operation(bit, 'RAM:u8[HL]') : operation(bit, reg),
    cycles: reg === '(HL)' ? memCycles : cycles,
  }));
}

export const rlc: InstructionInfo = {
  identifier: 'rlc',
  name: 'Rotate Left Circular',
  description: 'Rotates the operand left. Bit 7 is copied to carry and bit 0.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.RLC_B,
    'rlc',
    (r) => [`CF = ${r} >> 7`, `${r} = (${r} << 1) | CF`],
    8,
    15,
  ),
};

export const rrc: InstructionInfo = {
  identifier: 'rrc',
  name: 'Rotate Right Circular',
  description: 'Rotates the operand right. Bit 0 is copied to carry and bit 7.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.RRC_B,
    'rrc',
    (r) => [`CF = ${r} & 1`, `${r} = (${r} >> 1) | (CF << 7)`],
    8,
    15,
  ),
};

export const rl: InstructionInfo = {
  identifier: 'rl',
  name: 'Rotate Left through Carry',
  description: 'Rotates the operand left through carry.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.RL_B,
    'rl',
    (r) => [`tmp = CF`, `CF = ${r} >> 7`, `${r} = (${r} << 1) | tmp`],
    8,
    15,
  ),
};

export const rr: InstructionInfo = {
  identifier: 'rr',
  name: 'Rotate Right through Carry',
  description: 'Rotates the operand right through carry.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.RR_B,
    'rr',
    (r) => [`tmp = CF`, `CF = ${r} & 1`, `${r} = (${r} >> 1) | (tmp << 7)`],
    8,
    15,
  ),
};

export const sla: InstructionInfo = {
  identifier: 'sla',
  name: 'Shift Left Arithmetic',
  description:
    'Shifts the operand left by one bit. Bit 0 becomes 0, bit 7 goes to carry.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.SLA_B,
    'sla',
    (r) => [`CF = ${r} >> 7`, `${r} = ${r} << 1`],
    8,
    15,
  ),
};

export const sra: InstructionInfo = {
  identifier: 'sra',
  name: 'Shift Right Arithmetic',
  description:
    'Shifts the operand right by one bit. Bit 7 is preserved (sign extension), bit 0 goes to carry.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.SRA_B,
    'sra',
    (r) => [`CF = ${r} & 1`, `${r} = (${r} & 0x80) | (${r} >> 1)`],
    8,
    15,
  ),
};

export const srl: InstructionInfo = {
  identifier: 'srl',
  name: 'Shift Right Logical',
  description:
    'Shifts the operand right by one bit. Bit 7 becomes 0, bit 0 goes to carry.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: cbRegForms(
    CBOpcodes.SRL_B,
    'srl',
    (r) => [`CF = ${r} & 1`, `${r} = ${r} >> 1`],
    8,
    15,
  ),
};

export const bit: InstructionInfo = {
  identifier: 'bit',
  name: 'Test Bit',
  description:
    'Tests the specified bit of the operand. Z flag is set if the bit is 0.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    ...cbBitRegForms(
      CBOpcodes.BIT_0_B,
      0,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_1_B,
      1,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_2_B,
      2,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_3_B,
      3,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_4_B,
      4,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_5_B,
      5,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_6_B,
      6,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
    ...cbBitRegForms(
      CBOpcodes.BIT_7_B,
      7,
      (b, r) => [`ZF = ~(${r} >> ${b}) & 1`],
      8,
      12,
    ),
  ],
};

export const res: InstructionInfo = {
  identifier: 'res',
  name: 'Reset Bit',
  description: 'Resets (clears to 0) the specified bit of the operand.',
  modifies: [],
  forms: [
    ...cbBitRegForms(
      CBOpcodes.RES_0_B,
      0,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_1_B,
      1,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_2_B,
      2,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_3_B,
      3,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_4_B,
      4,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_5_B,
      5,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_6_B,
      6,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.RES_7_B,
      7,
      (b, r) => [`${r} = ${r} & ~(1 << ${b})`],
      8,
      15,
    ),
  ],
};

export const set: InstructionInfo = {
  identifier: 'set',
  name: 'Set Bit',
  description: 'Sets (to 1) the specified bit of the operand.',
  modifies: [],
  forms: [
    ...cbBitRegForms(
      CBOpcodes.SET_0_B,
      0,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_1_B,
      1,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_2_B,
      2,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_3_B,
      3,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_4_B,
      4,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_5_B,
      5,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_6_B,
      6,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
    ...cbBitRegForms(
      CBOpcodes.SET_7_B,
      7,
      (b, r) => [`${r} = ${r} | (1 << ${b})`],
      8,
      15,
    ),
  ],
};

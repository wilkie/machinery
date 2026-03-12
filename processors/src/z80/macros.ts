export const macros = {
  // For FLAGS, instead of calculating the actual flags after every operation,
  // we define a set of internal state that indicates the operation type and
  // helps lazily compute the flags as they are needed.

  // flag_op encodes the operation type and operand width:
  //
  // flag_op: R | A1 | N | L | A | S | W | C
  //
  // R  - Resolved: when set, flag computation is skipped (flags already final)
  // A1 - AND: HF = 1 (for AND, which uniquely sets HF = 1 among logic ops)
  // N  - NOCF: CF is not modified by this operation (INC/DEC)
  // L  - Logic: CF = 0, PF = parity (not overflow)
  // A  - ALU: all flags set based on a, b, alu_result
  // S  - SUB: subtraction (NF = 1, carry/HF borrow logic)
  // W  - Word: 0 = 8-bit, 1 = 16-bit
  // C  - Carry in: carry value input for ADC/SBC

  FLAG_OP_RESOLVED: 0x100,
  FLAG_OP_AND: 0x80,
  FLAG_OP_NOCF: 0x40,
  FLAG_OP_LOGIC: 0x20,
  FLAG_OP_ALU: 0x10,
  FLAG_OP_SUB: 0x8,
  FLAG_OP_16BIT: 0x4,
  FLAG_OP_8BIT: 0x0,
  FLAG_OP_CARRY: 0x1,

  // Composite masks
  FLAG_OP_ALU_ADD8: 0x10, // ALU | 8BIT
  FLAG_OP_ALU_ADD16: 0x14, // ALU | 16BIT
  FLAG_OP_ALU_SUB8: 0x18, // ALU | SUB | 8BIT
  FLAG_OP_ALU_SUB16: 0x1c, // ALU | SUB | 16BIT
  FLAG_OP_ALU_INC8: 0x50, // ALU | NOCF | 8BIT
  FLAG_OP_ALU_DEC8: 0x58, // ALU | NOCF | SUB | 8BIT

  // Resolve all flags at once
  RESOLVE_FLAGS: [
    '${RESOLVE_CF}',
    '${RESOLVE_SF}',
    '${RESOLVE_ZF}',
    '${RESOLVE_HF}',
    '${RESOLVE_PF}',
    '${RESOLVE_NF}',
    'flag_op = ${FLAG_OP_RESOLVED}',
  ],

  // ZF: set if result is zero
  // 8-bit: (alu_result & 0xff) == 0
  // 16-bit: (alu_result & 0xffff) == 0
  RESOLVE_ZF: [
    'ZF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_16BIT}) == 0 ? ((alu_result & 0xff) == 0 ? 1 : 0) : ((alu_result & 0xffff) == 0 ? 1 : 0)) : ZF',
  ],

  // SF: set if result is negative (high bit set)
  // 8-bit: bit 7 of result
  // 16-bit: bit 15 of result
  RESOLVE_SF: [
    'SF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_16BIT}) == 0 ? ((alu_result & 0x80) >> 7) : ((alu_result & 0x8000) >> 15)) : SF',
  ],

  // CF: carry/borrow flag
  // Logic ops: CF = 0
  // NOCF: CF unchanged
  // ALU add: carry out of bit 7 (8-bit) or bit 15 (16-bit)
  // ALU sub: borrow (a < b + carry_in)
  RESOLVE_CF: [
    'CF = ((flag_op < ${FLAG_OP_RESOLVED}) && (flag_op & ${FLAG_OP_NOCF} == 0)) ? ((flag_op & ${FLAG_OP_LOGIC}) > 0 ? 0 : ((flag_op & ${FLAG_OP_SUB}) > 0 ? (a < (b + (flag_op & ${FLAG_OP_CARRY})) ? 1 : 0) : (((a & b) | ((a | b) & ~alu_result)) >> ((flag_op & ${FLAG_OP_16BIT}) == 0 ? 7 : 15)) & 0x1)) : CF',
  ],

  // HF: half-carry flag
  // Logic AND: HF = 1
  // Logic OR/XOR: HF = 0
  // ALU: carry out of bit 3 (8-bit) or bit 11 (16-bit)
  RESOLVE_HF: [
    'HF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_AND}) > 0 ? 1 : ((flag_op & ${FLAG_OP_LOGIC}) > 0 ? 0 : ((a ^ b ^ (alu_result & ((flag_op & ${FLAG_OP_16BIT}) == 0 ? 0xFF : 0xFFFF))) >> ((flag_op & ${FLAG_OP_16BIT}) == 0 ? 4 : 12)) & 0x1)) : HF',
  ],

  // PF: parity (logic) or overflow (arithmetic)
  // Logic ops: parity of low byte (even number of set bits = 1)
  // ALU ops: signed overflow
  //   8-bit: (a ^ alu_result) & (sub ? (a ^ b) : (alu_result ^ b)) & 0x80
  //   16-bit: same pattern with 0x8000
  RESOLVE_PF: [
    'PF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_LOGIC}) > 0 ? ROM.PARITY[alu_result & 0xff] : (((flag_op & ${FLAG_OP_16BIT}) == 0 ? 0x80 : 0x8000) & (a ^ alu_result) & (((flag_op & ${FLAG_OP_SUB}) > 0 ? a : alu_result) ^ (b + (flag_op & ${FLAG_OP_CARRY})))) > 0 ? 1 : 0) : PF',
  ],

  // NF: subtract flag (set to 1 for subtract operations, 0 otherwise)
  RESOLVE_NF: [
    'NF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_SUB}) > 0 ? 1 : 0) : NF',
  ],
};

export default macros;

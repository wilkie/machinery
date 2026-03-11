export const macros = {
  // Mod/RM byte expansion helper macros
  MOD_RM_SEGMENT:
    '(DATA_SEG_BASE == 0xffff ? ${[DS_BASE,DS_BASE1,SS_BASE,SS_BASE1,DS_BASE2,DS_BASE3,SS_BASE2,DS_BASE4][%{rm}]} : DATA_SEG_BASE)',
  // [BX,BX,BP,BP,x,x,BP,BX][rm] -> [BX,BP][ is rm in [0, 1, 7] or not? ]
  MOD_RM_BASE:
    '((%{rm} & 0x6) == 0x4) ? 0 : ${[BX,BP][(((%{rm} + 1) & 7) + 5) >> 3]}',
  // [SI,DI,SI,DI,SI,DI,0,0][rm] -> [SI,DI][rm & 0x1]
  MOD_RM_INDEX: '((%{rm} & 0x6) == 0x6) ? 0 : ${[SI,DI][%{rm} & 0x1]}',
  MOD_RM_OFFSET: '(${MOD_RM_BASE}) + (${MOD_RM_INDEX})',
  //MOD_RM_OFFSET: "${MOD_RM_BX_BASE} + ${[SI,DI,SI,DI,SI,DI,0,0][rm]}",
  MOD_RM_RM8: '${[AL,CL,DL,BL,AH,CH,DH,BH][%{rm}]}',
  MOD_RM_RM16: '${[AX,CX,DX,BX,SP,BP,SI,DI][%{rm}]}',
  MOD_RM_RMS16: '${[ES,CS,SS,DS,FS,GS,HS,IS][%{rm}]}',
  MOD_RM_REG8: '${[AL,CL,DL,BL,AH,CH,DH,BH][%{reg}]}',
  MOD_RM_REG16: '${[AX,CX,DX,BX,SP,BP,SI,DI][%{reg}]}',
  MOD_RM_REGS16: '${[ES,CS,SS,DS,FS,GS,HS,IS][%{seg}]}',

  // For FLAGS, instead of calculating the actual flags after every operation,
  // we instead define a set of internal flags that indicate the operation and
  // help lazily compute the flags as they are needed

  // flag_op: R | G | HD | O | X | F | L | A | S | BB | C

  // R - When the flags are resolved, flag computation does not happen
  // G - Signed operation (treat 'a' as signed)
  // H - Shift
  // D - Shift Direction (0 = Left, 1 = Right)
  // O - OF left alone (OF = OF)
  // X - AF left alone (AF = AF)
  // F - CF left alone (CF = CF)
  // L - Logic (CF = 0, OF = 0)
  // A - ALU (All flags set based on operation)
  // S - SUB/SBB
  // BB - operand word size (00: 8, 01: 16, etc)
  // C - C_in (carry value input)

  // FLAGS is set to the appropriate values already
  FLAG_OP_RESOLVED: 0x8000,
  FLAG_OP_SIGNED: 0x800,
  FLAG_OP_SHIFT: 0x400,
  FLAG_OP_RIGHT: 0x200,
  FLAG_OP_NOOF: 0x100,
  FLAG_OP_NOAF: 0x80,
  FLAG_OP_NOCF: 0x40,
  FLAG_OP_LOGIC: 0x20,
  FLAG_OP_ALU: 0x10,
  FLAG_OP_SUB: 0x8,
  FLAG_OP_BITS: 0x6, // 0b110
  FLAG_OP_CARRY: 0x1,

  // Masks for different shifts
  FLAG_OP_SHIFT_LEFT: 0x400, // x10xxxxx_xxxxxxxx
  FLAG_OP_SHIFT_RIGHT: 0x600, // x11xxxxx_xxxxxxxx

  // Masks for different bit-widths
  FLAG_OP_8BIT: 0x0, // x00x
  FLAG_OP_16BIT: 0x2, // x01x

  // We defer resolution of most flags to speed up the emulation since they
  // are often rarely used. This macro expands to a set of microcode that will
  // resolve the flags to their final values.
  RESOLVE_FLAGS: [
    '${RESOLVE_CF}',
    '${RESOLVE_SF}',
    '${RESOLVE_OF}',
    '${RESOLVE_PF}',
    '${RESOLVE_AF}',
    '${RESOLVE_ZF}',
    'flag_op = ${FLAG_OP_RESOLVED}',
    'CF = CARRY',
  ],

  RESOLVE_ZF: [
    'ZF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_BITS}) == 0x0 ? ((alu_result & 0xff) == 0x0 ? 1 : 0) : ((alu_result & 0xffff) == 0x0 ? 1 : 0)) : ZF',
  ],

  // Look at the parity data and match it against the alu_result
  RESOLVE_PF: [
    'PF = flag_op < ${FLAG_OP_RESOLVED} ? ROM.PARITY[alu_result & 0xff] : PF',
  ],

  RESOLVE_SF: [
    'SF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_BITS}) == 0x0 ? ((alu_result & 0x80) > 0 ? 1 : 0) : ((alu_result & 0x8000) > 0 ? 1 : 0)) : SF',
  ],

  RESOLVE_OF: [
    //"OF = flag_op < ${FLAG_OP_RESOLVED} ? ((flag_op & ${FLAG_OP_LOGIC}) > 0 ? 0 : ((flag_op & ${FLAG_OP_BITS}) == 0x0 ? ((a ^ alu_result) & (a ^ b) & 0x80) >> 7 : ((a ^ alu_result) & (a ^ b) & 0x8000) >> 15)) : OF",
    'OF = ((flag_op < ${FLAG_OP_RESOLVED}) && (flag_op & ${FLAG_OP_NOOF} == 0)) ? ((flag_op & ${FLAG_OP_LOGIC}) > 0 ? 0 : ((flag_op & ${FLAG_OP_SHIFT}) > 0 ? (((((((flag_op & ${FLAG_OP_RIGHT}) > 0 ? alu_result : a) >> (((((flag_op & ${FLAG_OP_BITS}) >> 1) + 1) * 8) - 2)) - 1) & 0x3) < 2) ? 1 : 0) : (((flag_op & ${FLAG_OP_BITS}) == 0x0 ? 0x80 : 0x8000) & (a ^ alu_result) & (((flag_op & ${FLAG_OP_SUB}) > 0 ? a : alu_result) ^ (b + (flag_op & ${FLAG_OP_CARRY})))) > 0 ? 1 : 0)) : OF',
  ],

  // Depending on the operation, understand the carry flag
  RESOLVE_CF: [
    // CARRY = CARRY (if flags are resolved or NOCF bit is set)
    // CARRY = 0 (if LOGIC bit is set)
    // CARRY = (((a & b) | ((a | b) & ~alu_result)) >> 7) & 0x1 if non-sub 8-bit
    // CARRY = (((a & b) | ((a | b) & ~alu_result)) >> 15) & 0x1 if non-sub 8-bit
    // CARRY = (a >> (8 - b)) & 0x1 if 8-bit shift left
    // CARRY = (a >> (16 - b)) & 0x1 if 16-bit shift left
    // CARRY = (a >> (b - 1)) & 0x1 if any shift right
    // CARRY = a < (b + C_in) if sub
    'CARRY = ((flag_op < ${FLAG_OP_RESOLVED}) && (flag_op & ${FLAG_OP_NOCF} == 0)) ? ((flag_op & ${FLAG_OP_LOGIC}) > 0 ? 0 : ((flag_op & ${FLAG_OP_SHIFT}) > 0 ? (((flag_op & ${FLAG_OP_SIGNED}) == 0 ? a : a:i16) >> ((flag_op & ${FLAG_OP_RIGHT}) == 0 ? ((((flag_op & ${FLAG_OP_BITS}) == 0 ? 8 : 16) - b)) : (b - 1))) & 0x1 : ((flag_op & ${FLAG_OP_SUB}) > 0 ? (a < (b + (flag_op & ${FLAG_OP_CARRY})) ? 1 : 0) : (((a & b) | ((a | b) & ~alu_result)) >> ((flag_op & ${FLAG_OP_BITS}) == 0x0 ? 7 : 15)) & 0x1))) : CARRY',
  ],

  // Depending on the operation, understand the auxiliary carry flag
  RESOLVE_AF: [
    'AF = ((flag_op < ${FLAG_OP_RESOLVED}) && (flag_op & ${FLAG_OP_NOAF} == 0)) ? ((a ^ b ^ alu_result) & 0x10) >> 4 : AF',
  ],

  // Raises exceptions
  UD_EXCEPTION: ['#6'],
};

export default macros;

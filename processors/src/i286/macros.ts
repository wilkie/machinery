export const macros = {
  // Mod/RM byte expansion helper macros
  MOD_RM_SEGMENT:
    '(DATA_SEG_BASE == 0xffff ? ${[DS_BASE,DS_BASE1,SS_BASE,SS_BASE1,DS_BASE2,DS_BASE3,SS_BASE2,DS_BASE4][%{rm}]} : DATA_SEG_BASE)',
  MOD_RM_SEGMENT_LIMIT_MIN:
    '(DATA_SEG_BASE == 0xffff ? ${[DS_LIMIT_MIN,DS_LIMIT_MIN1,SS_LIMIT_MIN,SS_LIMIT_MIN1,DS_LIMIT_MIN2,DS_LIMIT_MIN3,SS_LIMIT_MIN2,DS_LIMIT_MIN4][%{rm}]} : DATA_SEG_LIMIT_MIN)',
  MOD_RM_SEGMENT_LIMIT_MAX:
    '(DATA_SEG_BASE == 0xffff ? ${[DS_LIMIT_MAX,DS_LIMIT_MAX1,SS_LIMIT_MAX,SS_LIMIT_MAX1,DS_LIMIT_MAX2,DS_LIMIT_MAX3,SS_LIMIT_MAX2,DS_LIMIT_MAX4][%{rm}]} : DATA_SEG_LIMIT_MAX)',
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
  FLAG_OP_DIV: 0x1000,
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
    `ZF = flag_op < $\{FLAG_OP_RESOLVED}
      ? ((flag_op & $\{FLAG_OP_BITS}) == 0x0
        ? ((alu_result & 0xff) == 0x0 ? 1 : 0)
        : ((alu_result & 0xffff) == 0x0 ? 1 : 0))
      : ZF`,
  ],

  // Look at the parity data and match it against the alu_result
  RESOLVE_PF: [
    `PF = flag_op < $\{FLAG_OP_RESOLVED}
      ? ROM.PARITY[alu_result & 0xff]
      : PF`,
  ],

  RESOLVE_SF: [
    `SF = flag_op < $\{FLAG_OP_RESOLVED}
      ? ((flag_op & $\{FLAG_OP_BITS}) == 0x0
        ? ((alu_result & 0x80) > 0 ? 1 : 0)
        : ((alu_result & 0x8000) > 0 ? 1 : 0))
      : SF`,
  ],

  RESOLVE_OF: [
    `OF = ((flag_op < $\{FLAG_OP_RESOLVED}) && (flag_op & $\{FLAG_OP_NOOF} == 0))
      ? ((flag_op & $\{FLAG_OP_LOGIC}) > 0
        ? 0
        : ((flag_op & $\{FLAG_OP_DIV}) > 0
          ? CARRY
          : ((flag_op & $\{FLAG_OP_SHIFT}) > 0
            ? ((flag_op & $\{FLAG_OP_RIGHT}) > 0
              ? (((((flag_op & $\{FLAG_OP_BITS}) == 0x0 ? 0x80 : 0x8000) & (((flag_op & $\{FLAG_OP_SIGNED}) == 0 ? a >>> (b - 1) : ((flag_op & $\{FLAG_OP_BITS}) == 0 ? a:i8 : a:i16) >> (b - 1)) ^ alu_result)) > 0) ? 1 : 0)
              : ((((flag_op & $\{FLAG_OP_BITS}) == 0x0 ? 0x80 : 0x8000) & ((a << (b - 1)) ^ alu_result)) > 0 ? 1 : 0))
            : (((flag_op & $\{FLAG_OP_BITS}) == 0x0 ? 0x80 : 0x8000) & (a ^ alu_result) & (((flag_op & $\{FLAG_OP_SUB}) > 0 ? a : alu_result) ^ b)) > 0 ? 1 : 0)))
      : OF`,
  ],

  // Depending on the operation, understand the carry flag
  RESOLVE_CF: [
    // CARRY = CARRY (if flags are resolved or NOCF bit is set)
    // CARRY = 0 (if LOGIC bit is set)
    // CARRY = (((AX & 1) ? ((AX & ~1) + (b << 8)) & 0xffff : AX) >> 8) < b (div8: undo the last nonrestoring division operation and re-perform the last compare)
    // CARRY = (((AX & 1) ? (((AX & ~1) | (DX << 16)) + (b << 16)) & 0xffffffff : AX | (DX << 16)) >> 16) < b (div16: undo the last nonrestoring division operation and re-perform the last compare)
    // CARRY = (((a & b) | ((a | b) & ~alu_result)) >> 7) & 0x1 if non-sub 8-bit
    // CARRY = (((a & b) | ((a | b) & ~alu_result)) >> 15) & 0x1 if non-sub 8-bit
    // CARRY = (a >> (8 - b)) & 0x1 if 8-bit shift left
    // CARRY = (a >> (16 - b)) & 0x1 if 16-bit shift left
    // CARRY = (a >> (b - 1)) & 0x1 if any shift right
    // CARRY = a < (b + C_in) if sub
    `CARRY = ((flag_op < $\{FLAG_OP_RESOLVED}) && (flag_op & $\{FLAG_OP_NOCF} == 0))
      ? ((flag_op & $\{FLAG_OP_LOGIC}) > 0
        ? 0
        : ((flag_op & $\{FLAG_OP_DIV}) > 0
          ? ((flag_op & $\{FLAG_OP_SIGNED}) > 0
            ? ((flag_op & $\{FLAG_OP_BITS}) == 0
              ? (alu_result:i8 < b:i8 ? 1 : 0)
              : (alu_result:i16 < b:i16 ? 1 : 0))
            : ((flag_op & $\{FLAG_OP_BITS}) == 0
              ? (((((AX & 1) > 0) ? ((AX & ~1) + (b << 8)):u16 >> 8 : AH) < b) ? 1 : 0)
              : (((((AX & 1) > 0) ? (((AX & ~1) | (DX:u32 << 16)) + (b:u32 << 16)):u32 >> 16 : DX:u32) < b) ? 1 : 0)))
          : ((flag_op & $\{FLAG_OP_SHIFT}) > 0
            ? (((flag_op & $\{FLAG_OP_SIGNED}) == 0 ? a : ((flag_op & $\{FLAG_OP_BITS}) == 0 ? a:i8 : a:i16)) >> ((flag_op & $\{FLAG_OP_RIGHT}) == 0 ? ((((flag_op & $\{FLAG_OP_BITS}) == 0 ? 8 : 16) - b)) : (b - 1))) & 0x1
            : ((flag_op & $\{FLAG_OP_SUB}) > 0
              ? (a < (b + (flag_op & $\{FLAG_OP_CARRY})) ? 1 : 0)
              : (((a & b) | ((a | b) & ~alu_result)) >> ((flag_op & $\{FLAG_OP_BITS}) == 0x0 ? 7 : 15)) & 0x1))))
      : CARRY`,
  ],

  // Depending on the operation, understand the auxiliary carry flag
  // Logic ops (OR/AND/XOR/TEST) clear AF; ALU ops compute from operands.
  RESOLVE_AF: [
    `AF = ((flag_op < $\{FLAG_OP_RESOLVED}) && (flag_op & $\{FLAG_OP_NOAF} == 0))
      ? ((flag_op & $\{FLAG_OP_LOGIC}) > 0
        ? 0
        : ((flag_op & $\{FLAG_OP_SHIFT}) > 0
          ? ((flag_op & $\{FLAG_OP_RIGHT}) > 0
            ? 1
            : (((a << (b - 1)) & 0x8) != 0 ? 1 : 0))
          : ((a ^ b ^ alu_result) & 0x10) >> 4))
      : AF`,
  ],

  SEGMENT_LIMIT_CHECK_REAL: ['#GP if offset == 0xffff'],

  SEGMENT_LIMIT_CHECK_PROTECTED8: [
    '#GP if offset < ${MOD_RM_SEGMENT_LIMIT_MIN}',
    '#GP if offset > ${MOD_RM_SEGMENT_LIMIT_MAX}',
  ],

  SEGMENT_LIMIT_CHECK_PROTECTED16: [
    '#GP if (offset + 1) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
    '#GP if (offset + 1) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
  ],

  SEGMENT_LIMIT_CHECK_PROTECTED32: [
    '#GP if (offset + 3) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
    '#GP if (offset + 3) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
  ],

  // Resolves a descriptor from a selector in 'tmp' (no exceptions).
  // Expects locals: index, desc_type, desc_s, desc_a, desc_dpl, desc_p, desc_limit, desc_base, desc_valid.
  // Sets desc_valid=1 if the descriptor was found, 0 otherwise.
  RESOLVE_DESCRIPTOR: [
    'desc_valid = 0',
    'index = tmp >> 3',
    ';; GDT lookup (TI=0)',
    'if (tmp & 0x0004) == 0 && index != 0',
    [
      'if (index * 8) <= GDTR.limit',
      [
        'desc_type = RAM.GDT.gates[index].SD.type',
        'desc_s = RAM.GDT.gates[index].SD.S',
        'desc_a = RAM.GDT.gates[index].SD.A',
        'desc_dpl = RAM.GDT.gates[index].SD.DPL',
        'desc_p = RAM.GDT.gates[index].SD.P',
        'desc_limit = RAM.GDT.gates[index].SD.limit',
        'desc_base = RAM.GDT.gates[index].SD.base',
        'desc_valid = 1',
      ],
      'end if',
    ],
    'end if',
    ';; LDT lookup (TI=1)',
    'if (tmp & 0x0004) != 0 && index != 0',
    [
      'if (index * 8) <= LDTR.limit',
      [
        'desc_type = RAM.LDT.gates[index].SD.type',
        'desc_s = RAM.LDT.gates[index].SD.S',
        'desc_a = RAM.LDT.gates[index].SD.A',
        'desc_dpl = RAM.LDT.gates[index].SD.DPL',
        'desc_p = RAM.LDT.gates[index].SD.P',
        'desc_limit = RAM.LDT.gates[index].SD.limit',
        'desc_base = RAM.LDT.gates[index].SD.base',
        'desc_valid = 1',
      ],
      'end if',
    ],
    'end if',
  ],

  // Loads descriptor fields from GDT or LDT based on TI bit in 'tmp'.
  // Expects locals: tmp (selector), index (already set to tmp >> 3),
  //   desc_type, desc_s, desc_dpl, desc_p, desc_limit, desc_base.
  // Does NOT check bounds or null — caller must do that.
  LOAD_DESCRIPTOR_FIELDS: [
    'if (tmp & 0x0004) == 0',
    [
      'desc_type = RAM.GDT.gates[index].SD.type',
      'desc_s = RAM.GDT.gates[index].SD.S',
      'desc_dpl = RAM.GDT.gates[index].SD.DPL',
      'desc_p = RAM.GDT.gates[index].SD.P',
      'desc_limit = RAM.GDT.gates[index].SD.limit',
      'desc_base = RAM.GDT.gates[index].SD.base',
    ],
    'end if',
    'if (tmp & 0x0004) != 0',
    [
      'desc_type = RAM.LDT.gates[index].SD.type',
      'desc_s = RAM.LDT.gates[index].SD.S',
      'desc_dpl = RAM.LDT.gates[index].SD.DPL',
      'desc_p = RAM.LDT.gates[index].SD.P',
      'desc_limit = RAM.LDT.gates[index].SD.limit',
      'desc_base = RAM.LDT.gates[index].SD.base',
    ],
    'end if',
  ],

  // Sets the accessed bit on the descriptor resolved by RESOLVE_DESCRIPTOR.
  // Expects locals: tmp (selector), index.
  SET_DESCRIPTOR_ACCESSED: [
    'if (tmp & 0x0004) == 0',
    ['RAM.GDT.gates[index].SD.A = 1'],
    'end if',
    'if (tmp & 0x0004) != 0',
    ['RAM.LDT.gates[index].SD.A = 1'],
    'end if',
  ],

  // Checks that a selector in 'tmp' is within descriptor table bounds.
  // Expects locals: index. Raises #GP with ERROR_CODE = tmp if out of bounds.
  DESCRIPTOR_BOUNDS_CHECK: [
    'if (tmp & 0x0004) == 0',
    ['#GP if (index * 8) > GDTR.limit'],
    'end if',
    'if (tmp & 0x0004) != 0',
    ['#GP if (index * 8) > LDTR.limit'],
    'end if',
  ],

  // Loads the A bit from a descriptor (GDT or LDT based on TI bit in 'tmp').
  // Expects locals: tmp (selector), index, desc_a.
  LOAD_DESCRIPTOR_A: [
    'if (tmp & 0x0004) == 0',
    ['desc_a = RAM.GDT.gates[index].SD.A'],
    'end if',
    'if (tmp & 0x0004) != 0',
    ['desc_a = RAM.LDT.gates[index].SD.A'],
    'end if',
  ],

  // Loads call gate descriptor fields (CGD overlay) from GDT or LDT.
  // Expects locals: tmp (selector), index, gate_target_sel, gate_target_off, gate_word_count.
  LOAD_CALL_GATE_FIELDS: [
    'if (tmp & 0x0004) == 0',
    [
      'gate_target_sel = RAM.GDT.gates[index].CGD.selector',
      'gate_target_off = RAM.GDT.gates[index].CGD.offset',
      'gate_word_count = RAM.GDT.gates[index].CGD.word_count',
    ],
    'end if',
    'if (tmp & 0x0004) != 0',
    [
      'gate_target_sel = RAM.LDT.gates[index].CGD.selector',
      'gate_target_off = RAM.LDT.gates[index].CGD.offset',
      'gate_word_count = RAM.LDT.gates[index].CGD.word_count',
    ],
    'end if',
  ],

  // Reads new SS:SP from the TSS based on new_cpl (target privilege level).
  // Expects locals: new_cpl, new_ss, new_sp.
  CALL_GATE_READ_TSS_STACK: [
    'if new_cpl == 0',
    ['new_sp = RAM.TSS.SP0', 'new_ss = RAM.TSS.SS0'],
    'end if',
    'if new_cpl == 1',
    ['new_sp = RAM.TSS.SP1', 'new_ss = RAM.TSS.SS1'],
    'end if',
    'if new_cpl == 2',
    ['new_sp = RAM.TSS.SP2', 'new_ss = RAM.TSS.SS2'],
    'end if',
  ],

  // Copies up to 31 parameter words from old stack to new stack frame.
  // Expects locals: gate_word_count, stack_address, old_stack_addr.
  // Parameters are written at stack_address + 4 + 2*i from old_stack_addr + 2*i.
  CALL_GATE_COPY_PARAMS: Array.from({ length: 31 }, (_, i) => [
    `if gate_word_count > ${i}`,
    [
      `RAM:u16[stack_address + ${4 + i * 2}] = RAM:u16[old_stack_addr + ${i * 2}]`,
    ],
    'end if',
  ]).flat(),

  // Loads TSS selector from a task gate descriptor (TGD overlay) in GDT or LDT.
  // Expects locals: tmp (selector), index, tss_sel.
  LOAD_TASK_GATE_FIELDS: [
    'if (tmp & 0x0004) == 0',
    ['tss_sel = RAM.GDT.gates[index].TGD.selector'],
    'end if',
    'if (tmp & 0x0004) != 0',
    ['tss_sel = RAM.LDT.gates[index].TGD.selector'],
    'end if',
  ],

  // Hardware task switch macro.
  // Expects: tss_sel = new TSS selector (validated as available TSS, present in GDT)
  //          tss_index = GDT index of new TSS
  //          task_nesting = 1 for CALL/INT (nested), 0 for JMP (not nested)
  //          desc_base = base address of new TSS (from GDT descriptor)
  //          desc_limit = limit of new TSS (from GDT descriptor)
  // Expects locals: old_tr_sel, old_tss_index, ldtr_sel, ldtr_index,
  //   plus all registers are available for save/load.
  TASK_SWITCH: [
    '${RESOLVE_FLAGS}',
    ';; Save current task state to current TSS',
    'RAM.TSS.IP = IP',
    'RAM.TSS.FLAGS = FLAGS',
    'RAM.TSS.AX = AX',
    'RAM.TSS.CX = CX',
    'RAM.TSS.DX = DX',
    'RAM.TSS.BX = BX',
    'RAM.TSS.SP = SP',
    'RAM.TSS.BP = BP',
    'RAM.TSS.SI = SI',
    'RAM.TSS.DI = DI',
    'RAM.TSS.ES = ES',
    'RAM.TSS.CS = CS',
    'RAM.TSS.SS = SS',
    'RAM.TSS.DS = DS',
    ';; Record old TSS selector before switching TR',
    'old_tr_sel = TR.selector',
    'old_tss_index = old_tr_sel >> 3',
    ';; Mark old TSS as not busy for JMP (type = 000)',
    'if task_nesting == 0',
    ['RAM.GDT.gates[old_tss_index].SD.type = 0b000'],
    'end if',
    ';; Switch TR to new TSS',
    'TR.selector = tss_sel',
    'TR.base = desc_base',
    'TR.limit = desc_limit',
    ';; Mark new TSS as busy (type = 001)',
    'RAM.GDT.gates[tss_index].SD.type = 0b001',
    ';; For CALL/INT: set back_link in new TSS to old TSS selector',
    'if task_nesting == 1',
    ['RAM.TSS.back_link = old_tr_sel'],
    'end if',
    ';; Load LDTR from new TSS (must be done before segment loads that may use LDT)',
    'ldtr_sel = RAM.TSS.LDTR',
    'ldtr_index = ldtr_sel >> 3',
    'LDTR.selector = ldtr_sel',
    'if ldtr_index == 0',
    ['LDTR.base = 0', 'LDTR.limit = 0'],
    'end if',
    'if ldtr_index != 0',
    [
      'ERROR_CODE = ldtr_sel',
      '#TS if (ldtr_index * 8) > GDTR.limit',
      ';; Must be an LDT descriptor (S=0, A=0, type=001)',
      '#TS if RAM.GDT.gates[ldtr_index].SD.S != 0',
      '#TS if RAM.GDT.gates[ldtr_index].SD.A != 0',
      '#TS if RAM.GDT.gates[ldtr_index].SD.type != 0b001',
      '#NP if RAM.GDT.gates[ldtr_index].SD.P != 1',
      'LDTR.base = RAM.GDT.gates[ldtr_index].SD.base',
      'LDTR.limit = RAM.GDT.gates[ldtr_index].SD.limit',
    ],
    'end if',
    ';; Load general registers from new TSS',
    'AX = RAM.TSS.AX',
    'CX = RAM.TSS.CX',
    'DX = RAM.TSS.DX',
    'BX = RAM.TSS.BX',
    'BP = RAM.TSS.BP',
    'SI = RAM.TSS.SI',
    'DI = RAM.TSS.DI',
    ';; Load segment registers (triggers validation via set operations)',
    'ES = RAM.TSS.ES',
    'CS = RAM.TSS.CS',
    'SS = RAM.TSS.SS',
    'DS = RAM.TSS.DS',
    'SP = RAM.TSS.SP',
    ';; Load FLAGS (full replacement including IOPL)',
    'FLAGS = RAM.TSS.FLAGS | 0b10',
    ';; sync lazy flag state with restored FLAGS',
    'flag_op = ${FLAG_OP_RESOLVED}',
    'CARRY = CF',
    ';; For CALL/INT: set NT in new task',
    'if task_nesting == 1',
    ['NT = 1'],
    'end if',
    ';; Load IP (uses new CS_BASE from CS set)',
    'IP = RAM.TSS.IP',
    ';; IP must be in code segment limit',
    'ERROR_CODE = 0',
    '#GP if IP > CS_LIMIT_MAX',
  ],

  // Raises exceptions (uses conditional form so _ip_save rollback occurs)
  UD_EXCEPTION: ['#UD if 1 == 1'],
};

export default macros;

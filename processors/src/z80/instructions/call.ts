import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const call: InstructionInfo = {
  identifier: 'call',
  name: 'Call Subroutine',
  description:
    'Pushes the return address onto the stack and jumps to the given address.',
  modifies: [],
  macros: {
    CALL_OP: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = %{imm}'],
  },
  forms: [
    // CALL nn — unconditional
    {
      opcode: [Opcodes.CALL_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      addressing: 'absolute',
      operation: ['${CALL_OP}'],
      cycles: 17,
    },
    // CALL cc, nn — conditional
    {
      opcode: [Opcodes.CALL_NZ_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call nz'],
      addressing: 'absolute',
      operation: ['${RESOLVE_ZF}', 'if ZF == 0', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_Z_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call z'],
      addressing: 'absolute',
      operation: ['${RESOLVE_ZF}', 'if ZF == 1', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_NC_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call nc'],
      addressing: 'absolute',
      operation: ['${RESOLVE_CF}', 'if CF == 0', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_C_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call c'],
      addressing: 'absolute',
      operation: ['${RESOLVE_CF}', 'if CF == 1', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_PO_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call po'],
      addressing: 'absolute',
      operation: ['${RESOLVE_PF}', 'if PF == 0', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_PE_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call pe'],
      addressing: 'absolute',
      operation: ['${RESOLVE_PF}', 'if PF == 1', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_P_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call p'],
      addressing: 'absolute',
      operation: ['${RESOLVE_SF}', 'if SF == 0', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
    {
      opcode: [Opcodes.CALL_M_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['call m'],
      addressing: 'absolute',
      operation: ['${RESOLVE_SF}', 'if SF == 1', '${CALL_OP}', 'end if'],
      cycles: 17,
    },
  ],
};

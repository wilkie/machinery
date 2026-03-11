import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';
import type { OpcodeMatcher } from '@machinery/core';

/**
 * Z80 operand matchers.
 *
 * The Z80 uses a very different encoding scheme from x86. There is no ModRM byte.
 * Instead, the opcode byte itself encodes register selections in regular bit positions:
 *   - Bits 2-0 (z): source register
 *   - Bits 5-3 (y): destination register or operation selector
 *   - Bits 7-6 (x): instruction group
 *
 * Register encoding for r[z] and r[y]:
 *   0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 6=(HL), 7=A
 *
 * Register pair encoding for rp[p] (bits 5-4):
 *   0=BC, 1=DE, 2=HL, 3=SP
 *
 * Register pair encoding for rp2[p] (bits 5-4, for PUSH/POP):
 *   0=BC, 1=DE, 2=HL, 3=AF
 *
 * Condition encoding for cc[y] (bits 5-3):
 *   0=NZ, 1=Z, 2=NC, 3=C, 4=PO, 5=PE, 6=P, 7=M
 */
export const operands: OpcodeMatcher[] = [
  // --- Immediates ---
  {
    identifier: 'IMM_u8',
    name: 'Unsigned 8-bit Immediate',
    type: InstructionDataTypes.Immediate,
    size: 8,
    fields: [{ identifier: 'imm', offset: 0, size: 8 }],
  },
  {
    identifier: 'IMM_i8',
    name: 'Signed 8-bit Immediate',
    type: InstructionDataTypes.Immediate,
    size: 8,
    signed: true,
    fields: [{ identifier: 'imm', offset: 0, size: 8, signed: true }],
  },
  {
    identifier: 'IMM_u16',
    name: 'Unsigned 16-bit Immediate',
    type: InstructionDataTypes.Immediate,
    size: 16,
    fields: [{ identifier: 'imm', offset: 0, size: 16 }],
  },
  {
    identifier: 'IMM_MEM_u16',
    name: 'Unsigned 16-bit Immediate Dereference',
    type: InstructionDataTypes.Immediate,
    size: 16,
    fields: [
      {
        identifier: 'mem',
        type: InstructionOperandTypes.Memory,
        offset: 0,
        size: 16,
      },
    ],
  },

  // --- Displacements ---
  {
    identifier: 'DISP_i8',
    name: 'Signed 8-bit Displacement',
    type: InstructionDataTypes.Displacement,
    size: 8,
    signed: true,
    fields: [{ identifier: 'DISP', offset: 0, size: 8, signed: true }],
  },

  // --- Relative jump offset ---
  {
    identifier: 'REL_i8',
    name: 'Relative 8-bit Offset',
    type: InstructionDataTypes.Immediate,
    size: 8,
    signed: true,
    fields: [{ identifier: 'rel', offset: 0, size: 8, signed: true }],
  },

  // --- Register selectors embedded in opcode byte ---

  // Source register (bits 2-0): used in LD r,r' / ALU A,r
  // For the (HL) entry: type=Memory means the decoder treats it as memory indirect
  {
    identifier: 'REG8_SRC',
    name: 'Source Register (bits 2-0)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'src',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
        encoding: ['B', 'C', 'D', 'E', 'H', 'L', null, 'A'],
      },
      // bits 3-7 are don't-care for this matcher (matched by the opcode)
    ],
  },

  // Source register including (HL) as memory operand
  {
    identifier: 'REG8_SRC_OR_MEM',
    name: 'Source Register or (HL) (bits 2-0)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'src',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
        encoding: ['B', 'C', 'D', 'E', 'H', 'L', null, 'A'],
      },
    ],
  },

  // Destination register (bits 5-3): used in LD r,r' / ALU results
  {
    identifier: 'REG8_DST',
    name: 'Destination Register (bits 5-3)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'dst',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
        encoding: ['B', 'C', 'D', 'E', 'H', 'L', null, 'A'],
      },
    ],
  },

  // Register pair (bits 5-4): BC, DE, HL, SP
  {
    identifier: 'REG16_RP',
    name: 'Register Pair (bits 5-4)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rp',
        offset: 4,
        size: 2,
        type: InstructionOperandTypes.Register,
        encoding: ['BC', 'DE', 'HL', 'SP'],
      },
    ],
  },

  // Register pair for PUSH/POP (bits 5-4): BC, DE, HL, AF
  {
    identifier: 'REG16_RP2',
    name: 'Register Pair for Stack (bits 5-4)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rp',
        offset: 4,
        size: 2,
        type: InstructionOperandTypes.Register,
        encoding: ['BC', 'DE', 'HL', 'AF'],
      },
    ],
  },

  // Restart vector (bits 5-3): encodes the target address / 8
  {
    identifier: 'RST_VEC',
    name: 'Restart Vector (bits 5-3)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'vec',
        offset: 3,
        size: 3,
      },
    ],
  },

  // Bit number (bits 5-3) for CB-prefix BIT/SET/RES
  {
    identifier: 'BIT_NUM',
    name: 'Bit Number (bits 5-3)',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'bit',
        offset: 3,
        size: 3,
      },
    ],
  },

  // Port address (8-bit immediate for IN/OUT)
  {
    identifier: 'PORT_u8',
    name: 'Port Address',
    type: InstructionDataTypes.Immediate,
    size: 8,
    fields: [{ identifier: 'PORT', offset: 0, size: 8 }],
  },
];

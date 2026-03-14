import { RegisterTypes } from '@machinery/core';
import type { RegisterInfo } from '@machinery/core';

export const state: RegisterInfo[] = [
  // ALU operand registers
  {
    identifier: 'a',
    name: 'ALU Destination Operand',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'b',
    name: 'ALU Source Operand',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Divisor state
  {
    identifier: 'div_a',
    name: 'ALU Divisor Operand',
    size: 32,
    type: RegisterTypes.Integer,
  },
  // Last ALU result
  {
    identifier: 'alu_result',
    name: 'ALU Result',
    size: 32,
    type: RegisterTypes.Integer,
  },
  // The kind of pending flag operation
  {
    identifier: 'flag_op',
    name: 'FLAGS modifier state',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Repeat prefix loop target IP
  {
    identifier: 'REP',
    name: 'Repeat Prefix State',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Repeat (SCAS, CMPS) target flag comparison
  {
    identifier: 'REP_CHECK',
    name: 'Repeat Prefix Destination',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // The base for all data segment reads
  // (overridable by segment prefix)
  {
    identifier: 'DATA_SEG_BASE',
    name: 'Segment Override Base Address',
    size: 32,
    type: RegisterTypes.Integer,
    initialValue: 0xffff,
  },
  {
    identifier: 'DATA_SEG_LIMIT_MIN',
    name: 'Segment Override Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DATA_SEG_LIMIT_MAX',
    name: 'Segment Override Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'ES_BASE',
    name: 'Cached E Segment Base Address',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'CS_BASE',
    name: 'Cached Code Segment Base Address',
    size: 32,
    type: RegisterTypes.Integer,
  },
  // These are repeated to make ModRM decoding more efficient
  {
    identifier: 'DS_BASE',
    name: 'Cached Data Segment Base Address',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_BASE1',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_BASE',
    name: 'Cached Stack Segment Base Address',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_BASE1',
    name: 'Cached Stack Segment Base Address (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_BASE2',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_BASE3',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_BASE2',
    name: 'Cached Stack Segment Base Address (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_BASE4',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'FS_BASE',
    name: 'Cached F Segment Base Address',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'GS_BASE',
    name: 'Cached G Segment Base Address',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'ES_LIMIT_MIN',
    name: 'Cached E Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'CS_LIMIT_MIN',
    name: 'Cached Code Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // These are repeated to make ModRM decoding more efficient
  {
    identifier: 'DS_LIMIT_MIN',
    name: 'Cached Data Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MIN1',
    name: 'Cached Data Segment Limit (lower) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_LIMIT_MIN',
    name: 'Cached Stack Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_LIMIT_MIN1',
    name: 'Cached Stack Segment Limit (lower) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MIN2',
    name: 'Cached Data Segment Limit (lower) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MIN3',
    name: 'Cached Data Segment Limit (lower) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_LIMIT_MIN2',
    name: 'Cached Stack Segment Limit (lower) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MIN4',
    name: 'Cached Data Segment Limit (lower) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'FS_LIMIT_MIN',
    name: 'Cached F Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'GS_LIMIT_MIN',
    name: 'Cached G Segment Limit (lower)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'ES_LIMIT_MAX',
    name: 'Cached E Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'CS_LIMIT_MAX',
    name: 'Cached Code Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // These are repeated to make ModRM decoding more efficient
  {
    identifier: 'DS_LIMIT_MAX',
    name: 'Cached Data Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MAX1',
    name: 'Cached Data Segment Limit (upper) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_LIMIT_MAX',
    name: 'Cached Stack Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_LIMIT_MAX1',
    name: 'Cached Stack Segment Limit (upper) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MAX2',
    name: 'Cached Data Segment Limit (upper) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MAX3',
    name: 'Cached Data Segment Limit (upper) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_LIMIT_MAX2',
    name: 'Cached Stack Segment Limit (upper) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_LIMIT_MAX4',
    name: 'Cached Data Segment Limit (upper) (duplicate)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'FS_LIMIT_MAX',
    name: 'Cached F Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'GS_LIMIT_MAX',
    name: 'Cached G Segment Limit (upper)',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'ES_ACCESS',
    name: 'Cached E Segment Access Rights',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'CS_ACCESS',
    name: 'Cached Code Segment Access Rights',
    size: 32,
    type: RegisterTypes.Integer,
  },
  // These are repeated to make ModRM decoding more efficient
  {
    identifier: 'DS_ACCESS',
    name: 'Cached Data Segment Access Rights',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_ACCESS1',
    name: 'Cached Data Segment Access Rights (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_ACCESS',
    name: 'Cached Stack Segment Access Rights',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_ACCESS1',
    name: 'Cached Stack Segment Access Rights (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_ACCESS2',
    name: 'Cached Data Segment Access Rights (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_ACCESS3',
    name: 'Cached Data Segment Access Rights (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SS_ACCESS2',
    name: 'Cached Stack Segment Access Rights (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DS_ACCESS4',
    name: 'Cached Data Segment Access Rights (duplicate)',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'FS_ACCESS',
    name: 'Cached F Segment Access Rights',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'GS_ACCESS',
    name: 'Cached G Segment Access Rights',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'CARRY',
    name: 'Pending Carry Flag',
    size: 8,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'FETCH_IP',
    name: 'Precomputed effective address of the next instruction',
    size: 32,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'EXCEPTION_DEPTH',
    name: 'Exception Delivery Nesting Depth',
    size: 8,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'HALTED',
    name: 'Machine Halt Status',
    size: 1,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'HALTED_CS',
    name: 'CS Value at Halt',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'HALTED_IP',
    name: 'IP Value at Halt',
    size: 16,
    type: RegisterTypes.Integer,
  },
];

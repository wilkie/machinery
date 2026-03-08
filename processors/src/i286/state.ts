import { RegisterType } from '@machinery/core';
import type { RegisterInfo } from '@machinery/core';

export const state: RegisterInfo[] = [
  // ALU operand registers
  {
    identifier: 'a',
    name: 'ALU Destination Operand',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'b',
    name: 'ALU Source Operand',
    size: 16,
    type: RegisterType.Integer,
  },
  // Divisor state
  {
    identifier: 'div_a',
    name: 'ALU Divisor Operand',
    size: 32,
    type: RegisterType.Integer,
  },
  // Last ALU result
  {
    identifier: 'alu_result',
    name: 'ALU Result',
    size: 32,
    type: RegisterType.Integer,
  },
  // The kind of pending flag operation
  {
    identifier: 'flag_op',
    name: 'FLAGS modifier state',
    size: 16,
    type: RegisterType.Integer,
  },
  // Repeat prefix loop target IP
  {
    identifier: 'REP',
    name: 'Repeat Prefix State',
    size: 16,
    type: RegisterType.Integer,
  },
  // Repeat (SCAS, CMPS) target flag comparison
  {
    identifier: 'REP_CHECK',
    name: 'Repeat Prefix Destination',
    size: 16,
    type: RegisterType.Integer,
  },
  // The base for all data segment reads
  // (overridable by segment prefix)
  {
    identifier: 'DATA_SEG_BASE',
    name: 'Segment Override Base Address',
    size: 16,
    type: RegisterType.Integer,
    initialValue: 0xffff,
  },
  {
    identifier: 'ES_BASE',
    name: 'Cached E Segment Base Address',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'CS_BASE',
    name: 'Cached Code Segment Base Address',
    size: 16,
    type: RegisterType.Integer,
  },
  // These are repeated to make ModRM decoding more efficient
  {
    identifier: 'DS_BASE',
    name: 'Cached Data Segment Base Address',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'DS_BASE1',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'SS_BASE',
    name: 'Cached Stack Segment Base Address',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'SS_BASE1',
    name: 'Cached Stack Segment Base Address (duplicate)',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'DS_BASE2',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'DS_BASE3',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'SS_BASE2',
    name: 'Cached Stack Segment Base Address (duplicate)',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'DS_BASE4',
    name: 'Cached Data Segment Base Address (duplicate)',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'FS_BASE',
    name: 'Cached F Segment Base Address',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'GS_BASE',
    name: 'Cached G Segment Base Address',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'CARRY',
    name: 'Pending Carry Flag',
    size: 8,
    type: RegisterType.Integer,
  },
  {
    identifier: 'FETCH_IP',
    name: 'Precomputed effective address of the next instruction',
    size: 32,
    type: RegisterType.Integer,
  },
  {
    identifier: 'HALTED',
    name: 'Machine Halt Status',
    size: 1,
    type: RegisterType.Integer,
  },
  {
    identifier: 'HALTED_CS',
    name: 'CS Value at Halt',
    size: 16,
    type: RegisterType.Integer,
  },
  {
    identifier: 'HALTED_IP',
    name: 'IP Value at Halt',
    size: 16,
    type: RegisterType.Integer,
  },
];

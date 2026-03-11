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
  // Temporary register for operations
  {
    identifier: 'tmp',
    name: 'Temporary Register',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Interrupt flip-flops
  {
    identifier: 'IFF1',
    name: 'Interrupt Flip-Flop 1',
    size: 8,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'IFF2',
    name: 'Interrupt Flip-Flop 2',
    size: 8,
    type: RegisterTypes.Integer,
  },
  // Machine halt status
  {
    identifier: 'HALTED',
    name: 'Machine Halt Status',
    size: 1,
    type: RegisterTypes.Integer,
  },
];

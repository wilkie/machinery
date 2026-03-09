import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const seg: InstructionInfo = {
  identifier: 'seg',
  prefix: true,
  name: 'Segment Override Prefix',
  description: '',
  modifies: [],
  undefined: [],
  forms: [
    {
      opcode: [Opcodes.ES_SEG_PREFIX],
      operands: [],
      segmentOverride: 'ES',
      operation: ['DATA_SEG_BASE = ES_BASE'],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
    {
      opcode: [Opcodes.CS_SEG_PREFIX],
      operands: [],
      segmentOverride: 'CS',
      operation: ['DATA_SEG_BASE = CS_BASE'],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
    {
      opcode: [Opcodes.SS_SEG_PREFIX],
      operands: [],
      segmentOverride: 'SS',
      operation: ['DATA_SEG_BASE = SS_BASE'],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
    {
      opcode: [Opcodes.DS_SEG_PREFIX],
      operands: [],
      segmentOverride: 'DS',
      operation: ['DATA_SEG_BASE = DS_BASE'],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
  ],
};

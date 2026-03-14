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
      operation: [
        'DATA_SEG_BASE = ES_BASE',
        'DATA_SEG_LIMIT_MIN = ES_LIMIT_MIN',
        'DATA_SEG_LIMIT_MAX = ES_LIMIT_MAX',
      ],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
    {
      opcode: [Opcodes.CS_SEG_PREFIX],
      operands: [],
      segmentOverride: 'CS',
      operation: [
        'DATA_SEG_BASE = CS_BASE',
        'DATA_SEG_LIMIT_MIN = CS_LIMIT_MIN',
        'DATA_SEG_LIMIT_MAX = CS_LIMIT_MAX',
      ],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
    {
      opcode: [Opcodes.SS_SEG_PREFIX],
      operands: [],
      segmentOverride: 'SS',
      operation: [
        'DATA_SEG_BASE = SS_BASE',
        'DATA_SEG_LIMIT_MIN = SS_LIMIT_MIN',
        'DATA_SEG_LIMIT_MAX = SS_LIMIT_MAX',
      ],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
    {
      opcode: [Opcodes.DS_SEG_PREFIX],
      operands: [],
      segmentOverride: 'DS',
      operation: [
        'DATA_SEG_BASE = DS_BASE',
        'DATA_SEG_LIMIT_MIN = DS_LIMIT_MIN',
        'DATA_SEG_LIMIT_MAX = DS_LIMIT_MAX',
      ],
      finalize: ['DATA_SEG_BASE = 0xffff'],
    },
  ],
};

export const interrupts = {
  handler: {
    locals: [
      {
        identifier: 'NEW_CS',
        name: 'Target interrupt vector CS',
        size: 16,
      },
      {
        identifier: 'NEW_IP',
        name: 'Target interrupt vector IP',
        size: 16,
      },
      {
        identifier: 'stack_address',
        name: 'Effective Stack Address',
        size: 32,
      },
      {
        identifier: 'tmp',
        name: 'Temporary stack pointer',
        size: 16,
      },
    ],
    modes: {
      real: {
        operation: [
          ';; check IF flag for maskable interrupts',
          ';; check to see if the interrupt handler exists',
          'NEW_CS = RAM.IVT.vectors[vector].segment',
          'NEW_IP = RAM.IVT.vectors[vector].offset',
          ';; push CS / IP / FLAGS on to the stack',
          'tmp = SP - 6',
          'stack_address = SS_BASE + tmp',
          'RAM:u16[stack_address] = IP',
          'RAM:u16[stack_address + 2] = CS',
          'RAM:u16[stack_address + 4] = FLAGS',
          'SP = tmp',
          ';; jump to the interrupt handler',
          'CS = NEW_CS',
          'IP = NEW_IP',
        ],
      },
      protected: {
        operation: [
          ';; check IF flag for maskable interrupts',
          ';; check to see if the interrupt handler exists',
          'NEW_CS = RAM.IDT.gates[vector].segment',
          'NEW_IP = RAM.IDT.gates[vector].offset',
          ';; push CS / IP / FLAGS on to the stack',
          'tmp = SP - 6',
          'stack_address = SS_BASE + tmp',
          'RAM:u16[stack_address] = IP',
          'RAM:u16[stack_address + 2] = CS',
          'RAM:u16[stack_address + 4] = FLAGS',
          'SP = tmp',
          ';; jump to the interrupt handler',
          'CS = NEW_CS',
          'IP = NEW_IP',
        ],
      },
    },
  },
  vectors: [
    {
      identifier: 'DE',
      name: 'Divide Error',
      index: 0,
    },
    {
      identifier: 'DB',
      name: 'Single Step Interrupt',
      index: 1,
    },
    {
      identifier: 'NMI',
      name: 'Non-maskable Interrupt',
      index: 2,
    },
    {
      identifier: 'BP',
      name: 'Breakpoint',
      index: 3,
    },
    {
      identifier: 'OF',
      name: 'Overflow',
      index: 4,
    },
    {
      identifier: 'BR',
      name: 'Bound Range Exceeded',
      index: 5,
    },
    {
      identifier: 'UD',
      name: 'Invalid Opcode',
      index: 6,
    },
    {
      identifier: 'NM',
      name: 'Device Not Available',
      index: 7,
    },
    {
      identifier: 'DF',
      name: 'Double Fault',
      index: 8,
    },
    {
      identifier: 'TS',
      name: 'Invalid Task State Segment',
      index: 10,
    },
    {
      identifier: 'NP',
      name: 'Segment Not Present',
      index: 11,
    },
    {
      identifier: 'SS',
      name: 'Stack Segment Fault',
      index: 12,
    },
    {
      identifier: 'GP',
      name: 'General Protection Fault',
      index: 13,
    },
    {
      identifier: 'MF',
      name: 'x87 Floating-Point Exception',
      index: 16,
    },
  ],
};

export default interrupts;

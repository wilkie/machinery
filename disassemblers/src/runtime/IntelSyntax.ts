import type { Syntax } from './Syntax';
import type {
  DecodedInstruction,
  DecodedOperand,
  MemoryOperand,
} from './types';

/**
 * Intel-syntax formatter: `MOV AX, 0x1234`, `ADD [BX+SI+4], AL`
 */
export class IntelSyntax implements Syntax {
  formatInstruction(instr: DecodedInstruction): string {
    const parts: string[] = [];

    if (instr.prefix) {
      parts.push(instr.prefix.toUpperCase());
    }

    parts.push(instr.mnemonic.toUpperCase());

    if (instr.operands.length > 0) {
      const operandStrs = instr.operands.map((op) =>
        this.formatOperand(op, instr.address),
      );
      parts.push(operandStrs.join(', '));
    }

    // Join prefix/mnemonic with space, then operands
    if (instr.prefix) {
      return `${parts[0]} ${parts[1]}${parts.length > 2 ? ' ' + parts[2] : ''}`;
    }
    return `${parts[0]}${parts.length > 1 ? ' ' + parts[1] : ''}`;
  }

  formatOperand(operand: DecodedOperand, _instrAddress: number): string {
    switch (operand.type) {
      case 'register':
        return operand.name.toUpperCase();

      case 'immediate':
        return this.formatHex(operand.value, operand.size);

      case 'relative':
        return this.formatHex(operand.target, 16);

      case 'memory':
        return this.formatMemory(operand);

      case 'far_pointer':
        return `${this.formatHex(operand.segment, 16)}:${this.formatHex(operand.offset, 16)}`;

      default:
        return '?';
    }
  }

  private formatMemory(op: MemoryOperand): string {
    const parts: string[] = [];
    let prefix = '';

    if (op.segment) {
      prefix = `${op.segment.toUpperCase()}:`;
    }

    if (op.direct) {
      return `${prefix}[${this.formatHex(op.displacement, 16)}]`;
    }

    if (op.base) parts.push(op.base.toUpperCase());
    if (op.index) parts.push(op.index.toUpperCase());

    if (op.displacement !== 0) {
      if (op.displacement < 0) {
        parts.push(
          `- ${this.formatHex(-op.displacement, op.displacementSize)}`,
        );
      } else if (parts.length > 0) {
        parts.push(`+ ${this.formatHex(op.displacement, op.displacementSize)}`);
      } else {
        parts.push(this.formatHex(op.displacement, op.displacementSize));
      }
    }

    const inner = parts.join(' + ').replace(' + - ', ' - ');
    return `${prefix}[${inner}]`;
  }

  private formatHex(value: number, size: number): string {
    if (value < 0) {
      // For negative values in relative display, show the absolute target
      const mask = size === 8 ? 0xff : 0xffff;
      value = value & mask;
    }
    const digits = size <= 8 ? 2 : 4;
    return `0x${(value & (size <= 8 ? 0xff : 0xffff)).toString(16).padStart(digits, '0')}`;
  }
}

import type { Syntax } from './Syntax';
import type {
  DecodedInstruction,
  DecodedOperand,
  MemoryOperand,
} from './types';

/**
 * AT&T-syntax formatter (objdump-style): `mov %ax,%bx`, `add $0x42,%al`
 *
 * Key differences from Intel syntax:
 * - Registers prefixed with %
 * - Immediates prefixed with $
 * - Source, destination order is reversed (source first, dest second)
 * - Memory references use displacement(%base,%index) notation
 */
export class AttSyntax implements Syntax {
  formatInstruction(instr: DecodedInstruction): string {
    const parts: string[] = [];

    if (instr.prefix) {
      parts.push(instr.prefix.toLowerCase());
    }

    parts.push(instr.mnemonic.toLowerCase());

    if (instr.operands.length > 0) {
      // AT&T reverses operand order for two-operand instructions
      const ops = [...instr.operands];
      if (ops.length === 2) {
        ops.reverse();
      }
      const operandStrs = ops.map((op) =>
        this.formatOperand(op, instr.address),
      );
      parts.push(operandStrs.join(','));
    }

    if (instr.prefix) {
      return `${parts[0]} ${parts[1]}${parts.length > 2 ? ' ' + parts[2] : ''}`;
    }
    return `${parts[0]}${parts.length > 1 ? ' ' + parts.slice(1).join(' ') : ''}`;
  }

  formatOperand(operand: DecodedOperand, _instrAddress: number): string {
    switch (operand.type) {
      case 'register':
        return `%${operand.name.toLowerCase()}`;

      case 'immediate':
        return `$${this.formatHex(operand.value)}`;

      case 'relative':
        return this.formatHex(operand.target);

      case 'memory':
        return this.formatMemory(operand);

      case 'far_pointer':
        return `${this.formatHex(operand.segment)}:${this.formatHex(operand.offset)}`;

      default:
        return '?';
    }
  }

  private formatMemory(op: MemoryOperand): string {
    let prefix = '';
    if (op.segment) {
      prefix = `%${op.segment.toLowerCase()}:`;
    }

    if (op.direct) {
      return `${prefix}${this.formatHex(op.displacement)}`;
    }

    const regParts: string[] = [];
    if (op.base) regParts.push(`%${op.base.toLowerCase()}`);
    if (op.index) regParts.push(`%${op.index.toLowerCase()}`);

    const dispStr =
      op.displacement !== 0 ? this.formatHex(op.displacement) : '';
    const regStr = regParts.length > 0 ? `(${regParts.join(',')})` : '';

    return `${prefix}${dispStr}${regStr}`;
  }

  private formatHex(value: number): string {
    if (value < 0) {
      return `-0x${(-value).toString(16)}`;
    }
    return `0x${value.toString(16)}`;
  }
}

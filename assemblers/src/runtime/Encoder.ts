import type { Operand, MemoryOperand } from './ast';

export interface EncoderField {
  identifier: string;
  offset: number;
  size: number;
  signed?: boolean;
  match?: number;
  type?: number;
  encoding?: (string | null)[];
}

export interface EncoderMatcher {
  identifier: string;
  name?: string;
  type: number;
  size: number;
  signed?: boolean;
  fields?: EncoderField[];
}

export type OpcodeEntry =
  | { kind: 'fixed'; value: number }
  | { kind: 'ref'; identifier: string }
  | { kind: 'inline'; matcher: EncoderMatcher };

export interface EncoderForm {
  mnemonic: string;
  operands: string[];
  operandSize?: number;
  opcodeEntries: OpcodeEntry[];
  aliases?: string[];
  commutative?: boolean;
  distance?: 'short' | 'near' | 'far';
  addressing?: 'relative' | 'absolute';
  encodingPriority?: number;
}

// InstructionDataTypes/InstructionOperandTypes values from @machinery/core
const DataTypes = {
  Opcode: 0,
  Operand: 1,
  Displacement: 2,
  Immediate: 4,
} as const;
const OperandTypes = {
  Source: 0,
  Register: 1,
  SystemRegister: 2,
  Memory: 3,
  Interrupt: 4,
  RelativeJump: 5,
  AbsoluteJump: 6,
} as const;

export interface EncodeResult {
  bytes: number[];
  needsFixup?: { offset: number; size: number; relative?: boolean };
}

export class Encoder {
  private operandDefs: Record<string, EncoderMatcher>;
  private forms: EncoderForm[];
  private segmentPrefixes: Record<string, number[]>;
  private endianness: 'little' | 'big';

  constructor(
    operandDefinitions: Record<string, EncoderMatcher>,
    encoderForms: EncoderForm[],
    segmentOverridePrefixes: Record<string, number[]> = {},
    endianness: 'little' | 'big' = 'little',
  ) {
    this.operandDefs = operandDefinitions;
    this.forms = encoderForms;
    this.segmentPrefixes = segmentOverridePrefixes;
    this.endianness = endianness;
  }

  /** Emit a multi-byte value in the target's byte order. */
  private emitValue(value: number, numBytes: number): number[] {
    const bytes: number[] = [];
    if (this.endianness === 'big') {
      for (let i = numBytes - 1; i >= 0; i--) {
        bytes.push((value >> (i * 8)) & 0xff);
      }
    } else {
      for (let i = 0; i < numBytes; i++) {
        bytes.push((value >> (i * 8)) & 0xff);
      }
    }
    return bytes;
  }

  encode(
    mnemonic: string,
    operands: Operand[],
    labels: Record<string, number>,
    currentOffset: number,
    minSize?: number,
  ): EncodeResult {
    const upperMnemonic = mnemonic.toUpperCase();
    const candidates = this.findMatchingForms(upperMnemonic, operands);

    if (candidates.length === 0) {
      throw new Error(
        `No encoding found for: ${upperMnemonic} ${this.formatOperands(operands)}`,
      );
    }

    // Prefer smallest encoding; when sizes are equal, prefer lower encodingPriority
    let bestResult: EncodeResult | null = null;
    let bestSize = Infinity;
    let bestPriority = Infinity;

    // Account for segment prefix in minSize
    const segmentPrefix = this.getSegmentPrefix(operands);
    const segPrefixLen = segmentPrefix ? segmentPrefix.length : 0;
    const effectiveMinSize =
      minSize !== undefined ? minSize - segPrefixLen : undefined;

    for (const { form, effectiveOperands } of candidates) {
      try {
        const result = this.encodeForm(
          form,
          effectiveOperands,
          labels,
          currentOffset,
        );
        // If minSize is specified, skip encodings smaller than the minimum
        // (to maintain consistency with size estimation)
        if (
          effectiveMinSize !== undefined &&
          result.bytes.length < effectiveMinSize
        ) {
          continue;
        }
        const priority = form.encodingPriority ?? 0;
        if (
          result.bytes.length < bestSize ||
          (result.bytes.length === bestSize && priority < bestPriority)
        ) {
          bestSize = result.bytes.length;
          bestResult = result;
          bestPriority = priority;
        }
      } catch {
        continue;
      }
    }

    // If minSize filtering excluded all results, fall back to smallest without constraint
    if (!bestResult) {
      for (const { form, effectiveOperands } of candidates) {
        try {
          const result = this.encodeForm(
            form,
            effectiveOperands,
            labels,
            currentOffset,
          );
          if (!bestResult || result.bytes.length < bestResult.bytes.length) {
            bestResult = result;
          }
        } catch {
          continue;
        }
      }
    }

    if (!bestResult) {
      throw new Error(
        `Failed to encode: ${upperMnemonic} ${this.formatOperands(operands)}`,
      );
    }

    // Prepend segment override prefix if any memory operand specifies a segment
    if (segmentPrefix) {
      bestResult.bytes.unshift(...segmentPrefix);
      if (bestResult.needsFixup) {
        bestResult.needsFixup.offset += segmentPrefix.length;
      }
    }

    return bestResult;
  }

  estimateSize(mnemonic: string, operands: Operand[]): number {
    const upperMnemonic = mnemonic.toUpperCase();
    const candidates = this.findMatchingForms(upperMnemonic, operands);
    if (candidates.length === 0) return 0;

    // Return max possible size for safety in first pass
    let maxSize = 0;
    for (const { form } of candidates) {
      let size = 0;
      for (const entry of form.opcodeEntries) {
        if (entry.kind === 'fixed') {
          size += 1;
        } else {
          const matcher =
            entry.kind === 'ref'
              ? this.operandDefs[entry.identifier]
              : entry.matcher;
          if (matcher) {
            size += Math.ceil(matcher.size / 8);
          }
        }
      }
      if (size > maxSize) maxSize = size;
    }
    // Account for segment override prefix
    const segmentPrefix = this.getSegmentPrefix(operands);
    if (segmentPrefix) {
      maxSize += segmentPrefix.length;
    }

    return maxSize;
  }

  private getSegmentPrefix(operands: Operand[]): number[] | null {
    for (const op of operands) {
      if (op.type === 'memory' && op.segment) {
        const prefix = this.segmentPrefixes[op.segment];
        if (prefix) return prefix;
      }
    }
    return null;
  }

  private findMatchingForms(
    mnemonic: string,
    operands: Operand[],
  ): { form: EncoderForm; effectiveOperands: Operand[] }[] {
    const results: { form: EncoderForm; effectiveOperands: Operand[] }[] = [];

    for (const form of this.forms) {
      if (
        form.mnemonic !== mnemonic &&
        !(form.aliases && form.aliases.includes(mnemonic))
      ) {
        continue;
      }

      if (this.operandsMatch(form, operands)) {
        // Skip distance check when matched via alias (e.g., RETF implies far)
        const matchedViaAlias = form.mnemonic !== mnemonic;
        if (!matchedViaAlias && this.distanceMismatch(form, operands)) continue;
        if (this.canEncode(form, operands)) {
          results.push({ form, effectiveOperands: operands });
        }
      }

      // For commutative instructions, also try swapped operand order
      if (form.commutative && operands.length === 2) {
        const swapped = [operands[1], operands[0]];
        if (this.operandsMatch(form, swapped)) {
          if (this.canEncode(form, swapped)) {
            results.push({ form, effectiveOperands: swapped });
          }
        }
      }

      // For shift/rotate by immediate 1, also try the implicit-1 form (D0/D1)
      // which has only ["rm"] operands (no immediate)
      if (
        operands.length === 2 &&
        operands[1].type === 'immediate' &&
        operands[1].value === 1 &&
        form.operands.length === 1 &&
        form.operands[0] === 'rm'
      ) {
        const singleOp = [operands[0]];
        if (this.operandsMatch(form, singleOp)) {
          if (this.canEncode(form, singleOp)) {
            results.push({ form, effectiveOperands: singleOp });
          }
        }
      }
    }

    return results;
  }

  /**
   * Check if the form's distance/addressing doesn't match the operand's qualifiers.
   * The grammar sets `far: true` or `near: true` on operands based on syntax keywords.
   * Returns true if the form should be skipped.
   */
  private distanceMismatch(form: EncoderForm, operands: Operand[]): boolean {
    if (!form.distance) return false;

    const hasFarOperand = operands.some((op) => 'far' in op && op.far === true);
    const hasNearOperand = operands.some(
      (op) => 'near' in op && op.near === true,
    );

    // 'far' keyword: only match far forms
    if (hasFarOperand && form.distance !== 'far') return true;

    // 'near' keyword: only match near forms (skip short and far)
    if (hasNearOperand && form.distance !== 'near') return true;

    // No qualifier but form is far: skip (far forms require explicit 'far' keyword)
    if (!hasFarOperand && form.distance === 'far') return true;

    return false;
  }

  private operandsMatch(form: EncoderForm, operands: Operand[]): boolean {
    const formOps = form.operands;

    if (formOps.length !== operands.length) return false;

    for (let i = 0; i < formOps.length; i++) {
      const formOp = formOps[i];
      const actualOp = operands[i];

      switch (formOp) {
        case 'rm':
          if (actualOp.type !== 'register' && actualOp.type !== 'memory') {
            // Accept immediate if the form has a dual displacement/immediate matcher
            if (actualOp.type === 'immediate' && this.formHasDispImm(form)) {
              break;
            }
            return false;
          }
          if (form.operandSize !== undefined) {
            if (
              actualOp.type === 'register' &&
              actualOp.size !== form.operandSize
            )
              return false;
            if (
              actualOp.type === 'memory' &&
              actualOp.size !== undefined &&
              actualOp.size !== form.operandSize
            )
              return false;
          }
          break;
        case 'reg':
          if (actualOp.type !== 'register') return false;
          if (
            form.operandSize !== undefined &&
            actualOp.size !== form.operandSize
          )
            return false;
          break;
        case 'seg':
          if (actualOp.type !== 'register') return false;
          break;
        case 'imm':
        case 'level':
          if (actualOp.type !== 'immediate') return false;
          // Reject far pointers for non-ptr operand types
          if (
            actualOp.type === 'immediate' &&
            typeof actualOp.value === 'object' &&
            actualOp.value &&
            'seg' in actualOp.value
          )
            return false;
          break;
        case 'rel':
          if (actualOp.type !== 'immediate') return false;
          // Reject far pointers for relative jumps
          if (
            typeof actualOp.value === 'object' &&
            actualOp.value &&
            'seg' in actualOp.value
          )
            return false;
          break;
        case 'ptr':
          if (actualOp.type !== 'immediate') return false;
          // ptr requires a far pointer value (seg:off object), not a simple number or label
          if (
            typeof actualOp.value !== 'object' ||
            !actualOp.value ||
            !('seg' in actualOp.value)
          )
            return false;
          break;
        case 'mem':
          if (actualOp.type !== 'memory') return false;
          // 'mem' operand type = direct memory address (moffs), no base/index allowed
          if (actualOp.base || actualOp.index) return false;
          break;
        default:
          // Literal register name match (e.g., 'AL', 'AX', 'ES', 'CL', 'DX')
          if (actualOp.type === 'register' && actualOp.name === formOp) break;
          return false;
      }
    }

    return true;
  }

  private canEncode(form: EncoderForm, operands: Operand[]): boolean {
    // For relative jump forms, the value-fits check must consider the relative offset,
    // not the absolute target address. We skip the check here and let encoding handle it.
    const isRelative = form.operands.includes('rel');

    for (const entry of form.opcodeEntries) {
      if (entry.kind === 'fixed') continue;
      const matcher =
        entry.kind === 'ref'
          ? this.operandDefs[entry.identifier]
          : entry.matcher;
      if (!matcher) continue;

      // Check immediate value fits in the matcher's size
      if (
        matcher.type & DataTypes.Immediate ||
        matcher.type & DataTypes.Displacement
      ) {
        // Skip value-fits check for relative operands - the raw value is an absolute address,
        // not the relative offset that will be encoded
        if (!isRelative) {
          const immOperandIdx = this.findImmediateOperandIndex(form, matcher);
          if (immOperandIdx !== -1 && immOperandIdx < operands.length) {
            const operand = operands[immOperandIdx];
            if (
              operand.type === 'immediate' &&
              typeof operand.value === 'number'
            ) {
              let immValue = operand.value;
              // For sign-extended immediate forms (e.g., 0x83 group: 16-bit operandSize with 8-bit signed imm),
              // interpret the value as a signed integer at operand size before checking fit.
              // E.g., 0xFF85 as 16-bit signed = -123, which fits in a signed byte.
              if (
                matcher.signed &&
                form.operandSize &&
                matcher.size < form.operandSize
              ) {
                // Sign-extended immediate form (e.g., 0x83 group, 0x6A PUSH).
                // Reinterpret value as signed at operand size, then check strict signed fit.
                const signBit = 1 << (form.operandSize - 1);
                if (immValue >= signBit) {
                  immValue = immValue - (1 << form.operandSize);
                }
                // Must fit in strict signed range — unsigned values like 0x80 or 0xF0
                // cannot be sign-extended correctly (0x80 sign-extends to 0xFF80, not 0x0080).
                const smin = -(1 << (matcher.size - 1));
                const smax = (1 << (matcher.size - 1)) - 1;
                if (immValue < smin || immValue > smax) {
                  return false;
                }
              } else if (!this.valueFits(immValue, matcher.size)) {
                return false;
              }
            }
          }
        }
      }

      if (!matcher.fields) continue;

      for (const field of matcher.fields) {
        if (field.match !== undefined) {
          // Direct address forms (e.g., ModRM_110_xxx_00 with rm match: 6)
          // should not match memory operands that have base/index registers.
          // Those forms only match [disp16] (no base/index).
          if (field.identifier === 'rm') {
            const operandIdx = this.findOperandIndex(
              form.operands,
              field.identifier,
            );
            if (operandIdx !== -1 && operandIdx < operands.length) {
              const operand = operands[operandIdx];
              if (operand.type === 'memory') {
                const mem = operand as MemoryOperand;
                if (mem.base || mem.index) return false;
              }
            }
          }
          continue;
        }
        if (!field.encoding) continue;
        if (field.type === undefined) continue;

        const operandIdx = this.findOperandIndex(
          form.operands,
          field.identifier,
        );
        if (operandIdx === -1) continue;

        const operand = operands[operandIdx];
        if (
          field.type === OperandTypes.Register &&
          operand.type === 'register'
        ) {
          if (!field.encoding.includes(operand.name)) return false;
        } else if (
          field.type === OperandTypes.Memory &&
          operand.type === 'memory'
        ) {
          const memKey = this.memoryAddressingKey(operand);
          if (!field.encoding.includes(memKey)) return false;

          // Check displacement compatibility with the form's ModRM mode
          const mem = operand as MemoryOperand;
          const hasBase = mem.base || mem.index;
          if (hasBase) {
            const hasDispEntry = form.opcodeEntries.some(
              (e) =>
                (e.kind === 'ref' && e.identifier.startsWith('DISP')) ||
                (e.kind === 'inline' &&
                  e.matcher &&
                  (e.matcher.type & DataTypes.Displacement) !== 0),
            );
            const rawDisp = mem.displacement;
            const dispIsNumeric = typeof rawDisp === 'number';
            const disp = dispIsNumeric ? rawDisp : 0;

            if (!dispIsNumeric && rawDisp !== undefined && rawDisp !== null) {
              // Unresolved expression displacement — only allow 16-bit DISP forms
              if (!hasDispEntry) return false;
              for (const e of form.opcodeEntries) {
                const m =
                  e.kind === 'ref'
                    ? this.operandDefs[e.identifier]
                    : e.kind === 'inline'
                      ? e.matcher
                      : null;
                if (m && m.type & DataTypes.Displacement && m.size < 16)
                  return false;
              }
            } else {
              if (disp !== 0 && !hasDispEntry) {
                // Non-zero displacement but form has no DISP field — reject
                return false;
              }
              if (disp === 0 && hasDispEntry) {
                // Zero displacement but form requires DISP — reject,
                // unless no zero-displacement form can encode this memory mode.
                // (e.g., [BP] on x86: the mod=00 r/m=110 slot is stolen for
                // direct addressing, so the encoding array has null there.)
                if (!this.memKeyNeedsDisplacement(form, field, memKey))
                  return false;
              }
              if (disp !== 0 && hasDispEntry) {
                // Check displacement size fits the form's DISP entry
                for (const e of form.opcodeEntries) {
                  const m =
                    e.kind === 'ref'
                      ? this.operandDefs[e.identifier]
                      : e.kind === 'inline'
                        ? e.matcher
                        : null;
                  if (m && m.type & DataTypes.Displacement) {
                    if (!this.valueFits(disp, m.size)) return false;
                  }
                }
              }
            }
          }
        }
      }
    }
    return true;
  }

  private valueFits(value: number, bits: number): boolean {
    const min = -(1 << (bits - 1));
    const max = (1 << (bits - 1)) - 1;
    const umax = 1 << bits;
    // Accept values in signed range OR unsigned range.
    // Negative values are valid even for unsigned matchers since they represent
    // two's complement bit patterns (e.g., -11 as 16-bit = 0xFFF5).
    return (value >= min && value <= max) || (value >= 0 && value < umax);
  }

  /** Check if a form has a dual displacement/immediate matcher (type has both flags). */
  private formHasDispImm(form: EncoderForm): boolean {
    for (const entry of form.opcodeEntries) {
      if (entry.kind === 'fixed') continue;
      const matcher =
        entry.kind === 'ref'
          ? this.operandDefs[entry.identifier]
          : entry.matcher;
      if (
        matcher &&
        matcher.type & DataTypes.Displacement &&
        matcher.type & DataTypes.Immediate
      ) {
        return true;
      }
    }
    return false;
  }

  private findImmediateOperandIndex(
    form: EncoderForm,
    matcher: EncoderMatcher,
  ): number {
    const fieldId = matcher.fields?.[0]?.identifier ?? matcher.identifier;
    for (let i = 0; i < form.operands.length; i++) {
      const fop = form.operands[i];
      if (
        fop === 'imm' ||
        fop === 'rel' ||
        fop === 'ptr' ||
        fop === 'level' ||
        fop === 'mem'
      ) {
        if (
          fieldId === 'IMM' &&
          (fop === 'imm' || fop === 'rel' || fop === 'ptr' || fop === 'mem')
        )
          return i;
        if (fieldId === 'LEVEL' && fop === 'level') return i;
        // Pure displacement comes from memory operand, but dual disp/imm can match immediates
        if (fieldId === 'DISP' && !(matcher.type & DataTypes.Immediate))
          return -1;
        return i;
      }
    }
    return -1;
  }

  private encodeForm(
    form: EncoderForm,
    operands: Operand[],
    labels: Record<string, number>,
    currentOffset: number,
  ): EncodeResult {
    const bytes: number[] = [];
    let fixup: EncodeResult['needsFixup'];

    for (const entry of form.opcodeEntries) {
      if (entry.kind === 'fixed') {
        bytes.push(entry.value & 0xff);
        continue;
      }

      const matcher =
        entry.kind === 'ref'
          ? this.operandDefs[entry.identifier]
          : entry.matcher;
      if (!matcher) {
        throw new Error(
          `Unknown operand reference: ${entry.kind === 'ref' ? entry.identifier : 'inline'}`,
        );
      }

      const matcherBytes = this.encodeMatcher(
        matcher,
        form,
        operands,
        labels,
        currentOffset,
        bytes.length,
      );
      if (matcherBytes.fixup) {
        fixup = {
          offset: bytes.length + matcherBytes.fixup.offset,
          size: matcherBytes.fixup.size,
          relative: matcherBytes.fixup.relative,
        };
      }
      bytes.push(...matcherBytes.bytes);
    }

    return { bytes, needsFixup: fixup };
  }

  private encodeMatcher(
    matcher: EncoderMatcher,
    form: EncoderForm,
    operands: Operand[],
    labels: Record<string, number>,
    currentOffset: number,
    byteOffset: number,
  ): {
    bytes: number[];
    fixup?: { offset: number; size: number; relative?: boolean };
  } {
    const numBytes = Math.ceil(matcher.size / 8);

    // Displacement or Immediate with no sub-fields needing encoding from operands
    if (
      matcher.type & DataTypes.Displacement ||
      matcher.type & DataTypes.Immediate
    ) {
      return this.encodeValueMatcher(
        matcher,
        form,
        operands,
        labels,
        currentOffset,
        byteOffset,
      );
    }

    // Operand matcher (like ModRM) - build byte from fields
    if (matcher.fields && matcher.fields.length > 0) {
      let value = 0;

      for (const field of matcher.fields) {
        let fieldValue: number;

        if (field.match !== undefined) {
          fieldValue = field.match;
        } else if (field.encoding && field.type !== undefined) {
          const operandIdx = this.findOperandIndex(
            form.operands,
            field.identifier,
          );
          if (operandIdx === -1) {
            throw new Error(
              `Cannot find operand for field ${field.identifier}`,
            );
          }

          const operand = operands[operandIdx];
          fieldValue = this.lookupEncoding(field, operand);
        } else {
          fieldValue = 0;
        }

        // Place field value at the correct bit offset
        value |= (fieldValue & ((1 << field.size) - 1)) << field.offset;
      }

      return { bytes: this.emitValue(value, numBytes) };
    }

    // Fallback: emit zeros
    return { bytes: new Array(numBytes).fill(0) };
  }

  private encodeValueMatcher(
    matcher: EncoderMatcher,
    form: EncoderForm,
    operands: Operand[],
    labels: Record<string, number>,
    currentOffset: number,
    byteOffset: number,
  ): {
    bytes: number[];
    fixup?: { offset: number; size: number; relative?: boolean };
  } {
    const numBytes = Math.ceil(matcher.size / 8);
    const isRelative = form.operands.includes('rel');

    // Find the operand value
    let value = 0;
    let needsFixup = false;
    const fieldId = matcher.fields?.[0]?.identifier ?? matcher.identifier;

    // Map matcher to operand
    let operandIdx = -1;
    if (matcher.type & DataTypes.Immediate) {
      // Find the 'imm', 'rel', 'ptr', 'level', or 'mem' operand
      for (let i = 0; i < form.operands.length; i++) {
        const fop = form.operands[i];
        if (
          fop === 'imm' ||
          fop === 'rel' ||
          fop === 'ptr' ||
          fop === 'level' ||
          fop === 'mem'
        ) {
          // For forms with multiple immediates, match by field identifier
          if (
            fieldId === 'IMM' &&
            (fop === 'imm' || fop === 'rel' || fop === 'ptr' || fop === 'mem')
          ) {
            operandIdx = i;
            break;
          }
          if (fieldId === 'LEVEL' && fop === 'level') {
            operandIdx = i;
            break;
          }
          if (fieldId === 'DISP' && !(matcher.type & DataTypes.Immediate)) {
            // Pure displacement comes from memory operand, not from operand list
            break;
          }
          // Fallback for simple single-immediate
          if (operandIdx === -1) {
            operandIdx = i;
          }
        }
      }
    }

    if (matcher.type & DataTypes.Displacement) {
      // Displacement comes from memory operand
      const memIdx = form.operands.indexOf('rm');
      if (memIdx !== -1 && operands[memIdx]?.type === 'memory') {
        const mem = operands[memIdx] as MemoryOperand;
        value =
          (typeof mem.displacement === 'number' ? mem.displacement : 0) ?? 0;
      } else if (matcher.type & DataTypes.Immediate) {
        // Dual displacement/immediate: try immediate from 'imm' operand slot or 'rm' slot
        const immIdx = operandIdx !== -1 ? operandIdx : memIdx;
        if (immIdx !== -1 && operands[immIdx]?.type === 'immediate') {
          const operand = operands[immIdx];
          if (
            operand.type === 'immediate' &&
            typeof operand.value === 'number'
          ) {
            value = operand.value;
          } else if (
            operand.type === 'immediate' &&
            typeof operand.value === 'string'
          ) {
            needsFixup = true;
          }
        }
      }
    } else if (operandIdx !== -1 && operands[operandIdx]?.type === 'memory') {
      // Direct memory address: extract displacement from memory operand
      const mem = operands[operandIdx] as MemoryOperand;
      value =
        (typeof mem.displacement === 'number' ? mem.displacement : 0) ?? 0;
    } else if (operandIdx !== -1) {
      const operand = operands[operandIdx];
      if (operand.type === 'immediate') {
        if (typeof operand.value === 'number') {
          value = operand.value;
          if (isRelative) {
            // Calculate relative offset from end of instruction
            // This is approximate - the assembler will do a fixup pass
            value = value - (currentOffset + byteOffset + numBytes);
          }
        } else if (typeof operand.value === 'string') {
          // Label reference
          if (operand.value in labels) {
            value = labels[operand.value];
            if (isRelative) {
              value = value - (currentOffset + byteOffset + numBytes);
            }
          } else {
            needsFixup = true;
            value = 0;
          }
        } else if (
          typeof operand.value === 'object' &&
          operand.value !== null
        ) {
          // Far pointer: { seg, off }
          if ('seg' in operand.value && 'off' in operand.value) {
            const ptr = operand.value as { seg: number; off: number };
            if (fieldId === 'NEW_IP' || !fieldId) {
              value = typeof ptr.off === 'number' ? ptr.off : 0;
            } else if (fieldId === 'NEW_CS') {
              value = typeof ptr.seg === 'number' ? ptr.seg : 0;
            }
          }
        }
      }
    }

    // For relative jumps, verify the computed offset fits in the matcher's size
    // Must use strict signed range — unsigned values (128-255) would be sign-extended
    // as negative offsets by the CPU, jumping backwards instead of forwards.
    if (isRelative && !needsFixup) {
      const smin = -(1 << (matcher.size - 1));
      const smax = (1 << (matcher.size - 1)) - 1;
      if (value < smin || value > smax) {
        throw new Error(
          `Relative offset ${value} does not fit in ${matcher.size} bits`,
        );
      }
    }

    if ((matcher.signed || isRelative) && value < 0) {
      const mask = (1 << matcher.size) - 1;
      value = value & mask;
    }
    const bytes = this.emitValue(value, numBytes);

    const fixup = needsFixup
      ? { offset: 0, size: numBytes, relative: isRelative }
      : undefined;
    return { bytes, fixup };
  }

  private lookupEncoding(field: EncoderField, operand: Operand): number {
    if (!field.encoding) {
      throw new Error(`Field ${field.identifier} has no encoding`);
    }

    if (field.type === OperandTypes.Register) {
      let name: string;
      if (operand.type === 'register') {
        name = operand.name;
      } else {
        throw new Error(
          `Expected register operand for field ${field.identifier}`,
        );
      }
      const idx = field.encoding.indexOf(name);
      if (idx === -1) {
        throw new Error(
          `Register ${name} not found in encoding for ${field.identifier}`,
        );
      }
      return idx;
    }

    if (field.type === OperandTypes.Memory && operand.type === 'memory') {
      const memKey = this.memoryAddressingKey(operand);
      const idx = field.encoding.indexOf(memKey);
      if (idx === -1) {
        throw new Error(
          `Memory mode ${memKey} not found in encoding for ${field.identifier}`,
        );
      }
      return idx;
    }

    throw new Error(
      `Unsupported field type ${field.type} for operand type ${operand.type}`,
    );
  }

  private memoryAddressingKey(mem: MemoryOperand): string {
    const parts: string[] = [];
    if (mem.base) parts.push(mem.base);
    if (mem.index) parts.push(mem.index);
    return parts.join(' + ') || '';
  }

  private findOperandIndex(
    formOperands: string[],
    fieldIdentifier: string,
  ): number {
    // Direct match: field identifier matches operand slot name
    for (let i = 0; i < formOperands.length; i++) {
      if (formOperands[i] === fieldIdentifier) return i;
    }
    // If field is 'reg' and operands have 'seg', match segment
    if (fieldIdentifier === 'reg') {
      const segIdx = formOperands.indexOf('seg');
      if (segIdx !== -1) return segIdx;
    }
    return -1;
  }

  /**
   * Check if a memory addressing mode requires a displacement form because the
   * no-displacement form can't encode it. This is true when the encoding array
   * for the same field in a sibling no-displacement form has null at the index
   * for the given memory key, or when no such sibling form exists.
   */
  private memKeyNeedsDisplacement(
    currentForm: EncoderForm,
    currentField: EncoderField,
    memKey: string,
  ): boolean {
    // Find sibling forms: same mnemonic, same operand slots, no DISP entry
    for (const form of this.forms) {
      if (form.mnemonic !== currentForm.mnemonic) continue;
      if (form.operands.length !== currentForm.operands.length) continue;
      if (!form.operands.every((op, i) => op === currentForm.operands[i]))
        continue;

      // Must be a no-displacement form
      const hasDisp = form.opcodeEntries.some(
        (e) =>
          (e.kind === 'ref' && e.identifier.startsWith('DISP')) ||
          (e.kind === 'inline' &&
            e.matcher &&
            (e.matcher.type & DataTypes.Displacement) !== 0),
      );
      if (hasDisp) continue;

      // Find the same field (by identifier and type) in this form
      for (const entry of form.opcodeEntries) {
        if (entry.kind === 'fixed') continue;
        const matcher =
          entry.kind === 'ref'
            ? this.operandDefs[entry.identifier]
            : entry.matcher;
        if (!matcher?.fields) continue;
        for (const field of matcher.fields) {
          if (field.identifier !== currentField.identifier) continue;
          if (field.type !== currentField.type) continue;
          if (!field.encoding) continue;
          // If this field's encoding includes the memKey (non-null), the
          // no-displacement form can handle it — no need for displacement
          if (field.encoding.includes(memKey)) return false;
          // If null at that position, the no-displacement form can't encode it
          return true;
        }
      }
    }
    // No sibling no-displacement form found — must use displacement form
    return true;
  }

  private formatOperands(operands: Operand[]): string {
    return operands
      .map((op) => {
        switch (op.type) {
          case 'register':
            return op.name;
          case 'memory':
            return `[${op.base || ''}${op.index ? '+' + op.index : ''}${op.displacement ? '+' + op.displacement : ''}]`;
          case 'immediate':
            return String(op.value);
        }
      })
      .join(', ');
  }
}

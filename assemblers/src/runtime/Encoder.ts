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

  /**
   * Look up the operand type for a given operand name from the form's opcode matchers.
   * Returns the InstructionOperandTypes value (Register, Memory, SystemRegister) from
   * the matching field, or the InstructionDataTypes (Immediate, Displacement) from the
   * matcher itself. Returns undefined for literal register names not found in matchers.
   */
  private resolveOperandType(
    form: EncoderForm,
    operandName: string,
  ):
    | { fieldType?: number; dataType?: number; unattached?: boolean }
    | undefined {
    // First, check if any matcher field has this identifier
    for (const entry of form.opcodeEntries) {
      if (entry.kind === 'fixed') continue;
      const matcher =
        entry.kind === 'ref'
          ? this.operandDefs[entry.identifier]
          : entry.matcher;
      if (!matcher) continue;

      if (matcher.fields) {
        for (const field of matcher.fields) {
          if (field.identifier === operandName) {
            return { fieldType: field.type, dataType: matcher.type };
          }
        }
      }
    }

    // Second, check for matchers whose fields don't match ANY operand name.
    // These are "unattached" matchers (e.g., far pointer NEW_IP/NEW_CS) that
    // belong to the operand being queried if it's the only unmatched one.
    const matchedFieldIds = new Set<string>();
    for (const entry of form.opcodeEntries) {
      if (entry.kind === 'fixed') continue;
      const matcher =
        entry.kind === 'ref'
          ? this.operandDefs[entry.identifier]
          : entry.matcher;
      if (!matcher?.fields) continue;
      for (const field of matcher.fields) {
        if (form.operands.includes(field.identifier)) {
          matchedFieldIds.add(field.identifier);
        }
      }
    }

    // If this operand name isn't matched to any field, check if there are
    // unattached Immediate matchers — this indicates the operand takes immediate values.
    if (!matchedFieldIds.has(operandName)) {
      for (const entry of form.opcodeEntries) {
        if (entry.kind === 'fixed') continue;
        const matcher =
          entry.kind === 'ref'
            ? this.operandDefs[entry.identifier]
            : entry.matcher;
        if (!matcher) continue;
        if (matcher.type & DataTypes.Immediate) {
          // Check that this matcher's fields don't match any other operand
          const fieldsMatchOther = matcher.fields?.some(
            (f) =>
              form.operands.includes(f.identifier) &&
              f.identifier !== operandName,
          );
          if (!fieldsMatchOther) {
            return { dataType: matcher.type, unattached: true };
          }
        }
      }
    }

    return undefined;
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
      // which has only one operand that accepts register/memory (no immediate)
      if (
        operands.length === 2 &&
        operands[1].type === 'immediate' &&
        operands[1].value === 1 &&
        form.operands.length === 1 &&
        (() => {
          const resolved = this.resolveOperandType(form, form.operands[0]);
          return resolved && !(resolved.dataType! & DataTypes.Immediate);
        })()
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

      // Look up the operand type from the form's opcode matcher fields.
      const resolved = this.resolveOperandType(form, formOp);

      if (!resolved) {
        // No matching field in matchers — this is a literal operand (register name or constant).
        if (actualOp.type === 'register' && actualOp.name === formOp) continue;
        if (
          actualOp.type === 'immediate' &&
          typeof actualOp.value === 'number'
        ) {
          const formValue = Number(formOp);
          if (!isNaN(formValue) && actualOp.value === formValue) continue;
        }
        return false;
      }

      const { fieldType, dataType } = resolved;

      if (fieldType === OperandTypes.Register) {
        // Field is a register selector — accept register operands
        if (actualOp.type !== 'register') return false;
        if (
          form.operandSize !== undefined &&
          actualOp.size !== form.operandSize
        )
          return false;
      } else if (fieldType === OperandTypes.SystemRegister) {
        // System/segment register selector
        if (actualOp.type !== 'register') return false;
      } else if (fieldType === OperandTypes.Memory) {
        // Field is a memory operand
        if (dataType !== undefined && dataType & DataTypes.Immediate) {
          // Immediate that encodes a memory address (e.g., IMM_MEM_u16) — direct memory only
          if (actualOp.type !== 'memory') return false;
          if (actualOp.base || actualOp.index) return false;
        } else {
          // Operand-type memory (e.g., Z80 register-indirect in opcode byte)
          if (actualOp.type !== 'memory') return false;
          if (
            form.operandSize !== undefined &&
            actualOp.size !== undefined &&
            actualOp.size !== form.operandSize
          )
            return false;
        }
      } else if (dataType !== undefined && dataType & DataTypes.Immediate) {
        // Immediate value (imm, rel, level, ptr, etc.)
        if (actualOp.type !== 'immediate') return false;
        // Far pointer check only for unattached Immediates (e.g., ptr operands)
        // Regular attached immediates (e.g., RETF imm) don't require far pointer values
        if (resolved.unattached) {
          if (form.distance === 'far') {
            // Far forms with unattached Immediate require a far pointer (seg:off object)
            if (
              typeof actualOp.value !== 'object' ||
              !actualOp.value ||
              !('seg' in actualOp.value)
            )
              return false;
          } else {
            // Non-far unattached immediate forms reject far pointers
            if (
              typeof actualOp.value === 'object' &&
              actualOp.value &&
              'seg' in actualOp.value
            )
              return false;
          }
        } else {
          // Attached immediate — always reject far pointer values
          if (
            typeof actualOp.value === 'object' &&
            actualOp.value &&
            'seg' in actualOp.value
          )
            return false;
        }
      } else if (dataType !== undefined && dataType & DataTypes.Operand) {
        // Operand-type field with no specific fieldType — could be register or memory
        // depending on the specific matcher variant (e.g., ModRM rm field).
        // Accept both register and memory operands.
        if (actualOp.type !== 'register' && actualOp.type !== 'memory') {
          // Accept immediate if the form has a dual displacement/immediate matcher
          if (actualOp.type === 'immediate' && this.formHasDispImm(form)) {
            continue;
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
      } else {
        // Unknown field type — reject
        return false;
      }
    }

    return true;
  }

  private canEncode(form: EncoderForm, operands: Operand[]): boolean {
    // For relative jump forms, the value-fits check must consider the relative offset,
    // not the absolute target address. We skip the check here and let encoding handle it.
    const isRelative = form.addressing === 'relative';

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
          // Skip this check when the field has an explicit encoding (e.g., Z80
          // register-indirect modes like (HL) encoded as type: Memory).
          if (!field.encoding && form.operands.includes(field.identifier)) {
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
          // If the field also has encoding info, fall through to validate it;
          // otherwise skip (pure fixed-match field).
          if (!field.encoding) continue;
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
            const hasDispEntry = form.opcodeEntries.some((e) => {
              if (e.kind === 'fixed') return false;
              const m =
                e.kind === 'ref' ? this.operandDefs[e.identifier] : e.matcher;
              return m && (m.type & DataTypes.Displacement) !== 0;
            });
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
        } else {
          // Field type and operand type mismatch (e.g., Memory field with register operand)
          return false;
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

  /**
   * Get the ordinal position of a matcher among unattached Immediate matchers
   * (those whose field identifiers don't match any operand name).
   * Used for far pointers where the first Immediate is offset, second is segment.
   */
  private getUnattachedImmediateOrder(
    form: EncoderForm,
    targetMatcher: EncoderMatcher,
  ): number {
    let order = 0;
    for (const entry of form.opcodeEntries) {
      if (entry.kind === 'fixed') continue;
      const matcher =
        entry.kind === 'ref'
          ? this.operandDefs[entry.identifier]
          : entry.matcher;
      if (!matcher || !(matcher.type & DataTypes.Immediate)) continue;
      // Check if this is an unattached matcher (fields don't match operand names)
      const attached = matcher.fields?.some((f) =>
        form.operands.includes(f.identifier),
      );
      if (attached) continue;
      if (
        matcher === targetMatcher ||
        matcher.identifier === targetMatcher.identifier
      ) {
        return order;
      }
      order++;
    }
    return 0;
  }

  private findImmediateOperandIndex(
    form: EncoderForm,
    matcher: EncoderMatcher,
  ): number {
    const field = matcher.fields?.[0];
    const fieldId = field?.identifier ?? matcher.identifier;

    // If the field declares a specific operand type (e.g., Memory), find the operand
    // slot matching the field's identifier directly — no hardcoded name checks needed.
    if (field?.type === OperandTypes.Memory) {
      return form.operands.indexOf(fieldId);
    }

    // Field identifier now matches operand name directly (imm, rel, level, etc.)
    const directIdx = form.operands.indexOf(fieldId);
    if (directIdx !== -1) return directIdx;

    // Pure displacement comes from memory operand, not from operand list
    if (
      matcher.type & DataTypes.Displacement &&
      !(matcher.type & DataTypes.Immediate)
    )
      return -1;

    // Fallback: find first operand slot whose type resolves to Immediate
    for (let i = 0; i < form.operands.length; i++) {
      const r = this.resolveOperandType(form, form.operands[i]);
      if (r && r.dataType !== undefined && r.dataType & DataTypes.Immediate) {
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
    const isRelative = form.addressing === 'relative';

    // Find the operand value
    let value = 0;
    let needsFixup = false;
    const fieldId = matcher.fields?.[0]?.identifier ?? matcher.identifier;

    // Map matcher to operand
    let operandIdx = -1;

    if (matcher.type & DataTypes.Immediate) {
      // Field identifier now matches operand name directly (imm, rel, mem, level, etc.)
      operandIdx = form.operands.indexOf(fieldId);

      // Fallback: find first operand slot whose type resolves to Immediate
      if (operandIdx === -1) {
        for (let i = 0; i < form.operands.length; i++) {
          const r = this.resolveOperandType(form, form.operands[i]);
          if (
            r &&
            r.dataType !== undefined &&
            r.dataType & DataTypes.Immediate
          ) {
            operandIdx = i;
            break;
          }
        }
      }
    }

    if (matcher.type & DataTypes.Displacement) {
      // Displacement comes from memory operand — find the actual memory operand
      const memIdx = operands.findIndex((op) => op.type === 'memory');
      if (memIdx !== -1) {
        const mem = operands[memIdx] as MemoryOperand;
        if (typeof mem.displacement === 'number') {
          value = mem.displacement ?? 0;
        } else if (typeof mem.displacement === 'string') {
          // Label reference in displacement
          if (mem.displacement in labels) {
            value = labels[mem.displacement];
          } else {
            needsFixup = true;
            value = 0;
          }
        } else {
          value = 0;
        }
      } else if (matcher.type & DataTypes.Immediate) {
        // Dual displacement/immediate: try immediate from operand slot
        // When no memory operand is found, fall back to finding the Operand-type slot
        // (e.g., 'rm' operand receiving a bare label as immediate for LEA)
        let immIdx = operandIdx !== -1 ? operandIdx : memIdx;
        if (immIdx === -1) {
          // Find the operand slot that IS an immediate and whose form type is Operand
          immIdx = form.operands.findIndex((opName, idx) => {
            if (operands[idx]?.type !== 'immediate') return false;
            const r = this.resolveOperandType(form, opName);
            return r && r.dataType === DataTypes.Operand;
          });
        }
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
      if (typeof mem.displacement === 'number') {
        value = mem.displacement ?? 0;
      } else if (typeof mem.displacement === 'string') {
        if (mem.displacement in labels) {
          value = labels[mem.displacement];
        } else {
          needsFixup = true;
          value = 0;
        }
      } else {
        value = 0;
      }
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
          // Determine which component by this matcher's position among unattached
          // Immediate matchers: first → offset, second → segment.
          if ('seg' in operand.value && 'off' in operand.value) {
            const ptr = operand.value as { seg: number; off: number };
            const immOrder = this.getUnattachedImmediateOrder(form, matcher);
            if (immOrder === 0) {
              value = typeof ptr.off === 'number' ? ptr.off : 0;
            } else {
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
    return formOperands.indexOf(fieldIdentifier);
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
      const hasDisp = form.opcodeEntries.some((e) => {
        if (e.kind === 'fixed') return false;
        const m = e.kind === 'ref' ? this.operandDefs[e.identifier] : e.matcher;
        return m && (m.type & DataTypes.Displacement) !== 0;
      });
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

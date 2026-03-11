import type {
  DecoderMap,
  InstructionForm,
  InstructionInfo,
  OpcodeMatcher,
  Target,
} from '@machinery/core';
import { InstructionOperandTypes } from '@machinery/core';

import type {
  DecodedInstruction,
  DecodedOperand,
  RelativeOperand,
} from './types';

/**
 * Tracks a byte consumed during trie walking along with the matcher
 * that described it (if any). This lets operand decoding extract
 * register/field values from opcode bytes.
 */
interface ConsumedByte {
  value: number;
  matcher?: OpcodeMatcher;
}

export interface PrefixState {
  segmentOverride?: string;
  rep?: string;
  lock?: boolean;
}

/**
 * Decodes a byte stream into instructions using a DecoderMap trie
 * generated from processor metadata.
 *
 * All architecture-specific behavior (register names, addressing modes,
 * endianness, etc.) is derived from the Target description — no
 * assumptions are hardcoded.
 */
export class Decoder {
  private data: Uint8Array;
  private pos: number;
  private decoderMap: DecoderMap;
  private prefixMap: DecoderMap;
  private endianness: 'little' | 'big';
  private registerSizes: Map<string, number>;
  private operandsMap: Map<string, OpcodeMatcher>;

  constructor(
    data: Uint8Array,
    decoderMap: DecoderMap,
    prefixMap: DecoderMap,
    target: Target,
  ) {
    this.data = data;
    this.pos = 0;
    this.decoderMap = decoderMap;
    this.prefixMap = prefixMap;
    this.endianness = target.endianness === 'big' ? 'big' : 'little';
    this.registerSizes = this.buildRegisterSizes(target.registers);
    this.operandsMap = new Map();
    if (target.operands) {
      for (const op of target.operands) {
        this.operandsMap.set(op.identifier, op);
      }
    }
  }

  /**
   * Build a lookup map from register name (lowercase) to size in bits,
   * using the Target's register definitions including global subfields.
   */
  private buildRegisterSizes(
    registers: Target['registers'],
  ): Map<string, number> {
    const map = new Map<string, number>();
    for (const reg of registers) {
      map.set(reg.identifier.toLowerCase(), reg.size);
      if (reg.fields) {
        for (const field of reg.fields) {
          if (field.global) {
            map.set(field.identifier.toLowerCase(), field.size);
          }
        }
      }
    }
    return map;
  }

  seek(offset: number): void {
    this.pos = offset;
  }

  hasMore(): boolean {
    return this.pos < this.data.length;
  }

  get offset(): number {
    return this.pos;
  }

  decode(): DecodedInstruction | null {
    if (!this.hasMore()) return null;

    const startPos = this.pos;
    const prefix = this.consumePrefixes();

    if (this.pos >= this.data.length) {
      return this.makeUnknown(startPos);
    }

    const consumed: ConsumedByte[] = [];
    const result = this.walkTrie(this.decoderMap, consumed);
    if (!result) {
      this.pos = startPos + 1;
      return this.makeUnknown(startPos);
    }

    const { instruction, form } = result;
    const operands = this.decodeOperands(
      form,
      consumed,
      prefix.segmentOverride,
    );

    for (const op of operands) {
      if (op.type === 'relative') {
        (op as RelativeOperand).target = this.pos + op.offset;
      }
    }

    const bytes = this.data.slice(startPos, this.pos);

    return {
      address: startPos,
      bytes: new Uint8Array(bytes),
      mnemonic: instruction.identifier,
      operands,
      segmentOverride: prefix.segmentOverride,
      prefix: prefix.lock ? 'lock' : prefix.rep,
    };
  }

  private consumePrefixes(): PrefixState {
    const state: PrefixState = {};
    while (this.pos < this.data.length) {
      const byte = this.data[this.pos];
      const prefixMatch = this.tryPrefixMatch(byte);
      if (!prefixMatch) break;
      this.pos++;
      const inst = prefixMatch.instruction;
      if (!inst) break;
      const form = prefixMatch.form;
      if (form?.segmentOverride) {
        state.segmentOverride = form.segmentOverride.toLowerCase();
      } else {
        const id = inst.identifier.toLowerCase();
        if (id === 'lock') state.lock = true;
        else if (id.startsWith('rep')) state.rep = id;
      }
    }
    return state;
  }

  private tryPrefixMatch(byte: number): DecoderMap | null {
    if (!this.prefixMap.exact) return null;
    const node = this.prefixMap.exact[byte];
    if (!node || !node.instruction) return null;
    return node;
  }

  /**
   * Walk the decoder trie, consuming bytes and recording them with
   * their associated matchers for later operand extraction.
   */
  private walkTrie(
    node: DecoderMap,
    consumed: ConsumedByte[],
  ): {
    instruction: InstructionInfo;
    form: InstructionForm;
  } | null {
    if (this.pos >= this.data.length) {
      if (node.instruction && node.form) {
        return { instruction: node.instruction, form: node.form };
      }
      return null;
    }

    const byte = this.data[this.pos];

    // Try exact match first
    if (node.exact) {
      const next = node.exact[byte];
      if (next) {
        if (next.instruction && next.form) {
          consumed.push({ value: byte });
          this.pos++;
          return { instruction: next.instruction, form: next.form };
        }
        if (next.exact || next.partial) {
          consumed.push({ value: byte });
          this.pos++;
          return this.walkTrie(next, consumed);
        }
      }
    }

    // Try partial (masked) matches
    if (node.partial) {
      for (const partial of node.partial) {
        if ((byte & partial.mask) === partial.and) {
          const matcher =
            typeof partial.matcher === 'object' ? partial.matcher : undefined;

          if (partial.map.instruction && partial.map.form) {
            // Terminal: consume the byte and record its matcher
            consumed.push({ value: byte, matcher });
            this.pos++;
            return {
              instruction: partial.map.instruction,
              form: partial.map.form,
            };
          }
          // Non-terminal: consume and continue
          consumed.push({ value: byte, matcher });
          this.pos++;
          return this.walkTrie(partial.map as DecoderMap, consumed);
        }
      }
    }

    // Node itself is terminal (open-ended matcher)
    if (node.instruction && node.form) {
      return { instruction: node.instruction, form: node.form };
    }

    return null;
  }

  /**
   * Decode operands using the form definition and the bytes consumed during trie walking.
   */
  private decodeOperands(
    form: InstructionForm,
    consumed: ConsumedByte[],
    segmentOverride: string | undefined,
  ): DecodedOperand[] {
    const operands: DecodedOperand[] = [];
    const operandNames = form.operands || [];
    const operandSize = form.operandSize || 16;

    // Find a ModRM-like consumed byte (one with a matcher named 'ModRM')
    const modrmEntry = consumed.find((c) => c.matcher?.name === 'ModRM');
    // If we don't have one from consumed bytes, check if the form expects one
    // and read it from the stream
    let modrm: number | null = modrmEntry?.value ?? null;
    let modrmMatcher: OpcodeMatcher | undefined = modrmEntry?.matcher as
      | OpcodeMatcher
      | undefined;

    if (modrm === null && this.formNeedsModRM(form)) {
      modrm = this.readByte();
      // Resolve the matcher from the form's opcode entries
      modrmMatcher = this.resolveModRMMatcher(form);
    }

    // Find inline opcode matchers that embed register info
    // (e.g., MOV 0xB0+r where bits 0-2 encode register)
    const inlineRegEntry = consumed.find(
      (c) =>
        c.matcher &&
        c.matcher.name !== 'ModRM' &&
        c.matcher.fields?.some(
          (f) =>
            f.match === undefined &&
            'type' in f &&
            f.type === InstructionOperandTypes.Register,
        ),
    );

    // Collect trailing opcode identifiers for IMM/DISP
    const suffixIds = this.getTrailingSuffixIds(form);

    for (const opName of operandNames) {
      const operand = this.decodeSingleOperand(
        opName,
        operandSize,
        form,
        modrm,
        modrmMatcher,
        segmentOverride,
        suffixIds,
        inlineRegEntry,
      );
      if (operand) operands.push(operand);
    }

    return operands;
  }

  /**
   * Resolve the ModRM OpcodeMatcher from the form's opcode entries,
   * looking up string references in the target's operands map.
   */
  private resolveModRMMatcher(
    form: InstructionForm,
  ): OpcodeMatcher | undefined {
    for (const entry of form.opcode) {
      if (typeof entry === 'string' && entry.startsWith('ModRM')) {
        return this.operandsMap.get(entry);
      }
      if (
        typeof entry === 'object' &&
        'name' in entry &&
        (entry as OpcodeMatcher).name === 'ModRM'
      ) {
        return entry as OpcodeMatcher;
      }
    }
    return undefined;
  }

  private decodeSingleOperand(
    name: string,
    size: number,
    form: InstructionForm,
    modrm: number | null,
    modrmMatcher: OpcodeMatcher | undefined,
    segmentOverride: string | undefined,
    suffixIds: string[],
    inlineRegEntry: ConsumedByte | undefined,
  ): DecodedOperand | null {
    // Handle fixed register operand names (e.g., 'AL', 'AX', 'CL', 'DX')
    if (this.isFixedRegisterName(name)) {
      const regSize = this.getRegisterSize(name);
      return { type: 'register', name: name.toLowerCase(), size: regSize };
    }

    switch (name) {
      case 'rm': {
        // If there's an inline register matcher (like MOV 0xB0+r), use it
        if (inlineRegEntry && modrm === null) {
          return this.decodeInlineRegister(inlineRegEntry, size);
        }
        if (modrm !== null && modrmMatcher) {
          return this.decodeRMFromMatcher(
            modrm,
            modrmMatcher,
            size,
            segmentOverride,
            suffixIds,
          );
        }
        return null;
      }

      case 'reg':
      case 'sreg':
      case 'seg': {
        if (modrm !== null && modrmMatcher) {
          return this.decodeRegFromMatcher(modrm, modrmMatcher, 'reg');
        }
        return null;
      }

      case 'ptr': {
        // Far pointer: offset, segment — sizes come from the form's inline
        // opcode entries (two consecutive Immediate-typed matchers)
        const ptrSizes = this.getFarPointerSizes(form);
        const offset = this.readValue(ptrSizes[0]);
        const segment = this.readValue(ptrSizes[1]);
        return { type: 'far_pointer', segment, offset };
      }

      case 'level': {
        // ENTER nesting level (imm8)
        const val = this.readByte();
        return { type: 'immediate', value: val, size: 8, signed: false };
      }

      case 'imm': {
        const immId = suffixIds.find((id) => id.startsWith('IMM_'));
        if (!immId) return null;
        return this.readImmediateOperand(immId);
      }

      case 'mem': {
        // Direct memory operand (MOV AL, [addr])
        const dispId = suffixIds.find((id) => id.startsWith('IMM_'));
        if (dispId) {
          const dispSize = this.parseFieldSize(dispId);
          const addr = this.readValue(dispSize);
          return {
            type: 'memory',
            displacement: addr,
            displacementSize: dispSize,
            size,
            segment: segmentOverride,
            direct: true,
          };
        }
        return null;
      }

      case 'rel': {
        const relId = suffixIds.find((id) => id.startsWith('REL_'));
        if (relId) {
          const relSize = this.parseFieldSize(relId);
          const raw = this.readValue(relSize);
          const offset = this.signExtend(raw, relSize);
          return { type: 'relative', offset, size: relSize, target: 0 };
        }
        // Fall through to IMM-based rel handling
        const relImmId = suffixIds.find((id) => id.startsWith('IMM_'));
        if (!relImmId) return null;
        const relSize2 = this.parseFieldSize(relImmId);
        const raw2 = this.readValue(relSize2);
        const offset2 = this.signExtend(raw2, relSize2);
        return { type: 'relative', offset: offset2, size: relSize2, target: 0 };
      }

      case '(imm)': {
        // Direct memory from immediate (e.g., LD (nn), A / OUT (n), A)
        const immId = suffixIds.find(
          (id) => id.startsWith('IMM_') || id.startsWith('PORT_'),
        );
        if (!immId) return null;
        const immSize = this.parseFieldSize(immId);
        const addr = this.readValue(immSize);
        return {
          type: 'memory',
          displacement: addr,
          displacementSize: immSize,
          size,
          direct: true,
        };
      }

      default:
        return this.decodeLiteralOperand(name, size);
    }
  }

  /**
   * Decode the 'rm' operand using the ModRM matcher's field definitions.
   * The rm field's type determines whether this is a register or memory operand:
   * - Register type: look up register name from encoding
   * - Memory type: parse encoding for base/index, read displacement from suffix
   * - Fixed match (no type): direct addressing, read displacement from suffix
   */
  private decodeRMFromMatcher(
    modrm: number,
    matcher: OpcodeMatcher,
    operandSize: number,
    segmentOverride: string | undefined,
    suffixIds: string[],
  ): DecodedOperand | null {
    const rmField = matcher.fields?.find((f) => f.identifier === 'rm');
    if (!rmField) return null;

    const fieldMask = (1 << rmField.size) - 1;
    const rmValue = (modrm >> rmField.offset) & fieldMask;

    // Check if the rm field represents a register (mod=11 equivalent)
    if (
      'type' in rmField &&
      rmField.type === InstructionOperandTypes.Register
    ) {
      const regName = rmField.encoding?.[rmValue];
      if (!regName) return null;
      const regSize = this.getRegisterSize(regName);
      return { type: 'register', name: regName.toLowerCase(), size: regSize };
    }

    // Check if the rm field represents memory addressing
    if ('type' in rmField && rmField.type === InstructionOperandTypes.Memory) {
      const encoding = rmField.encoding?.[rmValue];
      const { base, index } = this.parseMemoryEncoding(
        encoding as string | null,
      );
      const { displacement, displacementSize } =
        this.readDisplacementFromSuffix(suffixIds);

      return {
        type: 'memory',
        base,
        index,
        displacement,
        displacementSize,
        size: operandSize,
        segment: segmentOverride,
        direct: false,
      };
    }

    // rm field has a fixed match but no type — direct addressing
    // (e.g., mod=00, rm=110 on x86 which is a direct memory address)
    if (
      rmField.match !== undefined &&
      !('type' in rmField && rmField.type !== undefined)
    ) {
      const { displacement, displacementSize } =
        this.readDisplacementFromSuffix(suffixIds);
      return {
        type: 'memory',
        displacement,
        displacementSize,
        size: operandSize,
        segment: segmentOverride,
        direct: true,
      };
    }

    return null;
  }

  /**
   * Decode the 'reg' (or 'sreg'/'seg') operand from the ModRM matcher's
   * reg field encoding. The field's encoding array maps bit values to
   * register names.
   */
  private decodeRegFromMatcher(
    modrm: number,
    matcher: OpcodeMatcher,
    fieldId: string,
  ): DecodedOperand | null {
    const regField = matcher.fields?.find((f) => f.identifier === fieldId);
    if (!regField) return null;

    const fieldMask = (1 << regField.size) - 1;
    const regValue = (modrm >> regField.offset) & fieldMask;

    if (regField.encoding) {
      const regName = regField.encoding[regValue];
      if (!regName) return null;
      const regSize = this.getRegisterSize(regName);
      return { type: 'register', name: regName.toLowerCase(), size: regSize };
    }

    return null;
  }

  /**
   * Parse a memory encoding string (e.g., 'BX + SI') into base and index
   * register names.
   */
  private parseMemoryEncoding(encoding: string | null): {
    base?: string;
    index?: string;
  } {
    if (!encoding) return {};
    const parts = encoding.split(/\s*\+\s*/);
    return {
      base: parts[0]?.toLowerCase(),
      index: parts[1]?.toLowerCase(),
    };
  }

  /**
   * Read displacement bytes from the stream based on DISP_ suffix entries
   * in the form's opcode array.
   */
  private readDisplacementFromSuffix(suffixIds: string[]): {
    displacement: number;
    displacementSize: number;
  } {
    const dispId = suffixIds.find((id) => id.startsWith('DISP_'));
    if (!dispId) return { displacement: 0, displacementSize: 0 };

    const dispSize = this.parseFieldSize(dispId);
    const signed = dispId.includes('_i');
    const raw = this.readValue(dispSize);
    const displacement = signed ? this.signExtend(raw, dispSize) : raw;

    return { displacement, displacementSize: dispSize };
  }

  private decodeInlineRegister(
    entry: ConsumedByte,
    defaultSize: number,
  ): DecodedOperand | null {
    if (!entry.matcher?.fields) return null;
    for (const field of entry.matcher.fields) {
      if (
        field.match === undefined &&
        'type' in field &&
        field.type === InstructionOperandTypes.Register &&
        'encoding' in field &&
        field.encoding
      ) {
        const fieldMask = (1 << field.size) - 1;
        const value = (entry.value >> field.offset) & fieldMask;
        const regName = field.encoding[value];
        if (regName) {
          const regSize = this.getRegisterSize(regName);
          return {
            type: 'register',
            name: regName.toLowerCase(),
            size: regSize || defaultSize,
          };
        }
      }
    }
    return null;
  }

  private formNeedsModRM(form: InstructionForm): boolean {
    for (let i = 1; i < form.opcode.length; i++) {
      const entry = form.opcode[i];
      if (typeof entry === 'string' && entry.startsWith('ModRM')) return true;
      if (
        typeof entry === 'object' &&
        'name' in entry &&
        (entry as OpcodeMatcher).name === 'ModRM'
      )
        return true;
    }
    return false;
  }

  /**
   * Extract the sizes of the two immediate fields in a far pointer form's
   * opcode array (offset size, then segment size).
   */
  private getFarPointerSizes(form: InstructionForm): [number, number] {
    const sizes: number[] = [];
    for (const entry of form.opcode) {
      if (typeof entry === 'object' && 'size' in entry) {
        sizes.push((entry as OpcodeMatcher).size);
      }
    }
    // Default to 16-bit offset, 16-bit segment
    return [sizes[0] ?? 16, sizes[1] ?? 16];
  }

  private getTrailingSuffixIds(form: InstructionForm): string[] {
    const result: string[] = [];
    for (const entry of form.opcode) {
      const id =
        typeof entry === 'string'
          ? entry
          : typeof entry === 'object' && 'identifier' in entry
            ? (entry as OpcodeMatcher).identifier
            : '';
      if (
        id.startsWith('IMM_') ||
        id.startsWith('DISP_') ||
        id.startsWith('REL_') ||
        id.startsWith('PORT_')
      ) {
        result.push(id);
      }
    }
    return result;
  }

  private readImmediateOperand(id: string): DecodedOperand {
    const size = this.parseFieldSize(id);
    const signed = id.includes('_i');
    const raw = this.readValue(size);
    return {
      type: 'immediate',
      value: signed ? this.signExtend(raw, size) : raw,
      size,
      signed,
    };
  }

  /**
   * Check whether a name refers to a known register by looking it up
   * in the register size map built from the Target.
   */
  private isFixedRegisterName(name: string): boolean {
    return this.registerSizes.has(name.toLowerCase());
  }

  /**
   * Look up a register's size from the Target's register definitions.
   */
  private getRegisterSize(name: string): number {
    return this.registerSizes.get(name.toLowerCase()) ?? 16;
  }

  /**
   * Handle literal operand names that aren't field references or keywords.
   * Covers:
   *   - '(REG)' — register indirect memory (e.g., '(HL)', '(BC)', '(SP)', '(C)')
   *   - Numeric literals ('0', '1', '0x00', '0x38')
   *   - Quoted register names that didn't match isFixedRegisterName (e.g., "AF'")
   */
  private decodeLiteralOperand(
    name: string,
    size: number,
  ): DecodedOperand | null {
    // Register indirect: (REG)
    const indirectMatch = name.match(/^\((\w+)\)$/);
    if (indirectMatch) {
      const reg = indirectMatch[1];
      if (this.isFixedRegisterName(reg)) {
        return {
          type: 'memory',
          base: reg.toLowerCase(),
          displacement: 0,
          displacementSize: 0,
          size,
          direct: false,
        };
      }
    }

    // Numeric literal (e.g., '0', '1', '2', '0x00', '0x38')
    if (/^(0x[\da-fA-F]+|\d+)$/.test(name)) {
      const value = name.startsWith('0x')
        ? parseInt(name, 16)
        : parseInt(name, 10);
      return { type: 'immediate', value, size: 8, signed: false };
    }

    // Fallback: try as register name (handles shadow registers like "AF'")
    const stripped = name.replace(/'/g, '');
    if (this.isFixedRegisterName(stripped) || this.isFixedRegisterName(name)) {
      const regSize =
        this.getRegisterSize(stripped) || this.getRegisterSize(name) || 16;
      return { type: 'register', name: name.toLowerCase(), size: regSize };
    }

    return null;
  }

  private parseFieldSize(id: string): number {
    const match = id.match(/(\d+)/);
    if (match) {
      return parseInt(match[1], 10);
    }
    return 8;
  }

  private readByte(): number {
    if (this.pos >= this.data.length) return 0;
    return this.data[this.pos++];
  }

  /**
   * Read a multi-byte value from the stream, respecting the target's
   * endianness.
   */
  private readValue(bits: number): number {
    if (bits <= 8) return this.readByte();
    const byteCount = Math.ceil(bits / 8);
    if (this.endianness === 'little') {
      let result = 0;
      for (let i = 0; i < byteCount; i++) {
        result |= this.readByte() << (i * 8);
      }
      return result;
    } else {
      let result = 0;
      for (let i = 0; i < byteCount; i++) {
        result = (result << 8) | this.readByte();
      }
      return result;
    }
  }

  /**
   * Sign-extend a value of the given bit width.
   */
  private signExtend(val: number, bits: number): number {
    const signBit = 1 << (bits - 1);
    if (val & signBit) {
      return val - (1 << bits);
    }
    return val;
  }

  private makeUnknown(startPos: number): DecodedInstruction {
    if (this.pos <= startPos) this.pos = startPos + 1;
    const bytes = this.data.slice(startPos, this.pos);
    return {
      address: startPos,
      bytes: new Uint8Array(bytes),
      mnemonic: '(bad)',
      operands: [],
    };
  }
}

/**
 * Iterator that yields decoded instructions from a binary.
 */
export function* disassemble(
  data: Uint8Array,
  decoderMap: DecoderMap,
  prefixMap: DecoderMap,
  target: Target,
): Generator<DecodedInstruction> {
  const decoder = new Decoder(data, decoderMap, prefixMap, target);
  while (decoder.hasMore()) {
    const instr = decoder.decode();
    if (!instr) break;
    yield instr;
  }
}

import type {
  Program,
  ASTNode,
  DirectiveNode,
  Operand,
  ExpressionValue,
  ResolvableValue,
  ExpressionNode,
  FarPointer,
  ImmediateOperand,
} from './ast';
import type { EncoderMatcher, EncoderForm } from './Encoder';
import { Encoder } from './Encoder';

export interface AssemblerOptions {
  origin?: number;
}

export interface AssembleResult {
  binary: Uint8Array;
  labels: Record<string, number>;
  origin: number;
}

export class Assembler {
  private encoder: Encoder;
  private parse: (input: string) => Program;
  private prefixBytes: Record<string, number>;

  private endianness: 'little' | 'big';
  private alignmentFill: number;

  constructor(
    parser: { parse: (input: string) => Program },
    operandDefinitions: Record<string, EncoderMatcher>,
    encoderForms: EncoderForm[],
    segmentOverridePrefixes?: Record<string, number[]>,
    prefixBytes?: Record<string, number>,
    endianness?: 'little' | 'big',
    alignmentFill?: number,
  ) {
    this.parse = parser.parse.bind(parser);
    this.endianness = endianness || 'little';
    this.alignmentFill = alignmentFill ?? 0x00;
    this.encoder = new Encoder(
      operandDefinitions,
      encoderForms,
      segmentOverridePrefixes,
      this.endianness,
    );
    this.prefixBytes = prefixBytes || {};
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

  assemble(source: string, options?: AssemblerOptions): AssembleResult {
    const ast = this.parse(source);
    return this.assembleAST(ast, options);
  }

  assembleAST(ast: Program, options?: AssemblerOptions): AssembleResult {
    // Flatten any arrays in the AST (from label_directive rules that return [LabelNode, DirectiveNode])
    const flatAst: ASTNode[] = [];
    for (const node of ast) {
      if (Array.isArray(node)) {
        flatAst.push(...(node as ASTNode[]));
      } else {
        flatAst.push(node);
      }
    }

    let origin = options?.origin ?? 0;
    const constants: Record<string, number> = {};

    // Pre-scan for EQU directives and ORG
    for (const node of flatAst) {
      if (node.type === 'directive') {
        if (node.name === 'ORG' && node.args.length === 1) {
          const val = this.evaluateExpression(node.args[0], {}, constants, 0);
          if (val !== undefined) {
            origin = val;
          }
        }
      }
    }

    // Pre-scan EQU in multiple passes to resolve forward references between EQUs
    let changed = true;
    for (let pass = 0; pass < 10 && changed; pass++) {
      changed = false;
      for (const node of flatAst) {
        if (
          node.type === 'directive' &&
          node.name === 'EQU' &&
          node.args.length === 2
        ) {
          const name = node.args[0] as string;
          if (name in constants) continue;
          const val = this.evaluateExpression(node.args[1], {}, constants, 0);
          if (val !== undefined) {
            constants[name] = val;
            changed = true;
          }
        }
      }
    }

    // First pass: calculate sizes and collect label addresses
    const labels: Record<string, number> = { ...constants };

    let nodeInfos: { node: ASTNode; offset: number; size: number }[] = [];
    let prevSizes: number[] = [];

    // Run size estimation loop: first pass collects labels, subsequent passes
    // re-estimate sizes as labels become available and check for convergence.
    for (let iteration = 0; iteration < 10; iteration++) {
      let sizeChanged = false;
      nodeInfos = [];
      let offset = origin;
      let currentGlobalLabel = '';

      // Reset labels to constants only
      for (const key of Object.keys(labels)) {
        if (!(key in constants)) delete labels[key];
      }

      for (let nodeIdx = 0; nodeIdx < flatAst.length; nodeIdx++) {
        const node = flatAst[nodeIdx];
        if (node.type === 'label') {
          let labelName = node.name;
          if (labelName.startsWith('.')) {
            labelName = currentGlobalLabel + labelName;
          } else {
            currentGlobalLabel = labelName;
          }
          labels[labelName] = offset;
          nodeInfos.push({ node, offset, size: 0 });
        } else if (node.type === 'directive') {
          const size = this.directiveSize(node, labels, constants, offset);
          nodeInfos.push({ node, offset, size });
          offset += size;
        } else if (node.type === 'instruction') {
          const { prefixByte, mnemonic: actualMnemonic } =
            this.splitPrefixMnemonic(node.mnemonic);

          const resolved = this.fixupOperands(
            actualMnemonic,
            this.resolveOperandLabels(
              node.operands,
              labels,
              constants,
              offset,
              currentGlobalLabel,
            ),
          );
          let size: number;

          // Try actual encoding to get exact size (prefers smallest encoding)
          try {
            const prefixSize = prefixByte !== null ? 1 : 0;
            const result = this.encoder.encode(
              actualMnemonic,
              resolved,
              labels,
              offset + prefixSize,
            );
            if (result.needsFixup) {
              // Unresolved forward reference — use max size estimate (pessimistic)
              throw new Error('fixup');
            }
            size = result.bytes.length + prefixSize;
          } catch {
            // Encoding failed or has fixup — use estimate (e.g., unresolved forward refs)
            size =
              this.encoder.estimateSize(actualMnemonic, resolved) +
              (prefixByte !== null ? 1 : 0);
          }

          nodeInfos.push({ node, offset, size });
          offset += size;
        }
      }

      // Check if any sizes changed from the previous iteration
      if (iteration > 0) {
        const newSizes = nodeInfos.map((info) => info.size);
        if (newSizes.length === prevSizes.length) {
          for (let i = 0; i < newSizes.length; i++) {
            if (newSizes[i] !== prevSizes[i]) {
              sizeChanged = true;
              break;
            }
          }
        } else {
          sizeChanged = true;
        }
      }

      // Save sizes for next iteration comparison
      prevSizes = nodeInfos.map((info) => info.size);

      // After collecting labels, resolve EQU directives that reference labels
      // (e.g., "stack_top equ stack_buf + 2048" where stack_buf is a label)
      let equChanged = true;
      for (let eqPass = 0; eqPass < 10 && equChanged; eqPass++) {
        equChanged = false;
        for (const node of flatAst) {
          if (
            node.type === 'directive' &&
            node.name === 'EQU' &&
            node.args.length === 2
          ) {
            const name = node.args[0] as string;
            const val = this.evaluateExpression(
              node.args[1],
              labels,
              constants,
              0,
            );
            if (val !== undefined && labels[name] !== val) {
              labels[name] = val;
              constants[name] = val;
              equChanged = true;
            }
          }
        }
      }

      // After the first iteration, we have all labels. If no sizes changed, we're stable.
      if (iteration > 0 && !sizeChanged) break;
    }

    // Final passes: re-estimate sizes using the complete label set.
    // Forward references may have caused overestimation during sequential processing.
    // Uses snapshot approach: evaluate all instructions against the pre-pass layout,
    // then apply all changes at once and rebuild the layout for the next pass.
    for (let finalPass = 0; finalPass < 20; finalPass++) {
      let anyChanged = false;

      // Snapshot: evaluate each instruction at its current offset using current labels
      const newSizes: (number | null)[] = new Array(nodeInfos.length).fill(
        null,
      );
      let currentGlobalLabel = '';
      for (let i = 0; i < nodeInfos.length; i++) {
        const info = nodeInfos[i];
        const node = info.node;

        if (node.type === 'label') {
          const labelName = node.name;
          if (!labelName.startsWith('.')) currentGlobalLabel = labelName;
        } else if (node.type === 'instruction') {
          const { prefixByte, mnemonic: actualMnemonic } =
            this.splitPrefixMnemonic(node.mnemonic);
          const resolved = this.fixupOperands(
            actualMnemonic,
            this.resolveOperandLabels(
              node.operands,
              labels,
              constants,
              info.offset,
              currentGlobalLabel,
            ),
          );
          try {
            const prefixSize = prefixByte !== null ? 1 : 0;
            const result = this.encoder.encode(
              actualMnemonic,
              resolved,
              labels,
              info.offset + prefixSize,
            );
            newSizes[i] = result.bytes.length + prefixSize;
          } catch {
            // Keep current size
          }
        }
      }

      // Apply changes
      for (let i = 0; i < nodeInfos.length; i++) {
        if (newSizes[i] !== null && newSizes[i] !== nodeInfos[i].size) {
          nodeInfos[i].size = newSizes[i]!;
          anyChanged = true;
        }
      }

      if (!anyChanged) break;

      // Rebuild offsets and labels from the updated sizes
      let offset = origin;
      currentGlobalLabel = '';
      for (let i = 0; i < nodeInfos.length; i++) {
        const info = nodeInfos[i];
        const node = info.node;
        info.offset = offset;
        if (node.type === 'label') {
          let labelName = node.name;
          if (labelName.startsWith('.')) {
            labelName = currentGlobalLabel + labelName;
          } else {
            currentGlobalLabel = labelName;
          }
          labels[labelName] = offset;
        } else {
          if (node.type === 'directive') {
            const size = this.directiveSize(node, labels, constants, offset);
            if (size !== info.size) {
              info.size = size;
            }
          }
          offset += info.size;
        }
      }

      // Resolve EQUs with updated labels
      for (const node of flatAst) {
        if (
          node.type === 'directive' &&
          node.name === 'EQU' &&
          node.args.length === 2
        ) {
          const name = node.args[0] as string;
          const val = this.evaluateExpression(
            node.args[1],
            labels,
            constants,
            0,
          );
          if (val !== undefined) {
            labels[name] = val;
            constants[name] = val;
          }
        }
      }
    }

    // Second pass: encode with resolved labels
    const output: number[] = [];
    let offset2 = origin;
    let currentGlobalLabel2 = '';

    for (let infoIdx = 0; infoIdx < nodeInfos.length; infoIdx++) {
      const { node } = nodeInfos[infoIdx];

      if (node.type === 'label') {
        let labelName = node.name;
        if (labelName.startsWith('.')) {
          labelName = currentGlobalLabel2 + labelName;
        } else {
          currentGlobalLabel2 = labelName;
        }
        // Labels produce no bytes
        continue;
      }

      if (node.type === 'directive') {
        const bytes = this.encodeDirective(node, labels, constants, offset2);
        output.push(...bytes);
        offset2 += bytes.length;
        continue;
      }

      if (node.type === 'instruction') {
        const { prefixByte, mnemonic: actualMnemonic } =
          this.splitPrefixMnemonic(node.mnemonic);

        const resolved = this.fixupOperands(
          actualMnemonic,
          this.resolveOperandLabels(
            node.operands,
            labels,
            constants,
            offset2,
            currentGlobalLabel2,
          ),
        );

        // Add prefix byte if present
        if (prefixByte !== null) {
          output.push(prefixByte);
          offset2 += 1;
        }

        const expectedSize =
          nodeInfos[infoIdx].size - (prefixByte !== null ? 1 : 0);
        const result = this.encoder.encode(
          actualMnemonic,
          resolved,
          labels,
          offset2,
          expectedSize,
        );

        // Handle fixups for forward references
        if (result.needsFixup) {
          const fixup = result.needsFixup;
          // Try to resolve now that we have all labels
          for (const op of node.operands) {
            if (op.type === 'immediate') {
              const resolvedValue = this.resolveImmediateForFixup(
                op.value,
                labels,
                constants,
                fixup.relative,
                offset2,
                result.bytes.length,
                currentGlobalLabel2,
              );
              if (resolvedValue !== undefined) {
                const patchBytes = this.emitValue(resolvedValue, fixup.size);
                for (let i = 0; i < fixup.size; i++) {
                  result.bytes[fixup.offset + i] = patchBytes[i];
                }
              }
            }
          }
        }

        output.push(...result.bytes);
        offset2 += result.bytes.length;
      }
    }

    return {
      binary: new Uint8Array(output),
      labels,
      origin,
    };
  }

  evaluateExpression(
    expr: ExpressionValue | number | string | undefined,
    labels: Record<string, number>,
    constants: Record<string, number>,
    currentOffset: number,
  ): number | undefined {
    if (expr === undefined || expr === null) return undefined;
    if (typeof expr === 'number') return expr;
    if (typeof expr === 'string') {
      if (expr === '$') return currentOffset;
      if (expr in constants) return constants[expr];
      if (expr in labels) return labels[expr];
      return undefined;
    }
    if (
      typeof expr === 'object' &&
      expr !== null &&
      'type' in expr &&
      (expr as ExpressionNode).type === 'expression'
    ) {
      const node = expr as ExpressionNode;
      const left = this.evaluateExpression(
        node.left,
        labels,
        constants,
        currentOffset,
      );
      const right = this.evaluateExpression(
        node.right,
        labels,
        constants,
        currentOffset,
      );
      if (left === undefined || right === undefined) return undefined;
      switch (node.op) {
        case '+':
          return left + right;
        case '-':
          return left - right;
        case '*':
          return left * right;
        case '/':
          return right !== 0 ? Math.floor(left / right) : undefined;
        case '&':
          return left & right;
        case '|':
          return left | right;
        case '~':
          return ~right;
      }
    }
    return undefined;
  }

  private resolveImmediateForFixup(
    value: ImmediateOperand['value'],
    labels: Record<string, number>,
    constants: Record<string, number>,
    relative: boolean | undefined,
    offset: number,
    instrLen: number,
    currentGlobalLabel?: string,
  ): number | undefined {
    if (typeof value === 'string') {
      let labelKey = value;
      if (
        !(labelKey in labels) &&
        labelKey.startsWith('.') &&
        currentGlobalLabel
      ) {
        labelKey = currentGlobalLabel + labelKey;
      }
      if (labelKey in labels) {
        let v = labels[labelKey];
        if (relative) {
          v = v - (offset + instrLen);
        }
        return v;
      }
    }
    if (
      typeof value === 'object' &&
      value !== null &&
      'type' in value &&
      (value as ExpressionNode).type === 'expression'
    ) {
      const evaluated = this.evaluateExpression(
        value as ExpressionValue,
        labels,
        constants,
        offset,
      );
      if (evaluated !== undefined) {
        if (relative) {
          return evaluated - (offset + instrLen);
        }
        return evaluated;
      }
    }
    return undefined;
  }

  /**
   * Split a compound prefix mnemonic like "REP STOSB" into { prefix: "REP", mnemonic: "STOSB" }.
   * Returns null prefix if no prefix found.
   */
  private splitPrefixMnemonic(mnemonic: string): {
    prefix: string | null;
    mnemonic: string;
    prefixByte: number | null;
  } {
    const spaceIdx = mnemonic.indexOf(' ');
    if (spaceIdx !== -1) {
      const prefix = mnemonic.substring(0, spaceIdx).toUpperCase();
      const inner = mnemonic.substring(spaceIdx + 1).toUpperCase();
      const prefixByte = this.prefixBytes[prefix];
      if (prefixByte !== undefined) {
        return { prefix, mnemonic: inner, prefixByte };
      }
    }
    return { prefix: null, mnemonic, prefixByte: null };
  }

  private fixupOperands(_mnemonic: string, operands: Operand[]): Operand[] {
    return operands;
  }

  private resolveOperandLabels(
    operands: Operand[],
    labels: Record<string, number>,
    constants: Record<string, number>,
    currentOffset?: number,
    currentGlobalLabel?: string,
  ): Operand[] {
    const offset = currentOffset ?? 0;
    return operands.map((op) => {
      if (op.type === 'immediate') {
        const resolved = this.resolveExpressionValue(
          op.value,
          labels,
          constants,
          offset,
          currentGlobalLabel,
        );
        if (resolved !== op.value) {
          return { ...op, value: resolved };
        }
        return op;
      }
      if (op.type === 'memory') {
        if (op.displacement !== undefined) {
          const resolved = this.resolveExpressionValue(
            op.displacement,
            labels,
            constants,
            offset,
            currentGlobalLabel,
          );
          if (resolved !== op.displacement) {
            return {
              ...op,
              displacement:
                typeof resolved === 'number' ? resolved : op.displacement,
            };
          }
        }
        return op;
      }
      return op;
    });
  }

  private resolveExpressionValue(
    value: ResolvableValue,
    labels: Record<string, number>,
    constants: Record<string, number>,
    currentOffset: number,
    currentGlobalLabel?: string,
  ): ResolvableValue {
    if (typeof value === 'number') return value;
    if (typeof value === 'string') {
      if (value === '$') return currentOffset;
      if (value in constants) return constants[value];
      if (value in labels) return labels[value];
      // Try resolving local labels with global prefix
      if (value.startsWith('.') && currentGlobalLabel) {
        const fullName = currentGlobalLabel + value;
        if (fullName in labels) return labels[fullName];
      }
      return value; // Unresolved label reference
    }
    if (typeof value === 'object' && value !== null) {
      if ('type' in value && (value as ExpressionNode).type === 'expression') {
        const evaluated = this.evaluateExpression(
          value as ExpressionValue,
          labels,
          constants,
          currentOffset,
        );
        if (evaluated !== undefined) return evaluated;
        return value; // Cannot fully evaluate yet
      }
      // Far pointer: { seg, off }
      if ('seg' in value && 'off' in value) {
        const fp = value as FarPointer;
        const seg = this.resolveExpressionValue(
          fp.seg,
          labels,
          constants,
          currentOffset,
        );
        const off = this.resolveExpressionValue(
          fp.off,
          labels,
          constants,
          currentOffset,
        );
        if (seg !== fp.seg || off !== fp.off) {
          return { seg, off } as FarPointer;
        }
      }
    }
    return value;
  }

  private directiveSize(
    node: DirectiveNode,
    labels?: Record<string, number>,
    constants?: Record<string, number>,
    currentOffset?: number,
  ): number {
    const evalArg = (arg: ExpressionValue): number => {
      if (typeof arg === 'number') return arg;
      const val = this.evaluateExpression(
        arg,
        labels || {},
        constants || {},
        currentOffset || 0,
      );
      return val ?? 0;
    };

    switch (node.name) {
      case 'ORG':
      case 'EQU':
      case 'BITS':
      case 'SECTION': {
        // nasm aligns sections to 4 bytes in flat binary mode
        const sectionName = (
          typeof node.args[0] === 'string' ? node.args[0] : ''
        ).toLowerCase();
        if (sectionName === '.data' || sectionName === '.bss') {
          const currentPos = currentOffset || 0;
          const remainder = currentPos % 4;
          return remainder === 0 ? 0 : 4 - remainder;
        }
        return 0;
      }
      case 'ALIGN': {
        const alignment = evalArg(node.args[0]);
        if (alignment <= 0) return 0;
        const currentPos = currentOffset || 0;
        const remainder = currentPos % alignment;
        return remainder === 0 ? 0 : alignment - remainder;
      }
      case 'DB':
        return node.args.reduce((total: number, arg) => {
          if (typeof arg === 'string') return total + arg.length;
          return total + 1;
        }, 0);
      case 'DW':
        return node.args.length * 2;
      case 'DD':
        return node.args.length * 4;
      case 'TIMES': {
        // args: [count, 'DB'|'DW'|'DD', ...values]
        const count = evalArg(node.args[0]);
        const kind = node.args[1] as string;
        const valueArgs = node.args.slice(2);
        let perItem = 0;
        if (kind === 'DB') {
          perItem = valueArgs.reduce((total: number, arg) => {
            if (typeof arg === 'string') return total + arg.length;
            return total + 1;
          }, 0);
        } else if (kind === 'DW') {
          perItem = valueArgs.length * 2;
        } else if (kind === 'DD') {
          perItem = valueArgs.length * 4;
        }
        return count * perItem;
      }
      default:
        return 0;
    }
  }

  private encodeDirective(
    node: DirectiveNode,
    labels: Record<string, number>,
    constants: Record<string, number>,
    currentOffset?: number,
  ): number[] {
    const bytes: number[] = [];
    const offset = currentOffset ?? 0;

    const evalArg = (arg: ExpressionValue): number => {
      if (typeof arg === 'number') return arg;
      const val = this.evaluateExpression(arg, labels, constants, offset);
      return val ?? 0;
    };

    switch (node.name) {
      case 'ORG':
      case 'EQU':
      case 'BITS':
      case 'SECTION': {
        // nasm aligns sections to 4 bytes in flat binary mode
        const sectionName = (
          typeof node.args[0] === 'string' ? node.args[0] : ''
        ).toLowerCase();
        if (sectionName === '.data' || sectionName === '.bss') {
          const remainder = offset % 4;
          const padding = remainder === 0 ? 0 : 4 - remainder;
          for (let i = 0; i < padding; i++) {
            bytes.push(0x00); // Zero padding for data section alignment
          }
        }
        break;
      }
      case 'ALIGN': {
        const alignment = evalArg(node.args[0]);
        if (alignment > 0) {
          const remainder = offset % alignment;
          const padding = remainder === 0 ? 0 : alignment - remainder;
          for (let i = 0; i < padding; i++) {
            bytes.push(this.alignmentFill);
          }
        }
        break;
      }
      case 'DB':
        for (const arg of node.args) {
          if (typeof arg === 'string') {
            for (let i = 0; i < arg.length; i++) {
              bytes.push(arg.charCodeAt(i));
            }
          } else {
            bytes.push(evalArg(arg) & 0xff);
          }
        }
        break;
      case 'DW':
        for (const arg of node.args) {
          bytes.push(...this.emitValue(evalArg(arg), 2));
        }
        break;
      case 'DD':
        for (const arg of node.args) {
          bytes.push(...this.emitValue(evalArg(arg), 4));
        }
        break;
      case 'TIMES': {
        const count = evalArg(node.args[0]);
        const kind = node.args[1] as string;
        const valueArgs = node.args.slice(2);
        for (let i = 0; i < count; i++) {
          if (kind === 'DB') {
            for (const arg of valueArgs) {
              if (typeof arg === 'string') {
                for (let j = 0; j < arg.length; j++) {
                  bytes.push(arg.charCodeAt(j));
                }
              } else {
                bytes.push(evalArg(arg) & 0xff);
              }
            }
          } else if (kind === 'DW') {
            for (const arg of valueArgs) {
              bytes.push(...this.emitValue(evalArg(arg), 2));
            }
          } else if (kind === 'DD') {
            for (const arg of valueArgs) {
              bytes.push(...this.emitValue(evalArg(arg), 4));
            }
          }
        }
        break;
      }
    }

    return bytes;
  }
}

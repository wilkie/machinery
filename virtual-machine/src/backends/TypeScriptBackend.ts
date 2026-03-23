import type {
  ModeInfo,
  InstructionForm,
  InstructionFormFlat,
  InstructionFormModes,
  InstructionInfo,
  OpcodeMatcher,
  OpcodeMatcherField,
} from '@machinery/core';

import {
  AssignmentNode,
  BinaryExpressionNode,
  BinaryLogicNode,
  ComparisonEvaluationNode,
  ComparisonNode,
  ExpressionNode,
  OperandNode,
  NextIfNode,
  IfBlockNode,
  LocalOperandNode,
  LoopBlockNode,
  LoopIfNode,
  TernaryExpressionNode,
  UnaryExpressionNode,
} from '../ast';
import Backend from '../Backend';
import type {
  DecoderMap,
  ExpressionType,
  InputMap,
  GeneratedStatement,
  MemoryReference,
  RegisterReference,
  LocalMap,
  LocalReference,
  LocalsInfo,
  Reference,
} from '../types';

interface PrefixMapEntry {
  instruction: InstructionInfo;
  variant: number;
}

interface DecoderContext {
  /** The mode we are decoding */
  mode: string;
  /** The decoder code */
  code: string[];
  /** Any variables used within instruction operation */
  variables: string[];
  /** The maximum number of bytes the decoder will ever read on one pass */
  bytesCount: number;
  /** The prefix count */
  prefixCount: number;
  /** A mapping of known prefixes to instruction form information. */
  prefixMap: PrefixMapEntry[];
}

/**
 * Allowed-prefix condition collected from the decoder trie.
 * Each entry describes a byte-level condition at a given depth
 * that leads to a form with the prefix in its `allow` list.
 */
interface AllowedPrefixCondition {
  /** Sequence of (depth, byte-test) entries leading to this form. */
  path: { depth: number; type: 'exact'; byte: number }[];
  /** The mask/and partial match at the deepest level, if any. */
  partial?: { depth: number; mask: number; and: number };
}

/**
 * Recursively walks a DecoderMap trie and collects all byte-level paths
 * that lead to instruction forms whose `allow` array includes `prefixId`.
 * Also collects prefix byte values so the lookahead allows other prefixes
 * to follow (they'll be decoded normally in subsequent loop iterations).
 */
function collectAllowedConditions(
  decoder: DecoderMap,
  prefixId: string,
  depth: number = 0,
  path: { depth: number; type: 'exact'; byte: number }[] = [],
): { conditions: AllowedPrefixCondition[]; prefixBytes: number[] } {
  const conditions: AllowedPrefixCondition[] = [];
  const prefixBytes: number[] = [];

  // Check leaf node
  if (decoder.instruction && decoder.form) {
    const form = decoder.form as InstructionFormFlat & InstructionFormModes;
    if (form.allow?.includes(prefixId)) {
      conditions.push({ path: [...path] });
    }
    return { conditions, prefixBytes };
  }

  // Walk exact matches
  if (decoder.exact) {
    for (let i = 0; i < decoder.exact.length; i++) {
      const child = decoder.exact[i];
      if (!child) continue;
      // Collect prefix byte values — other prefixes are always allowed
      // to follow a disallowed prefix (they'll be processed normally)
      if (child.instruction?.prefix) {
        if (depth === 0) prefixBytes.push(i);
        continue;
      }
      const childPath = [...path, { depth, type: 'exact' as const, byte: i }];
      const sub = collectAllowedConditions(
        child,
        prefixId,
        depth + 1,
        childPath,
      );
      conditions.push(...sub.conditions);
    }
  }

  // Walk partial matches
  if (decoder.partial) {
    for (const { mask, and, map } of decoder.partial) {
      // Check leaf
      if (map.instruction && map.form) {
        const form = map.form as InstructionFormFlat & InstructionFormModes;
        if (form.allow?.includes(prefixId)) {
          conditions.push({
            path: [...path],
            partial: { depth, mask, and },
          });
        }
      } else {
        // Recurse into partial subtree
        const sub = collectAllowedConditions(
          map as DecoderMap,
          prefixId,
          depth + 1,
          path,
        );
        conditions.push(...sub.conditions);
      }
    }
  }

  return { conditions, prefixBytes };
}

/**
 * Build a JS condition expression that checks peeked bytes against
 * the collected allowed conditions for a prefix with `disallowed`.
 *
 * The generated expression references `_peek0`, `_peek1`, etc. for bytes
 * at _ip+0, _ip+1, ... which the caller must declare.
 */
function buildAllowedCondition(
  conditions: AllowedPrefixCondition[],
  prefixBytes: number[],
): { expr: string; maxDepth: number } {
  let maxDepth = 0;
  const parts: string[] = [];

  // Other prefix bytes are always allowed (they'll be decoded normally)
  if (prefixBytes.length > 0) {
    maxDepth = Math.max(maxDepth, 1);
    for (const b of prefixBytes) {
      parts.push(`_peek0 === 0x${b.toString(16)}`);
    }
  }

  for (const cond of conditions) {
    const checks: string[] = [];
    for (const step of cond.path) {
      maxDepth = Math.max(maxDepth, step.depth + 1);
      checks.push(`_peek${step.depth} === 0x${step.byte.toString(16)}`);
    }
    if (cond.partial) {
      maxDepth = Math.max(maxDepth, cond.partial.depth + 1);
      checks.push(
        `(_peek${cond.partial.depth} & 0x${cond.partial.mask.toString(16)}) === 0x${cond.partial.and.toString(16)}`,
      );
    }
    if (checks.length > 0) {
      parts.push(checks.length === 1 ? checks[0] : `(${checks.join(' && ')})`);
    }
  }

  return {
    expr: parts.length === 0 ? 'false' : parts.join(' || '),
    maxDepth,
  };
}

/** Sanitize an identifier for use as a JavaScript property name (e.g. AF' → AF_) */
function jsName(name: string): string {
  return name.replace(/'/g, '_');
}

class TypeScriptBackend extends Backend {
  applyCoercion(result: string[], coercion: string): string[] {
    const size = parseInt(coercion.slice(1));
    const last = result[result.length - 1];
    if (coercion.startsWith('i')) {
      result[result.length - 1] = `(${last} << ${32 - size} >> ${32 - size})`;
    } else if (size >= 32) {
      // For 32-bit unsigned, use >>> 0 to avoid JS signed int32 issues
      result[result.length - 1] = `((${last}) >>> 0)`;
    } else {
      const mask = (Math.pow(2, size) - 1) >>> 0;
      result[result.length - 1] = `((${last}) & 0x${mask.toString(16)})`;
    }
    return result;
  }

  prologue(): string[] {
    const target = this.target;
    const code: string[] = [];

    code.push('// This file is generated by @machinery/virtual-machine');
    code.push('/* eslint-disable */');
    code.push('');
    code.push('export default class Machine {');
    code.push(
      `  static readonly RAM_OFFSET = 0x${this.linearStart.toString(16)};`,
    );
    code.push('  memory: WebAssembly.Memory;');
    code.push('  mem8: Uint8Array;');
    code.push('  mem16: Uint16Array;');
    code.push('  mem32: Uint32Array;');
    code.push('');
    code.push('  constructor() {');
    code.push(
      '    this.memory = new WebAssembly.Memory({ initial: 30, maximum: 1024 });',
    );
    code.push('    this.mem8 = new Uint8Array(this.memory.buffer);');
    code.push('    this.mem16 = new Uint16Array(this.memory.buffer);');
    code.push('    this.mem32 = new Uint32Array(this.memory.buffer);');
    code.push('    this.mode = 0;');
    code.push('');
    // Initialize ROM
    code.push('    // Load ROM (if any)');
    for (const [name, info] of Object.entries(this.memoryMap || {})) {
      if (!info.data || info.data.length === 0) {
        continue;
      }
      code.push(`    // ${name}`);
      code.push(
        `    this.mem${info.size}.set(${JSON.stringify(info.data)}, 0x${info.start.toString(16)}${info.size !== 8 ? ` >> ${info.size / 16}` : ''});`,
      );
    }
    code.push('');
    code.push('    this.reset();');
    code.push('  }');
    code.push('');
    code.push('  reset() {');
    // Zero register space
    code.push('    // Zero registers');
    code.push(
      `    this.mem8.set(Array(0x${this.registerMap.size.toString(16)}).fill(0), 0x${this.registerMap.start.toString(16)});`,
    );
    // Initialize registers
    code.push('    // Initialize registers');
    for (const [name, info] of Object.entries(
      this.registerMap.registers || {},
    )) {
      if (info.initialValue === undefined || info.initialValue === 0) {
        continue;
      }
      code.push(`    // ${name}`);
      const generated: GeneratedStatement = {
        accesses: [],
        modifies: [name],
        code: [],
        context: {
          mode: this.target.modes?.[0]?.identifier || 'default',
          locals: {},
          localMap: { _ip: { identifier: '_ip' } },
        },
      };
      const writeCode = this.writeRegister(
        generated,
        {
          type: 'register',
          identifier: name,
          mapping: info,
          size: info.size,
        },
        `0x${info.initialValue.toString(16)}`,
      );
      for (const line of writeCode) {
        code.push(`    ${line};`);
      }
      for (const line of this.fromModifies(generated)) {
        code.push(`    ${line};`);
      }
    }
    code.push('  }');
    const locals: LocalsInfo = {};
    code.push('');
    if (this.parsed.interrupts.handler) {
      ((target.modes as Pick<ModeInfo, 'identifier'>[]) || [])
        .concat([{ identifier: 'default' }])
        .forEach((modeInfo) => {
          const mode = modeInfo.identifier;
          const { statement, localMap } =
            this.parsed.interrupts.handler?.[mode] ||
            this.parsed.interrupts.handler?.default ||
            {};

          // Transpile the statement node for this instruction form
          if (statement && localMap) {
            const { code: handlerCode } = this.fromStatement(statement, {
              mode,
              locals,
              localMap,
            });
            code.push(
              `  interrupt_${mode}(${localMap.vector?.identifier || '_'}: number) {`,
            );
            for (const [localName, localInfo] of Object.entries(localMap)) {
              if (localName === 'vector') {
                continue;
              }
              code.push(`    let ${localInfo.identifier} = 0;`);
            }
            for (const line of handlerCode) {
              code.push('    ' + line);
            }
          } else {
            code.push(`  interrupt_${mode}(vector: number) {`);
            code.push('    console.log("INT", vector);');
          }
          code.push('  }');
          code.push('');
        });
    }

    // Create functions for accessing programmable memory
    for (const memoryInfo of this.target.memory || []) {
      if (memoryInfo.type === 'programmable') {
        code.push(
          `  ${memoryInfo.identifier}_read(size: number, _address: number): number {`,
        );
        code.push('    if (size === 32) {');
        code.push(
          `    return 0x${((((memoryInfo.default || 0x0) << 24) | ((memoryInfo.default || 0x0) << 16) | ((memoryInfo.default || 0x0) << 8) | (memoryInfo.default || 0x0)) >>> 0).toString(16)};`,
        );
        code.push('    }');
        code.push('    else if (size === 16) {');
        code.push(
          `    return 0x${(((memoryInfo.default || 0x0) << 8) | (memoryInfo.default || 0x0)).toString(16)};`,
        );
        code.push('    }');
        code.push(`    return 0x${(memoryInfo.default || 0x0).toString(16)};`);
        code.push(`  }`);
        code.push('');
        code.push(
          `  ${memoryInfo.identifier}_write(_size: number, _address: number, _value: number) {`,
        );
        code.push(`  }`);
        code.push('');
      }
    }

    // Create functions for accessing system state
    const generatedDummy: GeneratedStatement = {
      accesses: [],
      modifies: [],
      code: [],
      context: {
        mode: 'default',
        locals: {},
        localMap: { _ip: { identifier: '_ip' } },
      },
    };
    if (this.parsed.system.mode) {
      code.push(`  get mode(): number {`);
      code.push(
        `    return ${this.readMemory(generatedDummy, this.parsed.system.mode, this.parsed.system.mode.address)};`,
      );
      code.push(`  }`);
      code.push('');
      code.push(`  set mode(value: number) {`);
      code.push(
        `    ${this.writeMemory(generatedDummy, this.parsed.system.mode, this.parsed.system.mode.address, 'value')};`,
      );
      code.push(`  }`);
      code.push('');
    }

    // Create functions for accessing registers
    for (const info of target.registers) {
      const name = info.identifier;
      const parsedRegister = this.parsed.registers[name];
      const getModes = Object.keys(parsedRegister?.get || {});
      const getOnlyDefault =
        getModes.length === 0 ||
        (getModes.length === 1 && getModes[0] === 'default');
      code.push(`  get ${jsName(name)}() {`);
      ((target.modes as Pick<ModeInfo, 'identifier'>[]) || [])
        .concat([{ identifier: 'default' }])
        .forEach((modeInfo, i) => {
          const mode = modeInfo.identifier;
          if (mode !== 'default' && getModes.includes(mode)) {
            code.push(`    ${i !== 0 ? 'else ' : ''}if (this.mode === ${i}) {`);
          } else if (mode === 'default' && !getOnlyDefault) {
            code.push('    else { // default');
          }
          if (getModes.includes(mode) || mode === 'default') {
            const { localMap }: { localMap: LocalMap } = parsedRegister?.get?.[
              mode
            ] || { localMap: {} };
            for (const [localName, localInfo] of Object.entries(localMap)) {
              if (localName === 'vector') {
                continue;
              }
              code.push(`      let ${localInfo.identifier} = 0;`);
            }
            const generated: GeneratedStatement = {
              accesses: [name],
              modifies: [],
              code: [],
              context: {
                mode,
                locals: {},
                localMap: { _ip: { identifier: '_ip' } },
                suppressRaise: true,
              },
            };
            const readCode = this.readRegister(generated, {
              type: 'register',
              identifier: name,
              mapping: this.registerMap.registers[name],
              size: this.registerMap.registers[name].size,
            });
            for (const line of this.fromAccessors(generated)) {
              code.push(`    ${getOnlyDefault ? '' : '  '}${line};`);
            }
            for (const line of readCode.slice(0, readCode.length - 1)) {
              code.push(`    ${getOnlyDefault ? '' : '  '}${line};`);
            }
            code.push(
              `    ${getOnlyDefault ? '' : '  '}return ${readCode[readCode.length - 1]};`,
            );
            if (!getOnlyDefault) {
              code.push(`    }`);
            }
          }
        });
      code.push('  }');
      code.push('');
      const setModes = Object.keys(parsedRegister?.set || {});
      const setOnlyDefault =
        setModes.length === 0 ||
        (setModes.length === 1 && setModes[0] === 'default');
      code.push(`  set ${jsName(name)}(value: number) {`);
      ((target.modes as Pick<ModeInfo, 'identifier'>[]) || [])
        .concat([{ identifier: 'default' }])
        .forEach((modeInfo, i) => {
          const mode = modeInfo.identifier;
          if (mode !== 'default' && setModes.includes(mode)) {
            code.push(
              `    ${i !== 0 ? 'else ' : ''}if (this.mode === ${i}) { // mode: ${mode}`,
            );
          } else if (mode === 'default' && !setOnlyDefault) {
            code.push('    else { // default');
          }
          if (setModes.includes(mode) || mode === 'default') {
            const { localMap }: { localMap: LocalMap } = parsedRegister?.set?.[
              mode
            ] || { localMap: {} };
            for (const [localName, localInfo] of Object.entries(localMap)) {
              if (localName === 'vector') {
                continue;
              }
              code.push(`      let ${localInfo.identifier} = 0;`);
            }
            const generated2: GeneratedStatement = {
              accesses: [],
              modifies: [name],
              code: [],
              context: {
                mode,
                locals: {},
                localMap: { _ip: { identifier: '_ip' } },
                suppressRaise: true,
              },
            };
            const writeCode = this.writeRegister(
              generated2,
              {
                type: 'register',
                identifier: name,
                mapping: this.registerMap.registers[name],
                size: this.registerMap.registers[name].size,
              },
              'value',
            );
            for (const line of writeCode) {
              code.push(`    ${setOnlyDefault ? '' : '  '}${line};`);
            }
            for (const line of this.fromModifies(generated2)) {
              code.push(`    ${setOnlyDefault ? '' : '  '}${line};`);
            }
            if (!setOnlyDefault) {
              code.push(`    }`);
            }
          }
        });
      code.push('  }');
      code.push('');

      for (const subinfo of info.fields || []) {
        if (!subinfo.global) {
          continue;
        }
        const subname = subinfo.identifier;
        const generated3: GeneratedStatement = {
          accesses: [],
          modifies: [],
          code: [],
          context: {
            mode: this.target.modes?.[0]?.identifier || 'default',
            locals: {},
            localMap: { _ip: { identifier: '_ip' } },
            suppressRaise: true,
          },
        };
        code.push('  get ' + jsName(subname) + '() {');
        for (const line of this.fromAccessors(generated3)) {
          code.push(`    ${line};`);
        }
        const readSubCode = this.readRegister(generated3, {
          type: 'register',
          identifier: subname,
          mapping: this.registerMap.registers[subname],
          size: this.registerMap.registers[subname].size,
        });
        for (const line of readSubCode.slice(0, readSubCode.length - 1)) {
          code.push(`    ${line};`);
        }
        code.push('    return ' + readSubCode[readSubCode.length - 1] + ';');
        code.push('  }');
        code.push('');
        const writeSubCode = this.writeRegister(
          generated3,
          {
            type: 'register',
            identifier: subname,
            mapping: this.registerMap.registers[subname],
            size: this.registerMap.registers[subname].size,
          },
          'value',
        );
        code.push('  set ' + jsName(subname) + '(value: number) {');
        for (const line of writeSubCode) {
          code.push(`    ${line};`);
        }
        for (const line of this.fromModifies(generated3)) {
          code.push(`    ${line};`);
        }
        code.push('  }');
        code.push('');
      }
    }

    return code;
  }

  epilogue(): string[] {
    return ['}'];
  }

  private craftInstruction(
    instruction: InstructionInfo,
    form: InstructionForm,
    index: number,
    _inputs: InputMap,
    variant: number,
    depth: number,
    indent: string,
    context: DecoderContext,
  ) {
    const target = this.target;
    const prefix = !!instruction.prefix;

    const ip =
      target.fetch?.effectiveRegister ||
      (Array.isArray(target.fetch?.register)
        ? target.fetch?.register?.[0]
        : target.fetch?.register);
    const instructionMemory = target.fetch?.memory;

    if (ip === undefined || instructionMemory === undefined) {
      throw new Error('Error: The target has no defined instruction pointer.');
    }

    const ipReference: RegisterReference = {
      type: 'register',
      identifier: ip,
      mapping: this.registerMap.registers[ip],
      size: this.registerMap.registers[ip].size,
    };

    const locals: LocalsInfo = {};
    for (const localInfo of instruction.locals || []) {
      locals[localInfo.identifier] = localInfo;
    }

    // Throwaway context
    const generated: GeneratedStatement = {
      accesses: [],
      modifies: [],
      code: [],
      context: {
        mode: context.mode || this.target.modes?.[0]?.identifier || 'default',
        locals: {},
        localMap: { _ip: { identifier: '_ip' } },
      },
    };

    const { statement, localMap } =
      this.parsed.instructions[instruction.identifier][variant][context.mode]
        .operation;

    // Read the rest of the data for the instruction before going to the execute phase
    let bytesRead = 0;
    context.code.push(`${indent}// ${instruction.identifier}[${variant}]`);
    for (let i = 0; i < form.opcode.length; i++) {
      let matcher: string | number | OpcodeMatcher | undefined = form.opcode[i];
      const matcher_name =
        typeof matcher === 'string'
          ? matcher
          : typeof matcher === 'number'
            ? matcher.toString()
            : matcher.identifier;
      if (typeof matcher === 'string') {
        // this is expanded to another named operand
        //context.code.push(indent + '// ' + matcher_name);
        matcher = (target.operands || ([] as OpcodeMatcher[])).find(
          (operand) => operand.identifier === matcher_name,
        );
        if (matcher === undefined) {
          throw new Error(`Error: No operand found called ${matcher_name}`);
        }
        if (typeof matcher === 'string') {
          throw new Error(
            `Error: Operand ${matcher_name} improperly aliases another matcher '${matcher}'`,
          );
        }
      }

      if (matcher === undefined) {
        // Ah, this matcher has no operand. bail.
        continue;
      }

      if (typeof matcher === 'number') {
        // ignore this
        //context.code.push(indent + '// ' + matcher);
        bytesRead++;
        continue;
      }

      // Read bytes we need for this matcher, if necessary
      if (bytesRead > depth) {
        if ((matcher.size || 8) === 8) {
          const var_name = 'b' + bytesRead.toString();
          const byte_reference: LocalReference = {
            type: 'local',
            identifier: var_name,
            mapping: {
              identifier: var_name,
            },
          };
          generated.context.locals[var_name] = {
            identifier: var_name,
            size: 8,
          };
          generated.context.localMap[var_name] = {
            identifier: var_name,
          };
          context.code.push(
            indent +
              this.writeLocal(generated, byte_reference, 'this.mem8[_ip]') +
              ';',
          );
          context.code.push(indent + '_ip++;');
          if (!context.variables.includes(var_name)) {
            context.variables.push(var_name);
          }
        } else if (matcher.size === 16) {
          const var_name = 'b' + bytesRead.toString();
          const ip_reference: LocalReference = {
            type: 'local',
            identifier: '_ip',
            mapping: {
              identifier: '_ip',
            },
          };
          const byte_reference: LocalReference = {
            type: 'local',
            identifier: var_name,
            mapping: {
              identifier: var_name,
            },
          };
          generated.context.locals[var_name] = {
            identifier: var_name,
            size: 16,
          };
          generated.context.localMap[var_name] = {
            identifier: var_name,
          };
          const address = new ExpressionNode(
            new LocalOperandNode(ip_reference, new OperandNode('_ip')),
          );
          context.code.push(
            indent +
              this.writeLocal(
                generated,
                byte_reference,
                this.readMemory(
                  generated,
                  {
                    type: 'memory',
                    identifier: 'memory',
                    aligned: false,
                    mapping: {
                      start: 0,
                      size: 16,
                      length: 32,
                      data: [],
                      info: {
                        identifier: '*',
                        name: '*',
                        endian: 'little',
                      },
                    },
                    address,
                    size: 16,
                  },
                  address,
                )[0],
              ) +
              ';',
          );
          context.code.push(indent + '_ip += 2;');
          if (!context.variables.includes(var_name)) {
            context.variables.push(var_name);
          }
        } else if (matcher.size === 32) {
          const var_name = 'b' + bytesRead.toString();
          const ip_reference: LocalReference = {
            type: 'local',
            identifier: '_ip',
            mapping: {
              identifier: '_ip',
            },
          };
          const byte_reference: LocalReference = {
            type: 'local',
            identifier: var_name,
            mapping: {
              identifier: var_name,
            },
          };
          generated.context.locals[var_name] = {
            identifier: var_name,
            size: 32,
          };
          generated.context.localMap[var_name] = {
            identifier: var_name,
          };
          const address = new ExpressionNode(
            new LocalOperandNode(ip_reference, new OperandNode('_ip')),
          );
          context.code.push(
            indent +
              this.writeLocal(
                generated,
                byte_reference,
                this.readMemory(
                  generated,
                  {
                    type: 'memory',
                    identifier: 'memory',
                    aligned: false,
                    mapping: {
                      start: 0,
                      size: 32,
                      length: 32,
                      data: [],
                      info: {
                        identifier: '*',
                        name: '*',
                        endian: 'little',
                      },
                    },
                    address,
                    size: 32,
                  },
                  address,
                )[0],
              ) +
              ';',
          );
          context.code.push(indent + '_ip += 4;');
          if (!context.variables.includes(var_name)) {
            context.variables.push(var_name);
          }
        }
      }
      bytesRead += (matcher.size || 8) / 8;

      for (const field of matcher.fields || [
        matcher as unknown as OpcodeMatcherField,
      ]) {
        if (typeof field === 'string') {
          continue;
        }

        if (field.match !== undefined) {
          continue;
        }

        const var_name = field.identifier;
        locals[var_name] = {
          identifier: var_name,
          size: matcher.size,
          signed: matcher.signed,
        };

        // Allocate a variable
        const variableCount = Object.keys(localMap).length;
        const allocated_name =
          var_name in localMap
            ? localMap[var_name].identifier
            : `v_${variableCount}`;
        if (!(var_name in localMap)) {
          localMap[var_name] = {
            identifier: allocated_name,
          };
        }

        //if (!context.variables.includes(var_name)) {
        //  context.variables.push(var_name);
        //}

        let readCode = allocated_name + ' = (';
        if (matcher.size === 8) {
          readCode += 'b' + (bytesRead - 1).toString();
        } else if (matcher.size === 16) {
          readCode += 'b' + (bytesRead - 2).toString();
        } else if (matcher.size === 32) {
          readCode += 'b' + (bytesRead - 4).toString();
        }

        if (field.offset) {
          readCode += ' >> ' + field.offset;
        }

        if (typeof field.size === 'number') {
          readCode += ' & 0x' + (Math.pow(2, field.size) - 1).toString(16);
        }

        readCode += ')';

        if (matcher.signed) {
          readCode +=
            ' << ' + (32 - matcher.size) + ' >> ' + (32 - matcher.size);
        } else {
          if (matcher.size === 32) {
            readCode += ' >>> 0';
          }
        }

        readCode += ';';
        readCode +=
          ' // ' + i + ' , ' + index + ' , ' + depth + ' , ' + bytesRead;
        context.code.push(indent + readCode);
      }
    }

    // Advance the instruction pointer, if we are supposed to do so
    if (target.fetch?.advancePointer) {
      context.code.push(indent + '// IP += 0x' + bytesRead.toString(16));
      context.code.push(
        indent +
          this.writeRegister(
            generated,
            ipReference,
            this.readRegister(generated, ipReference) +
              ' + 0x' +
              bytesRead.toString(16),
          ) +
          ';',
      );
      //context.code.push(indent + this.readGlobal(generated, ip) + " += 0x" + bytesRead.toString(16) + ";");
      generated.context.ipAdvance = bytesRead;

      // Check instruction length limit
      const limitInfo2 = this.parsed.limit?.[context.mode];
      if (limitInfo2 && !prefix) {
        const { code: limitCode } = this.fromStatement(
          limitInfo2.parsed.statement,
        );
        context.code.push(
          `${indent}if (_ip - _ip_start > ${limitInfo2.bytes}) { ${this.writeRegister(generated, ipReference, '_ip_save')}; ${limitCode.join(' ')} }`,
        );
      }
    }

    // Read next byte (if this is a prefix)
    if (prefix) {
      // If there is some finalize code, remember this prefix
      if (
        (form as InstructionFormFlat).finalize ||
        (form as InstructionFormModes).modes?.[context.mode]?.finalize
      ) {
        context.code.push(
          `${indent}_finalize.push(0x${context.prefixCount.toString(16)});`,
        );
        context.prefixMap.push({
          instruction,
          variant,
        });
        context.prefixCount++;
      }
    }

    // Transpile the statement node for this instruction form
    if (statement) {
      const { code } = this.fromStatement(statement, {
        mode: context.mode || this.target.modes?.[0]?.identifier || 'default',
        locals,
        localMap,
        ipAdvance: generated.context.ipAdvance,
      });
      for (const line of code) {
        context.code.push(indent + line);
      }
      for (const localInfo of Object.values(localMap)) {
        const var_name = localInfo.identifier;
        if (!context.variables.includes(var_name)) {
          context.variables.push(var_name);
        }
      }
    }
  }

  private craftDecoderState(
    decoder: DecoderMap,
    depth: number,
    indent: string,
    context: DecoderContext,
    prefix: boolean = false,
  ) {
    // Keep track of where we should inject variable declarations
    const target = this.target;
    const startIndex = context.code.length;

    const ip =
      target.fetch?.effectiveRegister ||
      (Array.isArray(target.fetch?.register)
        ? target.fetch?.register?.[0]
        : target.fetch?.register);
    const instructionMemory = target.fetch?.memory;

    if (ip === undefined || instructionMemory === undefined) {
      throw new Error('Error: The target has no defined instruction pointer.');
    }

    const ipReference: RegisterReference = {
      type: 'register',
      identifier: ip,
      mapping: this.registerMap.registers[ip],
      size: this.registerMap.registers[ip].size,
    };

    // Throwaway context
    const generated: GeneratedStatement = {
      accesses: [],
      modifies: [],
      code: [],
      context: {
        mode: this.target.modes?.[0]?.identifier || 'default',
        locals: {},
        localMap: { _ip: { identifier: '_ip' } },
      },
    };

    if (depth === 0 && !prefix) {
      context.code.push('');
      const { start } = this.memoryMap[instructionMemory];
      context.code.push(`${start}`);
      context.code.push(
        `${indent}let _ip = ${this.readRegister(generated, ipReference)}${start ? ` + 0x${start.toString(16)}` : ''};`,
      );
      context.code.push(`${indent}let _finalize: number[] = [];`);

      // Save IP at start of instruction (before prefixes) for fault rollback
      context.code.push(
        `${indent}let _ip_save = ${this.readRegister(generated, ipReference)};`,
      );
      const { start: ramStart } = this.memoryMap[instructionMemory];
      const ipStartExpr = `_ip_save${ramStart ? ` + 0x${ramStart.toString(16)}` : ''}`;
      context.code.push(`${indent}let _ip_start = ${ipStartExpr};`);
      // Define the prefix loop
      context.code.push(indent + 'outer: while (true) {');
      indent += '  ';
    }
    const byteName = 'b' + depth;
    context.bytesCount = Math.max(context.bytesCount || 0, depth);
    generated.context.locals[byteName] = {
      identifier: byteName,
      size: 8,
    };
    generated.context.localMap[byteName] = {
      identifier: byteName,
    };
    if (!prefix) {
      const byte_reference: LocalReference = {
        type: 'local',
        identifier: byteName,
        mapping: {
          identifier: byteName,
        },
      };
      context.code.push(
        indent +
          this.writeLocal(generated, byte_reference, 'this.mem8[_ip]') +
          ';',
      );
      context.code.push(indent + '_ip++;');
    }
    if (!context.variables.includes(byteName)) {
      context.variables.push(byteName);
    }

    // We embed the prefix decoder inside the normal decoder using the
    // labeled breaks and continues to make it work.
    if (decoder.exact) {
      context.code.push(indent + 'switch(' + byteName + ') {');
      // Track shared references to emit fallthrough case labels for aliases
      const emitted = new Set<DecoderMap>();
      for (let i = 0; i < (decoder.exact || []).length; i++) {
        const matcher = decoder.exact[i];
        if (matcher) {
          // Skip if this is an alias whose primary (lower) case was already emitted
          if (emitted.has(matcher)) continue;
          emitted.add(matcher);
          // Emit the primary case label
          context.code.push(indent + '  case ' + '0x' + i.toString(16) + ':');
          // Emit fallthrough case labels for any aliases (higher byte values
          // that share the same decoder map reference)
          for (let j = i + 1; j < decoder.exact.length; j++) {
            if (decoder.exact[j] === matcher) {
              context.code.push(
                indent + '  case ' + '0x' + j.toString(16) + ':',
              );
            }
          }
          // Go through these cases, too
          if (
            matcher.instruction &&
            matcher.form &&
            matcher.index !== undefined &&
            matcher.inputs !== undefined &&
            matcher.variant !== undefined
          ) {
            // This decoded to a known instruction... read the rest of the data for the
            // instruction before going to the execute phase
            this.craftInstruction(
              matcher.instruction,
              matcher.form,
              matcher.index,
              matcher.inputs,
              matcher.variant,
              depth,
              indent + '    ',
              context,
            );
            if (matcher.instruction.prefix) {
              if (matcher.instruction.disallowed) {
                // Prefix with disallowed: generate lookahead to check if
                // the next instruction allows this prefix, otherwise fault.
                const prefixId = matcher.instruction.identifier;
                const { conditions, prefixBytes } = collectAllowedConditions(
                  this.decoderMap,
                  prefixId,
                );
                const { expr, maxDepth } = buildAllowedCondition(
                  conditions,
                  prefixBytes,
                );
                const ind = indent + '    ';
                if (conditions.length > 0) {
                  // Declare peek variables for the bytes we need to inspect
                  for (let p = 0; p < maxDepth; p++) {
                    context.code.push(
                      `${ind}const _peek${p} = this.mem8[_ip${p > 0 ? ' + ' + p : ''}];`,
                    );
                  }
                  context.code.push(`${ind}if (${expr}) {`);
                  context.code.push(`${ind}  continue outer;`);
                  context.code.push(`${ind}}`);
                }
                // Disallowed path: resolve the interrupt vector from the
                // disallowed operation (e.g. '#UD' → '#6' → interrupt 6).
                const disallowedOp = (
                  (matcher.instruction.disallowed as InstructionFormModes)
                    .modes?.[context.mode]?.operation ||
                  (matcher.instruction.disallowed as InstructionFormFlat)
                    .operation
                ).flat(19) as string[];
                // Extract interrupt vector from the operation (supports
                // '#UD', '#6', '#GP' etc. via the target's interrupt table)
                let vector: number | undefined;
                if (disallowedOp) {
                  for (const op of disallowedOp) {
                    const intMatch = op.match(/^#(\w+)/);
                    if (intMatch) {
                      const name = intMatch[1];
                      // Check if it's a numeric vector
                      const num = parseInt(name, 10);
                      if (!isNaN(num)) {
                        vector = num;
                      } else {
                        // Look up named exception in interrupt vectors
                        const vec = this.target.interrupts?.vectors?.find(
                          (v) => v.identifier === name,
                        );
                        if (vec?.index !== undefined) {
                          vector = vec.index;
                        }
                      }
                    }
                  }
                }
                if (vector !== undefined) {
                  // Emit IP rollback + interrupt for the disallowed case
                  const ipReg =
                    this.target.fetch?.effectiveRegister ||
                    (Array.isArray(this.target.fetch?.register)
                      ? this.target.fetch?.register?.[0]
                      : this.target.fetch?.register);
                  if (ipReg && this.registerMap.registers[ipReg]) {
                    const ipRef: RegisterReference = {
                      type: 'register',
                      identifier: ipReg,
                      mapping: this.registerMap.registers[ipReg],
                      size: this.registerMap.registers[ipReg].size,
                    };
                    context.code.push(
                      `${ind}${this.writeRegister(generated, ipRef, '_ip_save')};`,
                    );
                  }
                  context.code.push(
                    `${ind}this.interrupt_${context.mode}(${vector}); return;`,
                  );
                } else {
                  // Fallback: no vector found, just break
                  context.code.push(`${ind}break outer;`);
                }
              } else {
                context.code.push(indent + '    continue outer;');
              }
            } else {
              context.code.push(indent + '    break outer;');
            }
          } else if (matcher.exact || matcher.partial) {
            // We further break this down by exact matches and partials
            this.craftDecoderState(
              matcher,
              depth + 1,
              indent + '    ',
              context,
              prefix,
            );
            context.code.push(indent + '    break;');
          } else {
            context.code.push(indent + '    break;');
          }
        }
      }
      context.code.push(indent + '  default:');
      indent = indent + '    ';
    }

    // Handle wildcard (consume-any-byte) nodes, e.g., displacement before final opcode
    if (decoder.wildcard) {
      // The wildcard byte has already been consumed at this depth
      // (byteName was read above). Recurse into the wildcard's map to match
      // the next byte.
      this.craftDecoderState(
        decoder.wildcard.map,
        depth + 1,
        indent,
        context,
        prefix,
      );
    }

    // Look at partials, if any (prefixes cannot have them)
    let ifword = 'if';
    for (const { mask, and, map } of decoder.partial || []) {
      context.code.push(
        indent +
          ifword +
          ' ((' +
          byteName +
          ' & 0x' +
          mask.toString(16) +
          ') === 0x' +
          and.toString(16) +
          ') {',
      );
      if (
        map.instruction &&
        map.form &&
        map.index !== undefined &&
        map.inputs !== undefined &&
        map.variant !== undefined
      ) {
        this.craftInstruction(
          map.instruction,
          map.form,
          map.index,
          map.inputs,
          map.variant,
          depth,
          indent + '  ',
          context,
        );
      } else {
        this.craftDecoderState(
          map,
          depth + 1,
          indent + '    ',
          context,
          prefix,
        );
      }
      context.code.push(indent + '}');
      ifword = 'else if';
    }
    // Maybe we cannot find any — emit the target's "unknown" operation (e.g. #UD)
    if ((decoder.partial || []).length > 0) {
      const unknownParsed = this.parsed.unknown?.[context.mode];
      context.code.push(indent + 'else {');
      if (unknownParsed) {
        // Restore IP to instruction start (before prefixes) — faults save the
        // faulting instruction address, not the current position.
        const ipReg =
          this.target.fetch?.effectiveRegister ||
          (Array.isArray(this.target.fetch?.register)
            ? this.target.fetch?.register?.[0]
            : this.target.fetch?.register);
        if (ipReg) {
          const mapping = this.registerMap.registers?.[ipReg];
          if (mapping) {
            let { size } = mapping;
            if (size <= 8) size = 8;
            else if (size <= 16) size = 16;
            else size = 32;
            context.code.push(
              indent + `  this.mem${size}[${mapping.index}] = _ip_save;`,
            );
          }
        }
        // Instruction length limit check — takes priority over unknown handler
        const limitInfo = this.parsed.limit?.[context.mode];
        if (limitInfo) {
          const { code: limitCode } = this.fromStatement(
            limitInfo.parsed.statement,
          );
          context.code.push(
            indent + `  if (_ip - _ip_start > ${limitInfo.bytes}) {`,
          );
          for (const line of limitCode) {
            context.code.push(indent + '    ' + line);
          }
          context.code.push(indent + '  }');
        }
        const { code } = this.fromStatement(unknownParsed.statement);
        for (const line of code) {
          context.code.push(indent + '  ' + line);
        }
      } else {
        context.code.push(indent + '  // undefined');
      }
      context.code.push(indent + '}');
    }
    if (decoder.exact) {
      indent = indent.substring(0, indent.length - 4);
      context.code.push(indent + '    break outer;');
      context.code.push(indent + '}');
    }

    if (depth === 0 && !prefix) {
      context.code.push(indent + '// never infinitely loop');
      context.code.push(indent + 'break;');
      indent = indent.substring(0, indent.length - 2);
      context.code.push(indent + '}');

      // Add finalizers
      context.code.push(`${indent}for (const prefixIndex of _finalize) {`);
      indent = indent + '  ';
      context.code.push(`${indent}switch(prefixIndex) {`);
      // Add all finalizers
      indent = indent + '  ';
      context.prefixMap.forEach((info: PrefixMapEntry, index: number) => {
        context.code.push(`${indent}case 0x${index.toString(16)}:`);
        indent = indent + '  ';
        const { instruction, variant } = info;
        const node =
          this.parsed.instructions[instruction.identifier][variant][
            context.mode
          ].finalize?.statement;
        if (node) {
          const { code } = this.fromStatement(node);
          for (const line of code) {
            context.code.push(indent + line);
          }
        }
        context.code.push(`${indent}break;`);
        indent = indent.substring(0, indent.length - 2);
      });
      context.code.push(`${indent}default:`);
      context.code.push(`${indent}  break;`);
      indent = indent.substring(0, indent.length - 2);
      context.code.push(`${indent}}`);
      indent = indent.substring(0, indent.length - 2);
      context.code.push(`${indent}}`);

      // Write in the temp variables and switches into the blank line we created
      let defineCode = '';
      context.variables.sort().forEach((name) => {
        defineCode += `let ${name} = 0; `;
      });
      context.code[startIndex] = indent + defineCode.trim();
    }
  }

  /**
   * Generates a bitstream decoder to execute instructions within that bitstream.
   *
   * Takes in decoder data via a DecoderMap.
   */
  buildDecoder(mode: string): string[] {
    const decoder = this.decoderMap;
    const context: DecoderContext = {
      mode,
      code: [],
      variables: [],
      bytesCount: 0,
      prefixCount: 0,
      prefixMap: [],
    };

    if (!decoder) {
      return [];
    }

    context.code.push(`  decode_${mode}() {`);
    const modeInfo = (this.target.modes || []).find(
      (info) => info.identifier === mode,
    );
    if (modeInfo?.decode) {
      this.craftDecoderState(decoder, 0, '    ', context);
    }
    context.code.push('  }');
    context.code.push('');

    return context.code;
  }

  comment(message: string): string[] {
    return [`// ${message}`];
  }

  wrapConditional(
    indexExpr: string,
    registerIndex: number,
    code: string[],
  ): string[] {
    return [
      `if (${indexExpr} === ${registerIndex}) {`,
      ...code.map((line) => `  ${line}`),
      `}`,
    ];
  }

  fromTernaryExpression(
    generated: GeneratedStatement,
    node: TernaryExpressionNode,
  ): string[] {
    const comparisonCode = this.fromComparison(generated, node.condition);
    const trueCode = this.fromOperandExpression(generated, node.operand);
    const falseCode = this.fromExpression(generated, node.whenFalse);
    return [
      ...comparisonCode.slice(0, comparisonCode.length - 1),
      ...trueCode.slice(0, trueCode.length - 1),
      ...falseCode.slice(0, falseCode.length - 1),
      `(${comparisonCode[comparisonCode.length - 1]} ? ${trueCode[trueCode.length - 1]} : ${falseCode[falseCode.length - 1]})`,
    ];
  }

  raise(
    generated: GeneratedStatement,
    value: string,
    condition?: ComparisonNode,
  ): string[] {
    // In register setter helpers, suppress raises — returning from the helper
    // is not the same as returning from the decode function.
    if (generated.context.suppressRaise) {
      return [];
    }

    // Build IP rollback code so the fault frame points to the faulting
    // instruction (before any prefix bytes were consumed) rather than past it.
    let rollback = '';
    if (generated.context.ipAdvance) {
      const ip =
        this.target.fetch?.effectiveRegister ||
        (Array.isArray(this.target.fetch?.register)
          ? this.target.fetch?.register?.[0]
          : this.target.fetch?.register);
      if (ip && this.registerMap.registers[ip]) {
        const ipRef: RegisterReference = {
          type: 'register',
          identifier: ip,
          mapping: this.registerMap.registers[ip],
          size: this.registerMap.registers[ip].size,
        };
        rollback = this.writeRegister(generated, ipRef, '_ip_save') + '; ';
      }
    }

    if (condition) {
      return [
        `if (${this.fromComparison(generated, condition)[0]}) { ${rollback}this.interrupt_${generated.context.mode}(${value}); return; }`,
      ];
    }

    return [`this.interrupt_${generated.context.mode}(${value}); return;`];
  }

  readRegister(
    _generated: GeneratedStatement,
    reference: RegisterReference,
  ): string[] {
    let { size } = reference.mapping;
    const { index, offset, length } = reference.mapping;
    const { size: destSize, signed } = reference;

    // Normalize to valid TypedArray width (8, 16, or 32)
    if (size <= 8) size = 8;
    else if (size <= 16) size = 16;
    else size = 32;

    return [
      `${signed ? '(' : ''}${destSize < size ? '(' : ''}${offset !== undefined ? '((' : ''}this.mem${size}[${index}]${offset !== undefined ? `${offset ? ` >> ${offset}` : ''}) & 0x${(Math.pow(2, length || 0) - 1).toString(16)})` : ''}${destSize < size ? ` & 0x${((1 << destSize) - 1).toString(16)})` : ''}${signed ? (destSize === 32 ? ' | 0)' : ` << ${32 - destSize} >> ${32 - destSize})`) : ''}`,
    ];
  }

  readRegisters(
    generated: GeneratedStatement,
    references: RegisterReference[],
    index: ExpressionNode,
  ): string[] {
    let { size } = references[0].mapping;
    const { size: destSize, signed } = references[0];

    // Normalize to valid TypedArray width (8, 16, or 32)
    if (size <= 8) size = 8;
    else if (size <= 16) size = 16;
    else size = 32;

    const effective = this.fromExpression(generated, index)[0];

    return [
      `${signed ? '(' : ''}${destSize !== size ? '(' : ''}this.mem${size}[${effective}]${destSize !== size ? ` & 0x${((1 << destSize) - 1).toString(16)})` : ''}${signed ? (destSize === 32 ? ' | 0)' : ` << ${32 - destSize} >> ${32 - destSize})`) : ''}`,
    ];
  }

  readMemory(
    generated: GeneratedStatement,
    reference: MemoryReference,
    address: ExpressionNode,
  ): string[] {
    const { size, signed, offset, aligned } = reference;

    // Normalize to valid TypedArray width (8, 16, or 32)
    let width = size;
    if (width <= 8) width = 8;
    else if (width <= 16) width = 16;
    else width = 32;

    const effective = this.fromExpression(generated, address)[0];

    if (reference.mapping.type === 'programmable') {
      return [`this.${reference.identifier}_read(${size}, ${effective})`];
    }

    if (width === 8) {
      return [
        `${size !== width ? '(' : ''}${offset ? '(' : ''}${signed ? '(' : ''}this.mem8[${effective}]${signed ? ` << ${32 - width} >> ${32 - width})` : ''}${offset ? ` >> ${offset})` : ''}${size !== width ? ` & 0x${(Math.pow(2, size || 0) - 1).toString(16)})` : ''}`,
      ];
    } else if (width === 16) {
      if (aligned) {
        return [
          `${size !== width ? '(' : ''}${offset ? '(' : ''}${signed ? '(' : ''}this.mem16[${effective}]${signed ? ` << ${32 - width} >> ${32 - width})` : ''}${offset ? ` >> ${offset})` : ''}${size !== width ? ` & 0x${(Math.pow(2, size || 0) - 1).toString(16)})` : ''}`,
        ];
      } else {
        return [
          `${size !== width ? '(' : ''}${offset ? '(' : ''}${signed ? '(' : ''}(${effective} & 0x1 ? (this.mem16[(${effective}) >> 1] >> 8) | ((this.mem16[((${effective}) >> 1) + 1] & 0xff) << 8) : this.mem16[(${effective}) >> 1])${signed ? ` << ${32 - width} >> ${32 - width})` : ''}${offset ? ` >> ${offset})` : ''}${size !== width ? ` & 0x${(Math.pow(2, size || 0) - 1).toString(16)})` : ''}`,
        ];
      }
    } else if (width === 32) {
      if (aligned) {
        return [
          `${size !== width ? '(' : ''}${offset ? '(' : ''}${signed ? '(' : ''}this.mem32[${effective}]${signed ? ' | 0)' : ''}${size !== width ? ` & 0x${(Math.pow(2, size || 0) - 1).toString(16)})` : ''}`,
        ];
      } else {
        return [
          `${size !== width ? '(' : ''}${offset ? '(' : ''}${signed ? '(' : ''}(${effective} & 0x3 ? ((this.mem32[(${effective}) >> 2] >> (8 * (${effective} % 4))) | ((this.mem32[((${effective}) >> 2) + 1] << (8 * (4 - ${effective} % 4))) & 0xffffffff) >>> 0) : this.mem32[(${effective}) >> 2])${signed ? ' | 0)' : ''}${size !== width ? ` & 0x${(Math.pow(2, size || 0) - 1).toString(16)})` : ''}`,
        ];
      }
    }

    return [`this.mem${width}[${effective}]`];
  }

  readLocal(
    _generated: GeneratedStatement,
    reference: LocalReference,
    coercion?: string,
  ): string[] {
    const result = [`${reference.mapping.identifier}`];
    return coercion ? this.applyCoercion(result, coercion) : result;
  }

  readSystem(_generated: GeneratedStatement, identifier: string): string[] {
    return [`this.${identifier.substring(1)}`];
  }

  writeRegister(
    _generated: GeneratedStatement,
    reference: RegisterReference,
    value: string,
    exprType?: ExpressionType,
  ): string[] {
    let { size, length } = reference.mapping;
    const { index, offset } = reference.mapping;

    length ||= size;
    // Normalize to valid TypedArray width (8, 16, or 32)
    if (size <= 8) size = 8;
    else if (size <= 16) size = 16;
    else size = 32;

    // Determine if we need to mask the value to `length` bits
    const needsMask =
      size !== length && !this.canSkipCoercion(exprType, length, false);

    return [
      `this.mem${size}[${index}] = ${offset !== undefined ? `(this.mem${size}[${index}] & ~0x${(((Math.pow(2, length) - 1) << offset) >>> 0).toString(16)}) | (` : ''}${(offset || 0) === 0 ? '' : '('}${value}${needsMask ? ` & 0x${(Math.pow(2, length) - 1).toString(16)}` : ''}${(offset || 0) === 0 ? '' : `) << ${offset}`}${offset !== undefined ? ')' : ''}`,
    ];
  }

  writeRegisters(
    generated: GeneratedStatement,
    references: RegisterReference[],
    index: ExpressionNode,
    value: ExpressionNode,
  ): string[] {
    let { size } = references[0].mapping;

    // Normalize to valid TypedArray width (8, 16, or 32)
    if (size <= 8) size = 8;
    else if (size <= 16) size = 16;
    else size = 32;

    const effective = this.fromExpression(generated, index)[0];
    const result = this.fromExpression(generated, value)[0];

    return [`this.mem${size}[${effective}] = ${result}`];
  }

  writeMemory(
    generated: GeneratedStatement,
    reference: MemoryReference,
    address: ExpressionNode,
    value: string,
  ): string[] {
    const originalSize = reference.size;
    let size = originalSize;

    // Normalize to valid TypedArray width (8, 16, or 32)
    if (size <= 8) size = 8;
    else if (size <= 16) size = 16;
    else size = 32;

    const effective = this.fromExpression(generated, address)[0];
    const offset = reference.offset || 0;

    // Sub-width field: read-modify-write to preserve surrounding bits
    if (originalSize < size && originalSize > 0) {
      const fieldMask = Math.pow(2, originalSize) - 1;
      const shiftedMask = (fieldMask << offset) >>> 0;
      return [
        `this.mem8[${effective}] = (this.mem8[${effective}] & ~0x${shiftedMask.toString(16)}) | (${offset === 0 ? '' : '('}${value}${originalSize < 8 ? ` & 0x${fieldMask.toString(16)}` : ''}${offset === 0 ? '' : `) << ${offset}`})`,
      ];
    }

    if (size === 8) {
      return [`this.mem8[${effective}] = ${value}`];
    } else if (size === 16) {
      return [
        `if (${effective} & 0x1) { const _value = ${value}; this.mem8[${effective}] = _value; this.mem8[(${effective}) + 1] = _value >> 8; } else { this.mem16[(${effective}) >> 1] = ${value}; }`,
      ];
    } else if (size === 32) {
      return [
        `if (${effective} & 0x3) { const _value = ${value}; const _base = ${effective} >> 2; const _shift = (${effective} & 0x3) * 8; this.mem32[_base] = (((this.mem32[_base] & (((1 << (_shift)) - 1))) | ((_value & (1 << (32 - _shift)) - 1) << _shift)) >>> 0); this.mem32[_base + 1] = (((this.mem32[_base + 1] & (((1 << (32 - _shift)) - 1)) << _shift) | (((_value >> (32 - _shift)) & (1 << (_shift)) - 1))) >>> 0); }`,
      ];
    }

    return [`this.mem${size}[${effective}] = ${value}`];
  }

  writeLocal(
    _generated: GeneratedStatement,
    reference: LocalReference,
    value: string,
    exprType?: ExpressionType,
  ): string[] {
    if (reference.mapping.size) {
      if (
        this.canSkipCoercion(
          exprType,
          reference.mapping.size,
          !!reference.mapping.signed,
        )
      ) {
        return [`${reference.mapping.identifier} = ${value}`];
      }
      const coercion =
        (reference.mapping.signed ? 'i' : 'u') + reference.mapping.size;
      const coerced = this.applyCoercion([value], coercion)[0];
      return [`${reference.mapping.identifier} = ${coerced}`];
    }
    return [`${reference.mapping.identifier} = ${value}`];
  }

  writeSystem(
    _generated: GeneratedStatement,
    identifier: string,
    value: string,
  ): string[] {
    return [`this.${identifier.substring(1)} = ${value}`];
  }

  readGlobal(generated: GeneratedStatement, reference: Reference): string[] {
    // Check register map for registers and state
    if (reference.type === 'register') {
      const name = reference.identifier;
      if (
        !this.suppressRegisterOperation &&
        !generated.accesses.includes(name)
      ) {
        generated.accesses.push(name);
      }
      let { size } = reference.mapping;
      const { index, offset, length } = reference.mapping;

      // Normalize to valid TypedArray width (8, 16, or 32)
      if (size <= 8) size = 8;
      else if (size <= 16) size = 16;
      else size = 32;

      return [
        `${offset !== undefined ? '((' : ''}this.mem${size}[${index}]${offset !== undefined ? `${offset ? ` >> ${offset}` : ''}) & 0x${(Math.pow(2, length || 0) - 1).toString(16)})` : ''}`,
      ];
    } else if (reference.type === 'memory') {
      console.log('MEMORY REFERENCE');
    }

    // Check locals
    /*if (!(name in generated.context.localMap) && name in generated.context.locals) {
      // Allocate another name
      const variableCount = Object.keys(generated.context.localMap).length;
      generated.context.localMap[name] = {
        identifier: `v_${variableCount}`,
      };
    }

    const allocated_name = generated.context.localMap[name]?.identifier;

    if (allocated_name === undefined) {
      throw new Error(`Cannot find local ${name} when generating machine target.`);
    }

    return [allocated_name];*/
    return [];
  }

  writeGlobal(
    generated: GeneratedStatement,
    name: string,
    value: string,
  ): string[] {
    // TODO: determine locals

    if (name in this.registerMap.registers) {
      if (
        !this.suppressRegisterOperation &&
        !generated.modifies.includes(name)
      ) {
        generated.modifies.push(name);
      }

      let { size, length } = this.registerMap.registers[name];
      const { index, offset } = this.registerMap.registers[name];
      length ||= size;
      // Normalize to valid TypedArray width (8, 16, or 32)
      if (size <= 8) size = 8;
      else if (size <= 16) size = 16;
      else size = 32;

      return [
        `this.mem${size}[${index}] = ${offset !== undefined ? `(this.mem${size}[${index}] & ~0x${(((Math.pow(2, length) - 1) << offset) >>> 0).toString(16)}) | (` : ''}${(offset || 0) === 0 ? '' : '('}${value}${size !== length ? ` & 0x${(Math.pow(2, length) - 1).toString(16)}` : ''}${(offset || 0) === 0 ? '' : `) << ${offset}`}${offset !== undefined ? ')' : ''}`,
      ];
    }

    if (name in this.memoryMap) {
      const info = this.memoryMap[name];
      return [`$${name} ${info.start}`];
    }

    // Check locals
    if (
      !(name in generated.context.localMap) &&
      name in generated.context.locals
    ) {
      // Allocate another name
      const variableCount = Object.keys(generated.context.localMap).length;
      generated.context.localMap[name] = {
        identifier: `v_${variableCount}`,
      };
    }

    const allocated_name = generated.context.localMap[name]?.identifier;

    if (allocated_name === undefined) {
      throw new Error(
        `Cannot find local ${name} when generating machine target.`,
      );
    }

    return [`${allocated_name} = ${value}`];
  }

  fromNextIf(generated: GeneratedStatement, node: NextIfNode): string[] {
    return [
      `if (${this.fromComparison(generated, node.condition)[0]}) { break outer; }`,
    ];
  }

  fromIf(generated: GeneratedStatement, node: IfBlockNode): string[] {
    const body = node.body
      ? this.fromStatement(node.body, generated.context)
      : undefined;

    const elseBody = node.elseBody
      ? this.fromStatement(node.elseBody, generated.context)
      : undefined;

    return [
      `if (${this.fromComparison(generated, node.condition)[0]}) {`,
      ...(body?.code.map((line) => `  ${line}`) || []),
      ...(elseBody ? ['} else {'] : []),
      ...(elseBody?.code.map((line) => `  ${line}`) || []),
      `} // end if`,
    ];
  }

  fromLoop(generated: GeneratedStatement, node: LoopBlockNode): string[] {
    const body = node.body
      ? this.fromStatement(node.body, generated.context)
      : undefined;

    if (node.condition instanceof LoopIfNode) {
      return [
        `while (${this.fromComparison(generated, node.condition.condition)[0]}) { // ${node.name}`,
        ...(body?.code.map((line) => `  ${line}`) || []),
        `} // end ${node.name}`,
      ];
    }

    return [
      `do { // ${node.name}`,
      ...(body?.code.map((line) => `  ${line}`) || []),
      `}${node.condition?.condition ? ` while (${this.fromComparison(generated, node.condition.condition)[0]}) // end ${node.name}` : ''}`,
    ];
  }

  fromAssignment(
    generated: GeneratedStatement,
    node: AssignmentNode,
  ): string[] {
    const ret = super.fromAssignment(generated, node);
    ret[ret.length - 1] += ';';
    return ret;
  }

  fromUnaryExpression(
    generated: GeneratedStatement,
    node: UnaryExpressionNode,
  ): string[] {
    return [
      `${node.operator}${this.fromOperandExpression(generated, node.operand)[0]}`,
    ];
  }

  fromBinaryExpression(
    generated: GeneratedStatement,
    node: BinaryExpressionNode,
  ) {
    if (node.operator === '//') {
      // Integer divide
      return [
        `(${this.fromOperandExpression(generated, node.operand)[0]} / ${this.fromOperandExpression(generated, node.argument)[0]} | 0)`,
      ];
    } else if (node.operator.startsWith('~>[')) {
      // Rotation Right
      const width = parseInt(
        node.operator.substring(3, node.operator.length - 1),
      );
      return [
        `((value, amount) => ((value >> amount) | (value & ((1 << amount) - 1)) << (${width} - amount)))(${this.fromOperandExpression(generated, node.operand)[0]}, (${this.fromOperandExpression(generated, node.argument)[0]}) % ${width})`,
      ];
    } else if (
      node.operator === '>>'
    ) {
      // Right shifts might be unsigned
      // This is true if the left-hand side is unsigned already
      const unsigned = (node.operand.coercion || 'u32').startsWith('u');
      console.log(unsigned);
      return [
        `(${this.fromOperandExpression(generated, node.operand)[0]} ${node.operator}${unsigned ? '>' : ''} ${this.fromOperandExpression(generated, node.argument)[0]})`,
      ];
    } else if (
      node.operator === '&' ||
      node.operator === '|' ||
      node.operator === '<<'
    ) {
      // Need to ensure that it is an unsigned result
      return [
        `((${this.fromOperandExpression(generated, node.operand)[0]} ${node.operator} ${this.fromOperandExpression(generated, node.argument)[0]}) >>> 0)`,
      ];
    } else if (node.operator.startsWith('<~[')) {
      // Rotation Left
      const width = parseInt(
        node.operator.substring(3, node.operator.length - 1),
      );
      return [
        `((value, amount) => (((value << amount) & ((1 << ${width}) - 1)) | (value >> (${width} - amount))))(${this.fromOperandExpression(generated, node.operand)}, (${this.fromOperandExpression(generated, node.argument)}) % ${width})`,
      ];
    }

    return [
      `(${this.fromOperandExpression(generated, node.operand)[0]} ${node.operator} ${this.fromOperandExpression(generated, node.argument)[0]})`,
    ];
  }

  fromBinaryLogic(generated: GeneratedStatement, node: BinaryLogicNode) {
    return [
      `(${this.fromComparison(generated, node.operand as ComparisonNode)[0]} ${node.operator} ${this.fromComparison(generated, node.argument)[0]})`,
    ];
  }

  fromComparisonEvaluation(
    generated: GeneratedStatement,
    node: ComparisonEvaluationNode,
  ): string[] {
    return [
      `${this.fromComparisonOperand(generated, node.operand)[0]} ${node.operator === '==' ? '===' : node.operator === '!=' ? '!==' : node.operator} ${this.fromComparisonOperand(generated, node.argument)[0]}`,
    ];
  }
}

export default TypeScriptBackend;

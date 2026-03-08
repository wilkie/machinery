import Tokenizer from './Tokenizer';

import type { Target } from '@machinery/core';
import type { InstructionInfo, OpcodeMatcher } from '@machinery/core';

import Generator from './Generator';
import type { BackendClass } from './Backend';
import { TypeScriptBackend } from './backends';
import type {
  DecoderMap,
  DecoderPartial,
  InputMap,
  MemoryMap,
  RegisterMap,
} from './types';

const BYTE_MAX = 256;

/**
 * A working representation of the given Target machine.
 */
class VirtualMachine {
  readonly target: Target;
  readonly name: string;
  protected tokenizer: Tokenizer;
  readonly memory: WebAssembly.Memory;
  readonly mem8: Uint8Array;
  readonly mem16: Uint16Array;
  readonly mem32: Uint32Array;
  readonly machine: Target;
  readonly memoryMap: MemoryMap;
  readonly registerMap: RegisterMap;
  readonly linearStart: number;
  readonly decoderMap: DecoderMap;
  readonly mode: string;

  constructor(target: Target) {
    this.target = target;
    this.name = target.name;

    // Create a tokenizer
    this.tokenizer = new Tokenizer(target.macros);

    this.machine = target;
    this.mode = this.target.modes?.[0]?.identifier || 'default';

    // Create processing memory
    this.memory = new WebAssembly.Memory({ initial: 30, maximum: 1024 });
    this.mem8 = new Uint8Array(this.memory.buffer);
    this.mem16 = new Uint16Array(this.memory.buffer);
    this.mem32 = new Uint32Array(this.memory.buffer);

    // Generate register space and memory map
    this.registerMap = this.generateRegisterMap();

    const { map: memoryMap, size: romSize } = this.generateMemoryMap(
      this.registerMap.size,
    );
    this.memoryMap = memoryMap;

    // Unallocated memory (or flexible RAM) is located at the end of the defined memory
    this.linearStart = this.registerMap.size + romSize;

    // Align linearStart to 64-bit word size
    // (it will be divisible evenly by 2, 4 and 8)
    this.linearStart = (this.linearStart + 7) & -8;

    this.decoderMap = this.generateDecoderMap();
  }

  generate(
    _instruction: InstructionInfo,
    _formIndex: number,
    backendClass: BackendClass = TypeScriptBackend,
  ) {
    const g = new Generator(this, backendClass);
    g.generate();
  }

  generateMemoryMap(memoryStart: number): {
    map: MemoryMap;
    size: number;
  } {
    const machine = this.machine;
    const ret: MemoryMap = {};
    let position = memoryStart;

    // Embed Memory
    for (const memoryInfo of machine.memory || []) {
      const memoryName = memoryInfo.identifier;
      ret[memoryName] = {
        start: position,
        size: 8,
        length: memoryInfo.max || memoryInfo.min || 0,
        data: [],
        type: memoryInfo.type,
        info: memoryInfo,
      };

      for (const info of memoryInfo.regions || []) {
        const name = info.identifier;
        const tag = `${memoryName}.${name}`;
        ret[tag] = {
          start: position,
          size: info.size || 8,
          length: (info.data?.length || 0) * ((info.size || 8) / 8),
          data: info.data || [],
          info: info,
        };

        this.mem8.set(ret[tag].data, ret[tag].start);
        position += ret[tag].length;
      }
    }

    return {
      map: ret,
      size: position,
    };
  }

  generateRegisterMap(): RegisterMap {
    const machine = this.machine;
    const ret: RegisterMap = {
      start: 0,
      size: 0,
      registers: {},
    };
    let i = 0;

    for (const info of [...machine.registers, ...(machine.state || [])]) {
      const name = info.identifier;

      // Align byte index to the appropriate word
      const bytes = Math.ceil(info.size / 8);
      i = (i + bytes - 1) & -bytes;
      ret.registers[name] = {
        identifier: name,
        size: info.size,
        index: Math.floor(i / bytes),
        initialValue: info.initialValue || 0,
      };

      for (const subinfo of info.fields || []) {
        // We can only read at a word alignment (unless we slice memory in weird ways, perhaps)
        // We will come up with the most efficient way to read this data via word alignment
        // 'i' is already aligned appropriately, so worst case we read the whole value

        // Determine if this is within a byte and which one
        let index = i + Math.floor((subinfo.offset - (subinfo.offset % 8)) / 8);
        // And the leftover offset in bits
        let offset = subinfo.offset % 8;
        // And the amount we need in bits, ultimately
        let length = subinfo.size;
        const bitsNeeded = (length + offset + 7) & -8;
        let size = 8;
        if (index !== i && bitsNeeded > 8) {
          // We need to just read the whole value
          index = i;
          size = info.size;
          offset = subinfo.offset;
          length = subinfo.size;
        }

        const tag = `${name}.${subinfo.identifier}`;
        if (offset === 0 && length === size) {
          ret.registers[tag] = {
            identifier: tag,
            size,
            index,
          };
          if (subinfo.global) {
            ret.registers[subinfo.identifier] = {
              identifier: subinfo.identifier,
              size,
              index,
            };
          }
        } else {
          ret.registers[tag] = {
            identifier: tag,
            size,
            index,
            offset,
            length,
          };
          if (subinfo.global) {
            ret.registers[subinfo.identifier] = {
              identifier: subinfo.identifier,
              size,
              index,
              offset,
              length,
            };
          }
        }
      }

      i += bytes;
    }

    // Align the size to 32
    ret.size = (i + 32 - 1) & -32;
    return ret;
  }

  generateDecoderMap(prefix: boolean = false): DecoderMap {
    const machine = this.machine;

    // Go through every instruction and build out a state machine
    const decoder: DecoderMap = {
      exact: new Array(BYTE_MAX),
      partial: [],
    };

    for (const instruction of Object.values(machine.instructions || {})) {
      if (prefix && !instruction.prefix) {
        continue;
      }

      let variant_index = -1;
      for (const form of instruction.forms) {
        variant_index++;
        let pass = decoder;
        const lastOpcodeIndex = form.opcode.length - 1;
        const inputs: InputMap = {};
        for (let i = 0; i < form.opcode.length; i++) {
          let matcher: string | number | OpcodeMatcher | undefined =
            form.opcode[i];
          const matcher_name =
            typeof matcher === 'string' || typeof matcher === 'number'
              ? matcher.toString()
              : matcher.identifier;
          if (typeof matcher === 'string') {
            matcher = (machine.operands || ([] as OpcodeMatcher[])).find(
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

          if (typeof matcher === 'number') {
            pass.exact ||= new Array(BYTE_MAX);

            if (i === lastOpcodeIndex) {
              pass.exact[matcher] = {
                instruction: instruction,
                form: form,
                variant: variant_index,
                inputs: { ...inputs },
                index: i,
              };
            } else {
              pass.exact[matcher] ||= {
                partial: [],
              };
              pass = pass.exact[matcher];
            }
          } else {
            if (!matcher) {
              console.log('Error: no matcher for instruction', name, form);
              break;
            }
            let and = 0;
            let mask = 0;

            if (typeof matcher !== 'string') {
              for (const [field_name, field] of Object.entries(
                matcher?.fields || {},
              )) {
                if (field.match !== undefined) {
                  inputs[field_name] = field.match;
                  mask |= (Math.pow(2, field.size) - 1) << field.offset;
                  and |= field.match << field.offset;
                }
              }
            }

            // No mask matcher means this is an open-ended opcode
            if (mask === 0x0) {
              // This is hopefully the only time we are here
              if (!pass.partial) {
                console.log(
                  'Error: There is no way to disambiguate two instructions.',
                );
                console.log('  for: ' + instruction.name);
                console.log(' form: ' + i);
                console.log(' with: ' + matcher_name);
              } else if (pass.partial?.length !== 0) {
                console.log('Error: two opcode matchers hit the same bucket.');
              }
              delete pass.partial;
              pass.instruction = instruction;
              pass.form = form;
              pass.variant = variant_index;
              pass.index = i - 1;
              pass.inputs = { ...inputs };
              i = lastOpcodeIndex;
            } else {
              const sequence: DecoderPartial = {
                matcher,
                mask,
                and,
                map: {
                  partial: [],
                },
              };
              pass.partial?.push(sequence);
              if (i === lastOpcodeIndex) {
                sequence.map = {
                  instruction: instruction,
                  form: form,
                  variant: variant_index,
                  inputs: { ...inputs },
                  index: i,
                };
              }
              pass = sequence.map;
            }
          }
        }
      }
    }

    return decoder;
  }
}

export default VirtualMachine;

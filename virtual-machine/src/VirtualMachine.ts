import Tokenizer from './Tokenizer';

import type {
  Target,
  MemoryInfo,
  InstructionInfo,
  OpcodeMatcher,
} from '@machinery/core';

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

    const { map: memoryMap, linearStart: ramStart } = this.generateMemoryMap(
      this.registerMap.size,
    );
    this.memoryMap = memoryMap;

    // Unallocated memory (or flexible RAM) is located at the end of the defined memory
    // romSize is already an absolute position (generateMemoryMap starts from registerMap.size)
    this.linearStart = ramStart;

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
    linearStart: number;
    size: number;
  } {
    const machine = this.machine;
    const ret: MemoryMap = {};
    let position = memoryStart;
    let linearStart: number | undefined;

    // Align next memory start to 64-bit word size
    // (it will be divisible evenly by 2, 4 and 8)
    memoryStart = (memoryStart + 7) & -8;

    const systemMemory: MemoryInfo[] = [
      {
        identifier: '@system',
        name: 'System State',
        type: 'ram',
        length: 4,
        endian: 'little',
        regions: [
          {
            identifier: 'mode',
            name: 'System Mode',
            offset: 0,
            size: 8,
          },
        ],
      },
    ];

    // Embed Memory
    for (const memoryInfo of systemMemory.concat(machine.memory || [])) {
      const memoryName = memoryInfo.identifier;
      ret[memoryName] = {
        start: position,
        size: 8,
        length: memoryInfo.max || memoryInfo.min || 0,
        data: [],
        type: memoryInfo.type,
        info: memoryInfo,
      };

      if (memoryInfo.type === 'ram' && memoryInfo.length === undefined) {
        linearStart ??= position;
      }

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

      if (memoryInfo.type === 'ram' && memoryInfo.length !== undefined) {
        position += memoryInfo.length;

        // Align next memory start to 64-bit word size
        // (it will be divisible evenly by 2, 4 and 8)
        position = (position + 7) & -8;
      }
    }

    linearStart ??= 0;

    return {
      map: ret,
      size: position,
      linearStart,
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
        // Compute the last opcode index that participates in decoding:
        // trailing matchers with mask=0 (e.g., DISP, IMM) are operand data
        // read by craftInstruction, not decoder nodes.
        let lastOpcodeIndex = form.opcode.length - 1;
        while (lastOpcodeIndex > 0) {
          let entry: string | number | OpcodeMatcher | undefined =
            form.opcode[lastOpcodeIndex];
          if (typeof entry === 'number') break; // numeric literal = decodable
          if (typeof entry === 'string') {
            entry = (machine.operands || []).find(
              (o) => o.identifier === entry,
            );
          }
          if (typeof entry === 'object' && entry) {
            let hasMask = false;
            for (const field of Object.values(entry.fields || {})) {
              if (field.match !== undefined) {
                hasMask = true;
                break;
              }
            }
            if (hasMask) break; // matcher with mask = decodable
          }
          lastOpcodeIndex--;
        }
        const inputs: InputMap = {};
        for (let i = 0; i <= lastOpcodeIndex; i++) {
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
            if (mask === 0x0 && i === lastOpcodeIndex) {
              // Terminal open-ended opcode — set instruction here
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
            } else if (mask === 0x0) {
              // Non-terminal open-ended opcode (e.g., displacement byte before
              // a final opcode byte). Use a wildcard node: consume any byte
              // value and continue matching in the next map level.
              if (!pass.wildcard) {
                pass.wildcard = {
                  matcher: matcher!,
                  map: { exact: new Array(BYTE_MAX), partial: [] },
                };
              }
              pass = pass.wildcard.map;
            } else if (mask === 0xff) {
              // Full byte match — treat like a numeric literal so it merges
              // with other instructions sharing the same prefix byte.
              pass.exact ||= new Array(BYTE_MAX);

              if (i === lastOpcodeIndex) {
                pass.exact[and] = {
                  instruction: instruction,
                  form: form,
                  variant: variant_index,
                  inputs: { ...inputs },
                  index: i,
                };
              } else {
                pass.exact[and] ||= {
                  partial: [],
                };
                pass = pass.exact[and];
              }
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

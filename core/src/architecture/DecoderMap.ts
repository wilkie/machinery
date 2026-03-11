import type {
  Target,
  InstructionInfo,
  InstructionForm,
  OpcodeMatcher,
} from './Target';

const BYTE_MAX = 256;

/**
 * Maps opcode matcher field names to their matched values.
 */
export interface InputMap {
  [key: string]: number;
}

/**
 * A partial match entry in the decoder trie, using a bitmask
 * to match specific fields within an opcode byte.
 */
export interface DecoderPartial {
  matcher: string | OpcodeMatcher;
  mask: number;
  and: number;
  map: Partial<DecoderMap>;
}

/**
 * A node in the instruction decoder trie.
 *
 * The decoder walks byte-by-byte through the instruction stream.
 * At each byte, it first checks `exact[byte]` for an exact match,
 * then falls through to `partial` entries which use bitmasks.
 *
 * Terminal nodes have `instruction`, `form`, `variant`, and `inputs` set.
 */
export interface DecoderMap {
  instruction?: InstructionInfo;
  form?: InstructionForm;
  variant?: number;
  inputs?: InputMap;
  index?: number;
  exact?: DecoderMap[];
  partial?: DecoderPartial[];
}

/**
 * Build a decoder trie from a processor target's instruction definitions.
 *
 * The resulting trie can be walked at runtime to decode instructions from
 * a byte stream, or traversed at build time to generate specialized decoder code.
 *
 * @param target - The processor target whose instructions to decode.
 * @param prefix - If true, only include prefix instructions; if false, exclude them.
 */
export function generateDecoderMap(
  target: Target,
  prefix: boolean = false,
): DecoderMap {
  const decoder: DecoderMap = {
    exact: new Array(BYTE_MAX),
    partial: [],
  };

  for (const instruction of target.instructions) {
    if (prefix && !instruction.prefix) continue;
    if (!prefix && instruction.prefix) continue;

    let variantIndex = -1;
    for (const form of instruction.forms) {
      variantIndex++;
      let pass: Partial<DecoderMap> = decoder;
      const lastOpcodeIndex = form.opcode.length - 1;
      const inputs: InputMap = {};

      for (let i = 0; i < form.opcode.length; i++) {
        let matcher: string | number | OpcodeMatcher | undefined =
          form.opcode[i];
        const matcherName =
          typeof matcher === 'string' || typeof matcher === 'number'
            ? matcher.toString()
            : matcher.identifier;

        if (typeof matcher === 'string') {
          matcher = (target.operands || []).find(
            (operand) => operand.identifier === matcherName,
          );
          if (matcher === undefined) {
            throw new Error(`No operand found called ${matcherName}`);
          }
          if (typeof matcher === 'string') {
            throw new Error(
              `Operand ${matcherName} improperly aliases another matcher '${matcher}'`,
            );
          }
        }

        if (typeof matcher === 'number') {
          pass.exact ||= new Array(BYTE_MAX);

          if (i === lastOpcodeIndex) {
            pass.exact[matcher] = {
              instruction,
              form,
              variant: variantIndex,
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
          if (!matcher) break;

          let and = 0;
          let mask = 0;

          if (typeof matcher !== 'string') {
            for (const field of matcher?.fields || []) {
              if (field.match !== undefined) {
                inputs[field.identifier] = field.match;
                mask |= (Math.pow(2, field.size) - 1) << field.offset;
                and |= field.match << field.offset;
              }
            }
          }

          if (mask === 0x0) {
            // Open-ended opcode — no mask means this matcher accepts all values
            delete pass.partial;
            pass.instruction = instruction;
            pass.form = form;
            pass.variant = variantIndex;
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
                instruction,
                form,
                variant: variantIndex,
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

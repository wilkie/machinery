import type {
  Target,
  InstructionInfo,
  InstructionForm,
  InstructionFormFlat,
  InstructionFormModes,
  MacrosInfo,
  OperationViaMode,
  OperationNotViaMode,
} from '@machinery/core';

import { OperandNode } from './ast';
import type Backend from './Backend';
import type { BackendClass } from './Backend';
import Parser from './Parser';
import Resolver from './Resolver';
import type {
  IntermediateRepresentation,
  ParsedOperation,
  LocalMap,
  LocalsInfo,
} from './types';
import type VirtualMachine from './VirtualMachine';

/**
 * Generates code to emulate a given VirtualMachine using a backend code
 * generator implementation.
 */
class Generator {
  readonly backend: Backend;
  readonly target: Target;
  readonly parser: Parser;
  readonly resolver: Resolver;
  readonly map: IntermediateRepresentation;

  constructor(machine: VirtualMachine, backend: BackendClass) {
    this.target = machine.target;

    // Get a state listing as the base 'locals'
    const locals: LocalsInfo = {};

    // Prep a parser with the general macros
    this.parser = new Parser(machine.target.macros, locals);
    this.resolver = new Resolver(machine, locals);

    // Parse all instructions
    const map: IntermediateRepresentation = {
      instructions: {},
      registers: {},
      interrupts: {
        vectors: {},
      },
      system: {},
    };

    // Generate the system references
    map.system.mode = this.resolver.locateMemoryOperand(
      new OperandNode('@system', undefined, new OperandNode('mode')),
      {
        mode: 'default',
        locals: {},
        localMap: {},
      },
    );

    // Generate all instruction ASTs
    for (const info of this.target.instructions || []) {
      const name = info.identifier;

      if (name in map.instructions) {
        throw new Error(`Instruction ${name} specified twice.`);
      }

      map.instructions[name] = (info.forms || []).map((form, variant) => {
        const ret: {
          [mode: string]: {
            operation: ParsedOperation;
            finalize?: ParsedOperation;
          };
        } = {};
        for (const modeInfo of machine.target.modes || [
          { identifier: 'default' },
        ]) {
          const mode = modeInfo.identifier;
          const operation = this.parseInstructionForm(
            info,
            form,
            mode,
            variant,
            info.macros,
          );

          if (!operation) {
            throw new Error(`Instruction ${name} could not be parsed.`);
          }

          ret[mode] = {
            operation,
            finalize:
              (form as InstructionFormFlat).finalize !== undefined
                ? this.parseInstructionForm(
                    info,
                    form,
                    mode,
                    variant,
                    info.macros,
                    true,
                  )
                : undefined,
          };
        }
        return ret;
      });
    }

    // Parse all register accessors/modifiers
    for (const info of [
      ...machine.target.registers,
      ...(machine.target.state || []),
    ]) {
      const name = info.identifier;
      map.registers[name] ||= {};

      for (const modeInfo of [{ identifier: 'default' }].concat(
        machine.target.modes || [],
      )) {
        const mode = modeInfo.identifier;

        if (info.get) {
          // Get locals
          const locals: LocalsInfo = {};
          const localMap: LocalMap = {};

          for (const local of (info.get as OperationViaMode).modes?.[mode]
            ?.locals ||
            (info.get as OperationNotViaMode).locals ||
            []) {
            locals[local.identifier] = local;
          }

          const code =
            mode !== 'default'
              ? (info.get as OperationViaMode).modes?.[mode]?.operation
                  ?.flat(19)
                  .join(' ; ')
              : (info.get as OperationNotViaMode).operation
                  ?.flat(19)
                  .join(' ; ');
          if (code) {
            map.registers[name].get ||= {};
            map.registers[name].get[mode] = {
              statement: this.parser.parse(code + ' ;', {}, locals),
              localMap,
            };
            map.registers[name].get[mode].statement = this.resolver.resolve(
              map.registers[name].get[mode].statement,
              locals,
              localMap,
            );

            if (
              (info.get as OperationViaMode).modes?.[mode]?.operation ===
                undefined &&
              mode !== 'default'
            ) {
              map.registers[name].get.default = map.registers[name].get[mode];
            }
          }
        }

        if (info.set) {
          // Get locals
          const locals: LocalsInfo = {};
          const localMap: LocalMap = {};

          for (const local of (info.set as OperationViaMode).modes?.[mode]
            ?.locals ||
            (info.set as OperationNotViaMode).locals ||
            []) {
            locals[local.identifier] = local;
          }
          const code =
            mode !== 'default'
              ? (info.set as OperationViaMode).modes?.[mode]?.operation
                  ?.flat(19)
                  .join(' ; ')
              : (info.set as OperationNotViaMode).operation
                  ?.flat(19)
                  .join(' ; ') || '';
          if (code) {
            map.registers[name].set ||= {};
            map.registers[name].set[mode] = {
              statement: this.parser.parse(code + ' ;', {}, locals),
              localMap,
            };
            map.registers[name].set[mode].statement = this.resolver.resolve(
              map.registers[name].set[mode].statement,
              locals,
              localMap,
            );

            if (
              (info.set as OperationViaMode).modes?.[mode]?.operation ===
                undefined &&
              mode !== 'default'
            ) {
              map.registers[name].set.default = map.registers[name].set[mode];
            }
          }
        }
      }
    }

    // Parse interrupt vector handlers
    if (machine.target.interrupts?.handler) {
      for (const modeInfo of [{ identifier: 'default' }].concat(
        machine.target.modes || [],
      )) {
        const mode = modeInfo.identifier;

        const locals: LocalsInfo = {};
        const localMap: LocalMap = {};

        for (const local of [
          ...((machine.target.interrupts.handler as OperationNotViaMode)
            .locals || []),
          ...((machine.target.interrupts.handler as OperationViaMode).modes?.[
            mode
          ]?.locals || []),
        ]) {
          locals[local.identifier] = local;
        }

        // Add the 'vector' identifier
        locals['vector'] = {
          identifier: 'vector',
          name: 'Interrupt Vector Index',
          size: 16,
        };

        const code =
          mode !== 'default'
            ? (machine.target.interrupts.handler as OperationViaMode).modes?.[
                mode
              ]?.operation
                ?.flat(19)
                .join(' ; ')
            : (
                machine.target.interrupts.handler as OperationNotViaMode
              ).operation
                ?.flat(19)
                .join(' ; ');
        const parsed = this.parser.parse((code || '') + ' ;', {}, locals);
        const statement = this.resolver.resolve(parsed, locals, localMap);
        map.interrupts.handler ||= {};
        map.interrupts.handler[mode] = {
          statement,
          localMap,
        };

        if (
          (machine.target.interrupts.handler as OperationViaMode | undefined)
            ?.modes?.[mode]?.operation === undefined &&
          mode !== 'default'
        ) {
          map.interrupts.handler.default = map.interrupts.handler[mode];
        }
      }
    }

    this.map = map;
    this.backend = new backend(machine, this.map);
  }

  generate() {
    console.log(this.backend.prologue().join('\n'));
    for (const modeInfo of this.target.modes || [{ identifier: 'default' }]) {
      const mode = modeInfo.identifier;
      console.log(this.backend.buildDecoder(mode).join('\n'));
    }
    console.log(this.backend.epilogue().join('\n'));
    //console.log(((this.map.instructions.adc[20].node as AssignmentNode).expression as TernaryExpressionNode).condition);
    //console.log(this.backend.fromNode(this.map.instructions.adc[20]));
  }

  parseInstruction(
    name: string,
    formIndex: number,
    mode: string,
    finalize?: boolean,
  ): ParsedOperation | undefined {
    const instruction = this.target.instructions.find(
      (info) => info.identifier === name,
    );
    if (!instruction) {
      throw new Error(`Instruction ${name} not found.`);
    }

    const form = instruction.forms[formIndex];
    if (!form) {
      throw new Error(`Instruction ${name} form ${formIndex} not found.`);
    }

    if (finalize && (form as InstructionFormFlat).finalize === undefined) {
      return;
    }

    return this.parseInstructionForm(
      instruction,
      form,
      mode,
      formIndex,
      instruction.macros,
      finalize,
    );
  }

  parseInstructionForm(
    info: InstructionInfo,
    form: InstructionForm,
    mode: string,
    variant: number,
    macros?: MacrosInfo,
    finalize?: boolean,
  ): ParsedOperation {
    // Get locals
    const locals: LocalsInfo = {};
    const localMap: LocalMap = {};

    for (const local of info.locals || []) {
      locals[local.identifier] = local;
    }

    for (const local of (form as InstructionFormModes).modes?.[mode]?.locals ||
      (form as InstructionFormFlat).locals ||
      []) {
      if (local.identifier in locals) {
        // We allow redefining the same local in a form
        if (
          info.locals?.find((item) => local.identifier === item.identifier) ===
          undefined
        ) {
          throw new Error(
            `Instruction form local ${local.identifier} overrides a global local`,
          );
        }
      }

      locals[local.identifier] = local;
    }

    // Parse locals from operands
    for (const matcher of form.opcode) {
      if (typeof matcher === 'string') {
        const operand = this.target.operands?.find(
          (item) => item.identifier === matcher,
        );

        if (!operand) {
          throw new Error(`Operand ${matcher} not found.`);
        }

        locals[operand.identifier] = {
          identifier: operand.identifier,
          size: operand.size,
          signed: operand.signed,
        };

        // Also add fields as locals
        for (const fieldInfo of operand.fields || []) {
          const fieldName = fieldInfo.identifier;
          locals[fieldName] = {
            identifier: fieldName,
            value: fieldInfo.value ?? fieldInfo.match,
            size: fieldInfo.size,
            signed: fieldInfo.signed,
          };
        }
      } else if (typeof matcher !== 'number' && matcher.identifier) {
        locals[matcher.identifier] = {
          identifier: matcher.identifier,
          size: matcher.size,
          signed: matcher.signed,
        };

        // Also add fields as locals
        for (const fieldInfo of matcher.fields || []) {
          const fieldName = fieldInfo.identifier;
          locals[fieldName] = {
            identifier: fieldName,
            value: fieldInfo.value ?? fieldInfo.match,
            size: fieldInfo.size,
            signed: fieldInfo.signed,
          };
        }
      }
    }

    try {
      const result = this.parser.parse(
        (finalize
          ? (form as InstructionFormModes).modes?.[mode]?.finalize ||
            (form as InstructionFormFlat).finalize ||
            []
          : (form as InstructionFormModes).modes?.[mode]?.operation ||
            (form as InstructionFormFlat).operation ||
            []
        )
          .flat(19)
          .join(' ; ') + ' ;',
        macros,
        locals,
      );
      if (result === undefined) {
        throw new Error(`Yikes`);
      }
      return {
        statement: this.resolver.resolve(result, locals, localMap),
        localMap,
      };
    } catch (err) {
      if (err instanceof Error) {
        console.error(
          `Parsing error: instruction ${info.identifier}[${variant}]`,
        );
        console.error(err);
      }
      throw err;
    }
  }
}

export default Generator;

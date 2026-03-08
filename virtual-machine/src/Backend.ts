import type { Target } from '@machinery/core';

import type VirtualMachine from './VirtualMachine';

import {
  ArrayAccessNode,
  AssignmentNode,
  BinaryExpressionNode,
  BinaryLogicNode,
  CallExpressionNode,
  ChoiceExpressionNode,
  CommentNode,
  ComparisonNode,
  ComparisonEvaluationNode,
  ExpressionNode,
  IfBlockNode,
  LocalOperandNode,
  LoopBlockNode,
  LoopIfNode,
  MemoryOperandNode,
  NextIfNode,
  Node,
  OperandNode,
  RaiseExpressionNode,
  RegisterChoiceExpressionNode,
  RegisterOperandNode,
  StatementNode,
  TernaryExpressionNode,
  UnaryExpressionNode,
} from './ast';
import type {
  DecoderMap,
  IntermediateRepresentation,
  MemoryMap,
  RegisterMap,
  MemoryReference,
  RegisterReference,
  LocalReference,
  GeneratorContext,
  GeneratedStatement,
} from './types';

/**
 * Abstract class that implements a machine backend when transforming the intermediate AST.
 */
class Backend {
  readonly machine: VirtualMachine;
  readonly parsed: IntermediateRepresentation;
  protected suppressRegisterOperation: boolean = false;

  constructor(machine: VirtualMachine, parsed: IntermediateRepresentation) {
    this.machine = machine;
    this.parsed = parsed;
  }

  get target(): Target {
    return this.machine.target;
  }

  get memoryMap(): MemoryMap {
    return this.machine.memoryMap;
  }

  get registerMap(): RegisterMap {
    return this.machine.registerMap;
  }

  get decoderMap(): DecoderMap {
    return this.machine.decoderMap;
  }

  prologue(): string[] {
    return [];
  }

  epilogue(): string[] {
    return [];
  }

  raise(
    generated: GeneratedStatement,
    value: string,
    condition?: ComparisonNode,
  ): string[] {
    return [
      `raise ${value}${condition ? ` if ${this.fromComparison(generated, condition)}` : ''}`,
    ];
  }

  fromComparison(
    generated: GeneratedStatement,
    node: ComparisonNode,
  ): string[] {
    if (node instanceof ComparisonEvaluationNode) {
      return this.fromComparisonEvaluation(generated, node);
    } else if (node instanceof BinaryLogicNode) {
      return this.fromBinaryLogic(generated, node);
    }

    return this.fromComparisonOperand(generated, node.operand);
  }

  fromComparisonOperand(
    generated: GeneratedStatement,
    node: ComparisonNode | ExpressionNode | OperandNode,
  ): string[] {
    if (node instanceof ExpressionNode) {
      return this.fromExpression(generated, node);
    } else if (node instanceof OperandNode) {
      return this.fromOperand(generated, node);
    }

    return this.fromComparison(generated, node);
  }

  fromNode(generated: GeneratedStatement, node: Node): string[] {
    if (node instanceof IfBlockNode) {
      return this.fromIf(generated, node);
    } else if (node instanceof LoopBlockNode) {
      return this.fromLoop(generated, node);
    } else if (node instanceof ExpressionNode) {
      return this.fromExpression(generated, node);
    } else if (node instanceof ComparisonNode) {
      return this.fromComparison(generated, node);
    } else if (node instanceof OperandNode) {
      return this.fromOperand(generated, node);
    } else if (node instanceof AssignmentNode) {
      return this.fromAssignment(generated, node);
    } else if (node instanceof NextIfNode) {
      return this.fromNextIf(generated, node);
    } else if (node instanceof CommentNode) {
      return this.comment('');
    }

    return [];
  }

  comment(message: string): string[] {
    return [`; ${message}`];
  }

  fromAccessors(generated: GeneratedStatement): string[] {
    const ret: string[] = [];
    for (const name of generated.accesses) {
      const parsedRegister = this.parsed.registers[name];
      if (parsedRegister?.get) {
        this.suppressRegisterOperation = true;
        // TODO: handle get() for accessors listed in here that aren't in the current list
        const node =
          parsedRegister.get[generated.context.mode] ||
          parsedRegister.get.default;
        if (node) {
          const getStatement = this.fromStatement(node);
          for (const line of this.comment(`${name}.get()`)) {
            ret.push(line);
          }
          for (const line of getStatement.code) {
            ret.push(line);
          }
        }
        this.suppressRegisterOperation = false;
      }
    }

    return ret;
  }

  fromModifies(generated: GeneratedStatement): string[] {
    const ret: string[] = [];
    for (const name of generated.modifies) {
      const parsedRegister = this.parsed.registers[name];
      if (parsedRegister?.set) {
        this.suppressRegisterOperation = true;
        // TODO: handle get/set() for accessors listed in here that aren't in the current list
        const node =
          parsedRegister.set[generated.context.mode] ||
          parsedRegister.set.default;
        if (node) {
          const setStatement = this.fromStatement(node);
          for (const line of this.comment(`${name}.set()`)) {
            ret.push(line);
          }
          for (const line of setStatement.code) {
            ret.push(line);
          }
        }
        this.suppressRegisterOperation = false;
      }
    }

    return ret;
  }

  fromStatement(
    node: StatementNode,
    context?: GeneratorContext,
  ): GeneratedStatement {
    const generated: GeneratedStatement = {
      accesses: [],
      modifies: [],
      code: [],
      context: context || {
        mode: this.target.modes?.[0]?.identifier || 'default',
        locals: {},
        localMap: {},
      },
    };

    const lines = this.fromNode(generated, node.node);

    // Prepend accessors...
    for (const line of this.fromAccessors(generated)) {
      generated.code.push(line);
    }

    // Output generated code
    for (const line of lines) {
      generated.code.push(line);
    }

    // Append modifiers
    for (const line of this.fromModifies(generated)) {
      generated.code.push(line);
    }

    // Get the next statement
    const next: GeneratedStatement | undefined = node.next
      ? this.fromStatement(node.next, generated.context)
      : undefined;

    // Add the following statement's code to our own
    for (const line of next?.code || []) {
      generated.code.push(line);
    }

    return generated;
  }

  fromIf(generated: GeneratedStatement, node: IfBlockNode): string[] {
    const body = node.body
      ? this.fromStatement(node.body, generated.context)
      : undefined;

    return [
      `if ${this.fromComparison(generated, node.condition)[0]} {`,
      ...(body?.code.map((line) => `  ${line}`) || []),
      `}`,
    ];
  }

  fromLoop(generated: GeneratedStatement, node: LoopBlockNode): string[] {
    const body = node.body
      ? this.fromStatement(node.body, generated.context)
      : undefined;

    if (node.condition instanceof LoopIfNode) {
      return [
        `loop until ${this.fromComparison(generated, node.condition.condition)[0]} { // ${node.name}`,
        ...(body?.code.map((line) => `  ${line}`) || []),
        `} // end ${node.name}`,
      ];
    }

    return [
      'loop {',
      ...(body?.code.map((line) => `  ${line}`) || []),
      `}${node.condition?.condition ? ` until (${this.fromComparison(generated, node.condition.condition)})` : ''}`,
    ];
  }

  fromTernaryExpression(
    generated: GeneratedStatement,
    node: TernaryExpressionNode,
  ): string[] {
    return [
      `(${this.fromComparison(generated, node.condition)[0]} ? ${this.fromOperandExpression(generated, node.operand)[0]} : ${this.fromExpression(generated, node.whenFalse)[0]})`,
    ];
  }

  fromComparisonEvaluation(
    _generated: GeneratedStatement,
    _node: ComparisonEvaluationNode,
  ): string[] {
    return [];
  }

  readRegister(
    _generated: GeneratedStatement,
    reference: RegisterReference,
  ): string[] {
    return [`mem[${reference.mapping}]`];
  }

  readRegisters(
    _generated: GeneratedStatement,
    _references: RegisterReference[],
    _index: ExpressionNode,
  ): string[] {
    return [''];
  }

  readMemory(
    _generated: GeneratedStatement,
    reference: MemoryReference,
    _address: ExpressionNode,
  ): string[] {
    return [`mem[${reference.mapping}]`];
  }

  readLocal(
    _generated: GeneratedStatement,
    reference: LocalReference,
  ): string[] {
    return [`${reference.mapping.identifier}`];
  }

  readGlobals(
    _generated: GeneratedStatement,
    entries: (number | string)[],
    index: string,
  ): string[] {
    return [`[${entries.join(',')}][${index}]`];
  }

  writeRegister(
    _generated: GeneratedStatement,
    reference: RegisterReference,
    value: string,
  ): string[] {
    return [`mem[${reference.mapping}] = ${value}`];
  }

  writeRegisters(
    _generated: GeneratedStatement,
    _references: RegisterReference[],
    _index: ExpressionNode,
    _value: ExpressionNode,
  ): string[] {
    return [''];
  }

  writeMemory(
    _generated: GeneratedStatement,
    reference: MemoryReference,
    _address: ExpressionNode,
    value: string,
  ): string[] {
    return [`mem[${reference.mapping}] = ${value}`];
  }

  writeLocal(
    _generated: GeneratedStatement,
    reference: LocalReference,
    value: string,
  ): string[] {
    return [`${reference.mapping.identifier} = ${value}`];
  }

  write(
    generated: GeneratedStatement,
    node: OperandNode,
    value: string,
  ): string[] {
    if (typeof node.value === 'number') {
      throw new Error(`Cannot assign to a constant value.`);
    }

    if (node instanceof LocalOperandNode) {
      return this.writeLocal(generated, node.reference, value);
    } else if (node instanceof MemoryOperandNode) {
      return this.writeMemory(
        generated,
        node.reference,
        node.reference.address,
        value,
      );
    } else if (node instanceof RegisterOperandNode) {
      const { identifier } = node.reference.mapping;
      if (
        !this.suppressRegisterOperation &&
        !generated.modifies.includes(identifier)
      ) {
        generated.modifies.push(identifier);
      }

      return this.writeRegister(generated, node.reference, value);
    }

    console.log(node);
    throw new Error(`Non dereferenced operand cannot be processed.`);
  }

  writeGlobals(
    _generated: GeneratedStatement,
    entries: (number | string)[],
    index: string,
    value: string,
  ): string[] {
    return [`[${entries.join(',')}][${index}] = ${value}`];
  }

  fromOperandExpression(
    generated: GeneratedStatement,
    node: OperandNode | ExpressionNode,
  ): string[] {
    if (node instanceof OperandNode) {
      return this.fromOperand(generated, node);
    }

    return this.fromExpression(generated, node);
  }

  fromOperand(generated: GeneratedStatement, node: OperandNode): string[] {
    if (typeof node.value === 'number') {
      return [node.value.toString()];
    }

    if (node instanceof LocalOperandNode) {
      return this.readLocal(generated, node.reference);
    } else if (node instanceof MemoryOperandNode) {
      return this.readMemory(generated, node.reference, node.reference.address);
    } else if (node instanceof RegisterOperandNode) {
      const { identifier } = node.reference.mapping;
      if (
        !this.suppressRegisterOperation &&
        !generated.accesses.includes(identifier)
      ) {
        generated.accesses.push(identifier);
      }
      return this.readRegister(generated, node.reference);
    }

    console.log(node);
    throw new Error(`Non dereferenced operand cannot be processed.`);
  }

  fromExpression(
    generated: GeneratedStatement,
    node: ExpressionNode,
  ): string[] {
    if (node instanceof RaiseExpressionNode) {
      return this.fromRaiseExpression(generated, node);
    }

    if (node instanceof CallExpressionNode) {
      return this.fromCallExpression(generated, node);
    }

    if (node instanceof RegisterChoiceExpressionNode) {
      return this.fromRegisterChoiceExpression(generated, node);
    }

    if (node instanceof ChoiceExpressionNode) {
      return this.fromChoiceExpression(generated, node);
    }

    if (node instanceof UnaryExpressionNode) {
      return this.fromUnaryExpression(generated, node);
    }

    if (node instanceof BinaryExpressionNode) {
      return this.fromBinaryExpression(generated, node);
    }

    if (node instanceof TernaryExpressionNode) {
      return this.fromTernaryExpression(generated, node);
    }

    if (node instanceof OperandNode) {
      return this.fromOperand(generated, node);
    }

    if (node.operand instanceof OperandNode) {
      return this.fromOperand(generated, node.operand);
    }

    return this.fromExpression(generated, node.operand);
  }

  fromNextIf(_generated: GeneratedStatement, _node: NextIfNode): string[] {
    return [];
  }

  fromAssignment(
    generated: GeneratedStatement,
    node: AssignmentNode,
  ): string[] {
    if (node.destination instanceof OperandNode) {
      const expressionCode = this.fromExpression(generated, node.expression);
      return [
        ...expressionCode.slice(0, expressionCode.length - 1),
        ...this.write(
          generated,
          node.destination,
          expressionCode[expressionCode.length - 1],
        ),
      ];
    }

    if (node.destination instanceof RegisterChoiceExpressionNode) {
      const choice: RegisterChoiceExpressionNode = node.destination;
      const choices: RegisterOperandNode[] =
        choice.choices as RegisterOperandNode[];

      // TODO: if there is a pattern, we should try to conditionally handle it
      for (const entry of choices) {
        if (entry.reference.type === 'register') {
          if (!generated.modifies.includes(entry.reference.identifier)) {
            generated.modifies.push(entry.reference.identifier);
          }
        }
      }

      return this.writeRegisters(
        generated,
        choices.map((choice) => choice.reference),
        choice.index,
        node.expression,
      );
    } else if (node.destination instanceof ChoiceExpressionNode) {
      /*
      const choice: ChoiceExpressionNode = node.destination;
      const entries = choice.choices.map((choiceNode) => {
        return choiceNode.value;
      });
      const choiceCode = this.fromNode(generated, choice.index.operand);
      const expressionCode = this.fromExpression(generated, node.expression);
      return [
        ...choiceCode.slice(0, choiceCode.length - 1),
        ...expressionCode.slice(0, expressionCode.length - 1),
        ...this.writeGlobals(generated, entries, choiceCode[choiceCode.length - 1], expressionCode[expressionCode.length - 1]),
      ];*/
    }

    return [];
  }

  fromUnaryExpression(
    generated: GeneratedStatement,
    node: UnaryExpressionNode,
  ): string[] {
    return [
      `${node.operator} ${this.fromOperandExpression(generated, node.operand)[0]}`,
    ];
  }

  fromBinaryExpression(
    generated: GeneratedStatement,
    node: BinaryExpressionNode,
  ): string[] {
    return [
      `${this.fromOperandExpression(generated, node.operand)[0]} ${node.operator} ${this.fromOperandExpression(generated, node.argument)[0]}`,
    ];
  }

  fromBinaryLogic(
    generated: GeneratedStatement,
    node: BinaryLogicNode,
  ): string[] {
    return [
      `${this.fromComparison(generated, node.operand as ComparisonNode)[0]} ${node.operator} ${this.fromComparison(generated, node.argument)[0]}`,
    ];
  }

  fromRaiseExpression(
    generated: GeneratedStatement,
    node: RaiseExpressionNode,
  ): string[] {
    return this.raise(
      generated,
      this.fromNode(generated, node.operand)[0],
      node.condition,
    );
  }

  fromArray(
    _generated: GeneratedStatement,
    _operand: OperandNode,
    _node: ArrayAccessNode,
  ): string[] {
    return [];
  }

  fromCallExpression(
    _generated: GeneratedStatement,
    _node: CallExpressionNode,
  ): string[] {
    return [];
  }

  fromRegisterChoiceExpression(
    generated: GeneratedStatement,
    node: RegisterChoiceExpressionNode,
  ): string[] {
    const choices: RegisterOperandNode[] =
      node.choices as RegisterOperandNode[];

    // TODO: if there is a pattern, we should try to conditionally handle it
    for (const entry of choices) {
      if (entry.reference.type === 'register') {
        if (!generated.accesses.includes(entry.reference.identifier)) {
          generated.accesses.push(entry.reference.identifier);
        }
      }
    }

    return this.readRegisters(
      generated,
      choices.map((choice) => choice.reference),
      node.index,
    );
  }

  fromChoiceExpression(
    _generated: GeneratedStatement,
    _node: ChoiceExpressionNode,
  ): string[] {
    throw new Error(`I dunno`);
  }

  buildDecoder(_mode: string): string[] {
    return [];
  }
}

export type BackendClass = new (
  machine: VirtualMachine,
  parsed: IntermediateRepresentation,
) => Backend;

export default Backend;

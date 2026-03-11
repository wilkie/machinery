export interface ExpressionNode {
  type: 'expression';
  op: '+' | '-' | '*' | '/' | '&' | '|' | '~';
  left: ExpressionValue;
  right: ExpressionValue;
}

export type ExpressionValue = number | string | ExpressionNode;

export interface FarPointer {
  seg: ExpressionValue;
  off: ExpressionValue;
}

export type ResolvableValue = ExpressionValue | FarPointer;

export interface RegisterOperand {
  type: 'register';
  name: string;
  size: number;
}

export interface MemoryOperand {
  type: 'memory';
  base?: string;
  index?: string;
  displacement?: number | ExpressionValue;
  size?: number;
  segment?: string;
  near?: boolean;
  far?: boolean;
}

export interface ImmediateOperand {
  type: 'immediate';
  value: ResolvableValue;
  short?: boolean;
  far?: boolean;
}

export type Operand = RegisterOperand | MemoryOperand | ImmediateOperand;

export interface InstructionNode {
  type: 'instruction';
  mnemonic: string;
  operands: Operand[];
  sizeHint?: number;
}

export interface LabelNode {
  type: 'label';
  name: string;
}

export interface DirectiveNode {
  type: 'directive';
  name: string;
  args: (number | string | ExpressionValue)[];
}

export type ASTNode = InstructionNode | LabelNode | DirectiveNode;

export type Program = ASTNode[];

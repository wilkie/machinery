export { Decoder, disassemble } from './runtime/Decoder';
export { IntelSyntax } from './runtime/IntelSyntax';
export { AttSyntax } from './runtime/AttSyntax';
export type { Syntax } from './runtime/Syntax';
export type {
  DecodedInstruction,
  DecodedOperand,
  RegisterOperand,
  MemoryOperand,
  ImmediateOperand,
  RelativeOperand,
  FarPointerOperand,
} from './runtime/types';

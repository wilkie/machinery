// @machinery/machines — parser and analyzer for .machine files.
//
// This package reads the .machine DSL (hardware description language used to
// describe processors and devices) and produces an analyzed representation
// suitable for downstream tools: assembler/disassembler generation, emulator
// generation, and cycle-accurate simulation.
//
// See core/ALU_REVIEW.md and core/ALU_WALK.md for the language design and an
// end-to-end example of how the inference pass derives instruction metadata
// from a routine's microcode body.

export { lex, type LexResult } from './lexer/lex.js';
export { injectIndentation } from './lexer/indent.js';
export * as tokens from './lexer/tokens.js';

export {
  parse,
  parseExpression,
  parseStatement,
  getDeclarations,
  getEnums,
  getBundles,
  getUnions,
  getRegisters,
  getMicrowords,
  getOperands,
  getRoutines,
  type ParseResult,
  type TopLevelDecl,
  type DeclarationKind,
  type EnumInfo,
  type BundleInfo,
  type BundleField,
  type UnionInfo,
  type UnionArm,
  type RegisterInfo,
  type FieldInfo,
  type MicrowordInfo,
  type OperandInfo,
  type OperandField,
  type RoutineInfo,
} from './parser/parse.js';
export { MachineParser, machineParser } from './parser/parser.js';

// Semantic phase — slice 1: AST lowering.
export { lowerFile } from './semantic/lower.js';
export type {
  SourceLocation,
  File as AstFile,
  Declaration as AstDeclaration,
  EnumDeclaration as AstEnumDeclaration,
  EnumVariant as AstEnumVariant,
  BundleDeclaration as AstBundleDeclaration,
  UnionDeclaration as AstUnionDeclaration,
  NamedField as AstNamedField,
  TypeRef as AstTypeRef,
  TypeRefSimple as AstTypeRefSimple,
} from './semantic/ast.js';
export {
  isEnumDeclaration,
  isBundleDeclaration,
  isUnionDeclaration,
} from './semantic/ast.js';

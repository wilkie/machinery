// Token definitions for the .machine DSL.
//
// The lexer produces a raw token stream which is then handed to
// `injectIndentation` (see ./indent.ts) to add INDENT/OUTDENT tokens at
// line boundaries. The parser consumes the post-processed stream and
// treats indentation as ordinary tokens.

import { createToken, Lexer, type TokenType } from 'chevrotain';

// ---- Trivia ----------------------------------------------------------------

/** Horizontal whitespace. Skipped — the indent processor uses the column of
 *  the first non-whitespace token on a line rather than counting spaces. */
export const Spaces = createToken({
  name: 'Spaces',
  pattern: /[ \t]+/,
  group: Lexer.SKIPPED,
});

/** Line comment starting with `;`. Skipped. */
export const Comment = createToken({
  name: 'Comment',
  pattern: /;[^\n]*/,
  group: Lexer.SKIPPED,
});

/** End-of-line. NOT skipped — the indent processor needs line boundaries. */
export const Newline = createToken({
  name: 'Newline',
  pattern: /\r?\n/,
  line_breaks: true,
});

// ---- Synthetic indent tokens ----------------------------------------------

/** Injected by the indent processor when a line is more indented than the
 *  previous non-blank line. Never produced by the raw lexer. */
export const Indent = createToken({
  name: 'Indent',
  pattern: Lexer.NA,
});

/** Injected by the indent processor when a line is less indented than the
 *  previous non-blank line. One per level popped from the indent stack. */
export const Outdent = createToken({
  name: 'Outdent',
  pattern: Lexer.NA,
});

// ---- Identifiers and keywords ---------------------------------------------

export const Identifier = createToken({
  name: 'Identifier',
  pattern: /[A-Za-z_][A-Za-z0-9_]*/,
});

function keyword(name: string, lexeme: string): TokenType {
  return createToken({
    name,
    // Word boundary on the right so `unit` doesn't match the `unit` in
    // `unitX`; longer_alt defers to Identifier when a longer word-like
    // match is possible.
    pattern: new RegExp(lexeme),
    longer_alt: Identifier,
  });
}

// Top-level construct keywords.
export const Unit = keyword('Unit', 'unit');
export const Machine = keyword('Machine', 'machine');
export const Register = keyword('Register', 'register');
export const Enum = keyword('Enum', 'enum');
export const Bundle = keyword('Bundle', 'bundle');
export const Union = keyword('Union', 'union');
export const Microword = keyword('Microword', 'microword');
export const Operand = keyword('Operand', 'operand');
export const Routine = keyword('Routine', 'routine');

// Routine-level directives that take an identifier argument.
export const Call = keyword('Call', 'call');
export const Fetch = keyword('Fetch', 'fetch');

// Block-level keywords used inside declaration bodies.
export const Field = keyword('Field', 'field');

// Section markers used inside microword (and later operand / routine)
// bodies. Reserved globally by the lexer; meaningful only in the contexts
// the grammar places them.
export const Description = keyword('Description', 'description');
export const Fields = keyword('Fields', 'fields');
export const Ready = keyword('Ready', 'ready');
export const Effect = keyword('Effect', 'effect');

// Control-flow keywords.
export const Mux = keyword('Mux', 'mux');
export const When = keyword('When', 'when');
export const If = keyword('If', 'if');
export const Elif = keyword('Elif', 'elif');
export const Else = keyword('Else', 'else');
export const Assert = keyword('Assert', 'assert');

// ---- Literals --------------------------------------------------------------

export const HexLiteral = createToken({
  name: 'HexLiteral',
  pattern: /0x[0-9a-fA-F]+/,
});

export const DecimalLiteral = createToken({
  name: 'DecimalLiteral',
  pattern: /[0-9]+/,
});

export const StringLiteral = createToken({
  name: 'StringLiteral',
  pattern: /"(?:\\.|[^"\\])*"/,
});

// ---- Multi-char operators -------------------------------------------------
// Declared before their single-char prefixes in the token vocabulary so
// the lexer's maximal-munch picks them up first.

export const Arrow = createToken({ name: 'Arrow', pattern: /->/ });
export const BackArrow = createToken({ name: 'BackArrow', pattern: /<-/ });
export const ColonEqual = createToken({ name: 'ColonEqual', pattern: /:=/ });
export const EqualEqual = createToken({ name: 'EqualEqual', pattern: /==/ });
export const NotEqual = createToken({ name: 'NotEqual', pattern: /!=/ });
export const LessEqual = createToken({ name: 'LessEqual', pattern: /<=/ });
export const GreaterEqual = createToken({ name: 'GreaterEqual', pattern: />=/ });
export const AndAnd = createToken({ name: 'AndAnd', pattern: /&&/ });
export const OrOr = createToken({ name: 'OrOr', pattern: /\|\|/ });
export const ShiftLeft = createToken({ name: 'ShiftLeft', pattern: /<</ });
export const ShiftRight = createToken({ name: 'ShiftRight', pattern: />>/ });
export const DotDot = createToken({ name: 'DotDot', pattern: /\.\./ });

// ---- Single-char operators and punctuation --------------------------------

export const LBrace = createToken({ name: 'LBrace', pattern: /\{/ });
export const RBrace = createToken({ name: 'RBrace', pattern: /\}/ });
export const LParen = createToken({ name: 'LParen', pattern: /\(/ });
export const RParen = createToken({ name: 'RParen', pattern: /\)/ });
export const LBracket = createToken({ name: 'LBracket', pattern: /\[/ });
export const RBracket = createToken({ name: 'RBracket', pattern: /\]/ });
export const Colon = createToken({ name: 'Colon', pattern: /:/ });
export const Comma = createToken({ name: 'Comma', pattern: /,/ });
export const Dot = createToken({ name: 'Dot', pattern: /\./ });
export const At = createToken({ name: 'At', pattern: /@/ });
export const Equal = createToken({ name: 'Equal', pattern: /=/ });
export const Less = createToken({ name: 'Less', pattern: /</ });
export const Greater = createToken({ name: 'Greater', pattern: />/ });
export const Plus = createToken({ name: 'Plus', pattern: /\+/ });
export const Minus = createToken({ name: 'Minus', pattern: /-/ });
export const Star = createToken({ name: 'Star', pattern: /\*/ });
export const Slash = createToken({ name: 'Slash', pattern: /\// });
export const Pipe = createToken({ name: 'Pipe', pattern: /\|/ });
export const Amp = createToken({ name: 'Amp', pattern: /&/ });
export const Caret = createToken({ name: 'Caret', pattern: /\^/ });
export const Tilde = createToken({ name: 'Tilde', pattern: /~/ });
export const Bang = createToken({ name: 'Bang', pattern: /!/ });
export const Question = createToken({ name: 'Question', pattern: /\?/ });

// ---- Token vocabulary ------------------------------------------------------
// Order matters for the lexer: comments before operators (so `;...` is a
// comment, not a semicolon), multi-char operators before single-char,
// keywords before Identifier.

export const allTokens: TokenType[] = [
  // Trivia and newlines.
  Comment,
  Spaces,
  Newline,

  // Synthetic indent tokens (Lexer.NA pattern, never matched by the raw
  // lexer but need to be in the vocabulary so parsers can reference them).
  Indent,
  Outdent,

  // Numeric and string literals (must come before Identifier).
  HexLiteral,
  DecimalLiteral,
  StringLiteral,

  // Keywords (longer_alt on each defers to Identifier for `unitX`-like words).
  Unit,
  Machine,
  Register,
  Enum,
  Bundle,
  Union,
  Microword,
  Operand,
  Routine,
  Call,
  Fetch,
  // `Fields` must come before `Field` so the lexer matches the longer
  // keyword first — Chevrotain uses list order to break prefix ties.
  Fields,
  Field,
  Description,
  Ready,
  Effect,
  Mux,
  When,
  If,
  Elif,
  Else,
  Assert,

  // Identifier catches everything else word-like.
  Identifier,

  // Multi-char operators before single-char.
  Arrow,
  BackArrow,
  ColonEqual,
  EqualEqual,
  NotEqual,
  LessEqual,
  GreaterEqual,
  AndAnd,
  OrOr,
  ShiftLeft,
  ShiftRight,
  DotDot,

  // Single-char punctuation and operators.
  LBrace,
  RBrace,
  LParen,
  RParen,
  LBracket,
  RBracket,
  Colon,
  Comma,
  Dot,
  At,
  Equal,
  Less,
  Greater,
  Plus,
  Minus,
  Star,
  Slash,
  Pipe,
  Amp,
  Caret,
  Tilde,
  Bang,
  Question,
];

/** The raw lexer. Call `.tokenize(source)` to get a token stream with
 *  comments and horizontal whitespace stripped. Does not produce
 *  INDENT/OUTDENT — run the output through `injectIndentation`. */
export const machineLexer = new Lexer(allTokens);

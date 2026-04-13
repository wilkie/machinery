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

/**
 * Prose trivia — punctuation characters that only appear in
 * `description` block prose (backticks for inline code, em/en dashes,
 * curly quotes). They aren't part of the grammar but they also
 * shouldn't fail the lexer. Grouped as skipped so the parser never
 * sees them.
 */
export const ProseTrivia = createToken({
  name: 'ProseTrivia',
  // U+0027 ASCII apostrophe (e.g. "prefetcher's"),
  // U+0060 backtick (inline code in prose),
  // U+2013 en dash, U+2014 em dash,
  // U+2018/U+2019 curly single quotes, U+201C/U+201D curly doubles,
  // U+2026 ellipsis.
  pattern: /['`\u2013\u2014\u2018\u2019\u201C\u201D\u2026]+/,
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
/**
 * `instance NAME : decl { port: expr, ... }` — declares a named
 * long-lived instance of a unit or machine. Legal inside unit and
 * machine bodies, both at the top of a body and inline with other
 * statements. See core/ALU_UNITS_MACHINES.md for the full semantic
 * model.
 */
export const Instance = keyword('Instance', 'instance');
export const Register = keyword('Register', 'register');
// `Registers` is the plural section marker used inside machine bodies
// (a block of per-register declarations). Declared here so both live
// in the same place, but must be listed before `Register` in
// `allTokens` for the longest-prefix lexer rule.
export const Registers = keyword('Registers', 'registers');
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
export const Wire = keyword('Wire', 'wire');
// `Wires` is the plural section marker (`wires in`, `wires out`).
// Must be listed before `Wire` in `allTokens` for longest-prefix match.
export const Wires = keyword('Wires', 'wires');
export const Default = keyword('Default', 'default');

/**
 * `rom` is the section marker for read-only build-time-populated
 * arrays inside a machine body. Used as `rom NAME:Type[]` followed
 * by an indented listing of identifiers (typically routines). See
 * core/ALU_UNITS_MACHINES.md for the semantic model.
 *
 * Soft keyword (categorized as Identifier) so files with a field
 * or local named `rom` still parse — the section dispatch checks
 * for the literal token at section-open positions in machineBody.
 */
export const Rom = createToken({
  name: 'Rom',
  pattern: /rom/,
  longer_alt: Identifier,
  categories: Identifier,
});

/**
 * `In`, `Out`, and `Id` are soft keywords — they mark specific
 * grammar positions (`wires in`, `wires out`, `id NAME`) but are
 * common short words that users naturally use as identifiers
 * (`out.empty`, `control.in`, a field named `id`, and so on).
 * Categorizing them under `Identifier` lets parser rules that do
 * `CONSUME(Identifier)` accept them transparently in
 * identifier-position contexts, while the grammar rules that need
 * the specific keyword (`CONSUME(In)`, etc.) still dispatch on them
 * unambiguously.
 */
export const In = createToken({
  name: 'In',
  pattern: /in/,
  longer_alt: Identifier,
  categories: Identifier,
});
export const Out = createToken({
  name: 'Out',
  pattern: /out/,
  longer_alt: Identifier,
  categories: Identifier,
});
export const Id = createToken({
  name: 'Id',
  pattern: /id/,
  longer_alt: Identifier,
  categories: Identifier,
});

// Section markers used inside microword / operand / routine bodies.
// `Description` is a hard keyword because machineBody's section
// dispatch would collide with the `statement` alternative if it were
// categorized as Identifier. The others are soft keywords — they
// dispatch to specific sections via `CONSUME(Fields)` /
// `CONSUME(Ready)` / etc. while also acting as identifiers when used
// as bundle field names, enum variants, member accesses, etc.
export const Description = keyword('Description', 'description');

function softKeyword(name: string, lexeme: string) {
  return createToken({
    name,
    pattern: new RegExp(lexeme),
    longer_alt: Identifier,
    categories: Identifier,
  });
}

export const Fields = softKeyword('Fields', 'fields');
export const Ready = softKeyword('Ready', 'ready');
export const Effect = softKeyword('Effect', 'effect');
export const Terminal = softKeyword('Terminal', 'terminal');
export const Entry = softKeyword('Entry', 'entry');
export const Allow = softKeyword('Allow', 'allow');
export const Modifies = softKeyword('Modifies', 'modifies');
export const References = softKeyword('References', 'references');
export const Micro = softKeyword('Micro', 'micro');

/**
 * `Size` is a soft keyword — it marks the `size:` section inside an operand
 * body, but it's also used as a plain identifier elsewhere (e.g., the
 * `size:BusSize` field in `bundle BusRequest`). Chevrotain's category
 * mechanism lets parser rules that CONSUME(Identifier) also accept Size
 * tokens, so the field-name usage still parses while operand-body grammar
 * can still dispatch on Size specifically.
 */
export const Size = createToken({
  name: 'Size',
  pattern: /size/,
  longer_alt: Identifier,
  categories: Identifier,
});

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

/**
 * `::` — the path separator / turbofish delimiter. Used at expression
 * call sites to disambiguate type arguments from `<` comparison:
 * `alu::<W>(op, a, b)` is unambiguously a parameterized call, whereas
 * `alu<W>(op, a, b)` would collide with the sequence `alu < W`, ...
 *
 * Only appears in the turbofish prefix of `callOp` today; enum member
 * access stays as `AluOp.add`, not `AluOp::add`.
 */
export const DoubleColon = createToken({
  name: 'DoubleColon',
  pattern: /::/,
});
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

/**
 * A `:` with no whitespace on either side — used exclusively to mark a
 * cast in expression position (`a:u8`, `raw:u16`, `(x + y):u8`). The
 * lookbehind/lookahead ensures the lexer picks this token over regular
 * `Colon` only when the colon is "tight" against both neighbors.
 *
 * Categorized as `Colon` so every parser rule that currently does
 * `CONSUME(Colon)` (register declarations, bundle/microword fields,
 * bit slices, record fields written without a space, etc.) still
 * matches transparently. Only the cast rule in the expression grammar
 * consumes `TightColon` specifically, so cast is the only place the
 * distinction is observable.
 *
 * This is how we avoid the cast-vs-ternary ambiguity: `a ? b : c`
 * tokenizes the separator as loose `Colon`, while `a:u8` tokenizes
 * as `TightColon` — cast can't accidentally swallow a ternary's `:`.
 */
export const TightColon = createToken({
  name: 'TightColon',
  pattern: /(?<=\S):(?=\S)/,
  categories: Colon,
});
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
  ProseTrivia,
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
  Instance,
  // `Registers` must come before `Register` so the lexer matches the
  // longer keyword first.
  Registers,
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
  // Same story: `Wires` before `Wire`.
  Wires,
  Wire,
  In,
  Out,
  Default,
  Id,
  // `Rom` is a soft keyword — must precede Identifier so the lexer
  // produces Rom tokens for bare `rom`. Parser rules that consume
  // Identifier also accept Rom via its `categories: Identifier`.
  Rom,
  Description,
  Ready,
  Effect,
  Terminal,
  Entry,
  Allow,
  Modifies,
  References,
  // `Microword` (declared earlier) must precede `Micro` for the same
  // longest-prefix-first reason as Fields/Field.
  Micro,
  // `Size` is a soft keyword; must come before Identifier so the lexer
  // produces Size tokens for bare `size`. Parser rules that consume
  // Identifier also accept Size via its `categories: Identifier`.
  Size,
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
  // `DoubleColon` must come before `TightColon` / `Colon` so the lexer
  // matches `::` as one token rather than two single colons.
  DoubleColon,
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
  // `TightColon` must come before `Colon` so the lexer tries the
  // narrower pattern first. Tokens categorized under Colon still match
  // any parser rule that CONSUME(Colon)s them.
  TightColon,
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

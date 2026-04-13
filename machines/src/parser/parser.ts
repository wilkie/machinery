// MachineParser — the Chevrotain CstParser for .machine files.
//
// Covered today:
//
//   * Every top-level declaration (register, enum, bundle, union,
//     microword, operand, unit, machine, routine) parses as a real
//     CST — no opaque body fallback remains for any declaration kind.
//
//   * A full C-family expression grammar (literals, identifiers,
//     member/call/index/cast/constructor postfix ops, unary ops,
//     binary ops with standard precedence, ternary, record literals,
//     turbofish type-parameterized calls) — see the "Expression
//     grammar" section further down.
//
//   * A full statement grammar covering `call` / `fetch` directives,
//     `mux` with when/else arms, `wire` declarations, `instance`
//     declarations (with inline or block-form port bindings),
//     `if`/`elif`/`else`, `assert`, assignments (combinational `=`
//     and register `<-`), anonymous output assignment (`= expr`),
//     and bare expression statements.
//
//   * Unit and machine body grammar with `wires in`/`wires out`,
//     `registers` (with optional nested `field` blocks), `id`,
//     `description`, and `default` sections, all interleaved freely
//     with statements.
//
//   * Parameterized type references via the `{...}` form:
//     `u{W+1}`, `Local{8}`. Declaration-site type parameters
//     (`unit alu<W:Width>`) and turbofish call sites
//     (`alu::<width>(...)`) continue to use `<...>`.
//
// The opaque `block` / `anyBodyToken` rules are kept as a safety net
// for grammar contexts we add in the future, but nothing in the
// current top-level dispatch routes to them anymore.
//
// See the comment blocks above individual rules for per-rule details.

import { CstParser, tokenMatcher } from 'chevrotain';
import {
  allTokens,
  // Keywords
  Unit,
  Machine,
  Instance,
  Register,
  Enum,
  Bundle,
  Union,
  Microword,
  Operand,
  Routine,
  Call,
  Fetch,
  Field,
  Wire,
  Wires,
  In,
  Out,
  Registers,
  Default,
  Id,
  Description,
  Fields,
  Ready,
  Effect,
  Entry,
  Allow,
  Modifies,
  References,
  Micro,
  Size,
  Mux,
  When,
  If,
  Elif,
  Else,
  Assert,
  // Structural
  Identifier,
  Newline,
  Indent,
  Outdent,
  // Literals
  HexLiteral,
  DecimalLiteral,
  StringLiteral,
  // Multi-char operators
  Arrow,
  BackArrow,
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
  // Single-char operators and punctuation
  LBrace,
  RBrace,
  LParen,
  RParen,
  LBracket,
  RBracket,
  Colon,
  TightColon,
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
} from '../lexer/tokens.js';

export class MachineParser extends CstParser {
  constructor() {
    super(allTokens, { recoveryEnabled: true });
    this.performSelfAnalysis();
  }

  // =========================================================================
  // Entry rule
  // =========================================================================

  public file = this.RULE('file', () => {
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.declaration) },
      ]);
    });
  });

  // =========================================================================
  // Declaration dispatch
  // =========================================================================

  public declaration = this.RULE('declaration', () => {
    this.OR([
      { ALT: () => this.SUBRULE(this.registerDecl) },
      { ALT: () => this.SUBRULE(this.enumDecl) },
      { ALT: () => this.SUBRULE(this.bundleDecl) },
      { ALT: () => this.SUBRULE(this.unionDecl) },
      { ALT: () => this.SUBRULE(this.unitDecl) },
      { ALT: () => this.SUBRULE(this.machineDecl) },
      { ALT: () => this.SUBRULE(this.microwordDecl) },
      { ALT: () => this.SUBRULE(this.operandDecl) },
      { ALT: () => this.SUBRULE(this.routineDecl) },
    ]);
  });

  // =========================================================================
  // Top-level declarations
  //
  // Each rule parses its header (the part before the indented body) and
  // then optionally consumes a Newline followed by an indented block.
  // Body handling varies by declaration kind:
  //
  //   - enum, bundle, union, register, microword, operand:
  //       the body is parsed by a dedicated rule that understands its
  //       internal structure (variants, named fields, field+offset,
  //       section markers, etc.).
  //
  //   - unit, machine, routine:
  //       the body is still consumed opaquely by `block` / `anyBodyToken`.
  //       A real grammar for each lands in future slices.
  // =========================================================================

  public registerDecl = this.RULE('registerDecl', () => {
    this.CONSUME(Register);
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
    this.OPTION(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.registerBody));
    });
  });

  /**
   * Indented body of a top-level register declaration: a sequence of
   * `field NAME:TYPE @ OFFSET` entries, one per line. Blank and
   * comment-only lines between fields are absorbed by the inner OR.
   *
   * The same `fieldDecl` rule will be reused for registers declared
   * inside `machine` bodies (e.g. executionUnit's nested `modrm` sub-
   * register) once machine-body grammar lands; the syntax has been
   * unified on the `@ OFFSET` form in both positions.
   */
  public registerBody = this.RULE('registerBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.fieldDecl) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * A single bit field inside a register body:
   *   `field NAME:TYPE @ OFFSET`
   *
   * The offset literal can be either decimal (`@ 8`) or hex (`@ 0x11`).
   */
  public fieldDecl = this.RULE('fieldDecl', () => {
    this.CONSUME(Field);
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
    this.OPTION(() => {
      this.CONSUME(At);
      this.OR([
        { ALT: () => this.CONSUME(DecimalLiteral) },
        { ALT: () => this.CONSUME(HexLiteral) },
      ]);
    });
  });

  public enumDecl = this.RULE('enumDecl', () => {
    this.CONSUME(Enum);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      // A declaration's header line ends with a Newline. Blank or
      // comment-only lines between the header and the body appear as
      // additional Newline tokens (the lexer skips the comment body),
      // so we have to consume 1+ Newlines before looking for INDENT.
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.enumBody));
    });
  });

  /**
   * Indented body of an enum: a sequence of identifier variants, one per
   * line. Blank lines and comment-only lines between variants are
   * tolerated (comments are stripped by the lexer; blank lines appear as
   * Newline tokens that the inner OR consumes without producing a variant).
   */
  public enumBody = this.RULE('enumBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.enumVariant) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  public enumVariant = this.RULE('enumVariant', () => {
    // `Fetch` is a hard keyword but can appear as a legitimate enum
    // variant name (e.g., `ExecuteState.fetch`). The same contextual
    // accommodation memberOp uses for `.fetch` applies here.
    this.OR([
      { ALT: () => this.CONSUME(Identifier) },
      { ALT: () => this.CONSUME(Fetch) },
    ]);
  });

  public bundleDecl = this.RULE('bundleDecl', () => {
    this.CONSUME(Bundle);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.bundleBody));
    });
  });

  /**
   * Indented body of a bundle: a sequence of `name: typeRef` fields, one
   * per line. Same Newline tolerance as enumBody — blank/comment-only
   * lines between fields are absorbed by the inner OR.
   */
  public bundleBody = this.RULE('bundleBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.namedField) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  public unionDecl = this.RULE('unionDecl', () => {
    this.CONSUME(Union);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.unionBody));
    });
  });

  /**
   * Indented body of a union: structurally identical to a bundle body
   * (each arm is `tagName: payloadType`), but kept as its own rule so
   * the CST distinguishes "bundle field" from "union arm" by node name.
   */
  public unionBody = this.RULE('unionBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.namedField) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * Shared by bundleBody, unionBody, microword fieldsSection, and operand
   * fieldsSection: `Identifier ':' typeRef ('@' offset)?`. The optional
   * bit offset only has semantic meaning for operand fields (ModRM's
   * `mod:u2 @ 6`); bundles, unions, and microword fields never use it.
   * The downstream walker decides whether to extract the offset.
   */
  public namedField = this.RULE('namedField', () => {
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
    this.OPTION(() => {
      this.CONSUME(At);
      this.OR([
        { ALT: () => this.CONSUME(DecimalLiteral) },
        { ALT: () => this.CONSUME(HexLiteral) },
      ]);
    });
  });

  public unitDecl = this.RULE('unitDecl', () => {
    this.CONSUME(Unit);
    this.CONSUME(Identifier);
    this.OPTION1(() => this.SUBRULE(this.typeParams));
    this.OPTION2(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION3(() => this.SUBRULE(this.unitBody));
    });
  });

  public machineDecl = this.RULE('machineDecl', () => {
    this.CONSUME(Machine);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.machineBody));
    });
  });

  public microwordDecl = this.RULE('microwordDecl', () => {
    this.CONSUME(Microword);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.microwordBody));
    });
  });

  /**
   * Indented body of a microword declaration. Sections can appear in any
   * order and any subset; each section is one of:
   *
   *   description       — prose, inline `description: "..."` or block form
   *   fields            — typed field list (inline `{...}` or block form)
   *   ready             — single-line readiness expression
   *   effect            — indented block of statements
   *
   * `description`, `fields`, and `ready` are parsed structurally: the
   * description's block body is still consumed opaquely via `block`, but
   * the fields list and the ready expression are fully-formed CST nodes
   * (fields via `namedField`, ready via the full expression grammar).
   * `effect` is still consumed opaquely until statement grammar arrives.
   * The walker today only surfaces the `fields` section.
   */
  public microwordBody = this.RULE('microwordBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.descriptionSection) },
        { ALT: () => this.SUBRULE(this.fieldsSection) },
        { ALT: () => this.SUBRULE(this.readyClause) },
        { ALT: () => this.SUBRULE(this.effectSection) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * A `description` section — either inline (`description: "text"`) or
   * block form (`description\n  indented prose...`). In the block form
   * the prose is consumed opaquely via the general `block` skipper; a
   * later slice can swap in structured prose extraction if we want to
   * surface descriptions in generated documentation.
   */
  public descriptionSection = this.RULE('descriptionSection', () => {
    this.CONSUME(Description);
    this.OR([
      {
        ALT: () => {
          this.CONSUME(Colon);
          this.CONSUME(StringLiteral);
        },
      },
      {
        ALT: () => {
          this.AT_LEAST_ONE(() => this.CONSUME(Newline));
          this.OPTION(() => this.SUBRULE(this.block));
        },
      },
    ]);
  });

  /**
   * A `fields` section. Two forms:
   *
   *   fields { a:T, b:U }     inline record-literal form, possibly empty
   *   fields
   *     a:T                   block form — indented namedField list
   *     b:U
   *
   * The block form is what ALU_NEW.machine's microwords actually use; the
   * inline form covers the `Retire` case where the microword has no
   * fields (`fields {}`).
   */
  public fieldsSection = this.RULE('fieldsSection', () => {
    this.CONSUME(Fields);
    this.OR([
      {
        ALT: () => {
          this.CONSUME(LBrace);
          this.OPTION1(() => {
            this.SUBRULE(this.namedField);
            this.MANY(() => {
              this.CONSUME(Comma);
              this.SUBRULE2(this.namedField);
            });
          });
          this.CONSUME(RBrace);
        },
      },
      {
        ALT: () => {
          this.AT_LEAST_ONE(() => this.CONSUME(Newline));
          this.OPTION2(() => this.SUBRULE(this.fieldsBody));
        },
      },
    ]);
  });

  /**
   * Block-form body of a `fields` section: an indented list of
   * `name:type` entries. Structurally identical to bundleBody and
   * unionBody — reuses the same `namedField` rule.
   */
  public fieldsBody = this.RULE('fieldsBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.namedField) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * A `ready: <expression>` clause. Parses the expression via the real
   * expression grammar below and then expects a terminating Newline.
   */
  public readyClause = this.RULE('readyClause', () => {
    this.CONSUME(Ready);
    this.CONSUME(Colon);
    this.SUBRULE(this.expression);
    this.CONSUME(Newline);
  });

  /**
   * An `effect` section: keyword followed by an indented block of
   * statements. Parses via the real statement grammar — assignments
   * (`FLAGS <- ...`, `dest <- prefetch.byte`), wire declarations,
   * `if` conditionals, and bare expression statements all land here.
   */
  public effectSection = this.RULE('effectSection', () => {
    this.CONSUME(Effect);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.statementBlock));
  });

  public operandDecl = this.RULE('operandDecl', () => {
    this.CONSUME(Operand);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION1(() => this.SUBRULE(this.operandBody));
    });
  });

  /**
   * Indented body of an operand declaration. Sections can appear in any
   * order and any subset; each section is one of:
   *
   *   description    — prose, inline `description: "..."` or block form
   *   size: N        — a single numeric literal (byte count)
   *   fields         — typed fields with optional @ offset (operand slots)
   *   fetch          — indented block of microword record literals
   *
   * `description`, `fields`, and `size` are parsed structurally:
   * `description`'s block body is still consumed opaquely via `block`,
   * but everything else surfaces as real CST nodes. `fetch` is still
   * consumed opaquely until statement grammar arrives.
   *
   * `descriptionSection` and `fieldsSection` are reused directly from
   * the microword body grammar — they share the same rules.
   */
  public operandBody = this.RULE('operandBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.descriptionSection) },
        { ALT: () => this.SUBRULE(this.sizeClause) },
        { ALT: () => this.SUBRULE(this.fieldsSection) },
        { ALT: () => this.SUBRULE(this.fetchSection) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * A `size: N` clause — how many instruction-stream bytes this operand
   * consumes. N can be decimal or hex (`size: 1`, `size: 0x02`).
   */
  public sizeClause = this.RULE('sizeClause', () => {
    this.CONSUME(Size);
    this.CONSUME(Colon);
    this.OR([
      { ALT: () => this.CONSUME(DecimalLiteral) },
      { ALT: () => this.CONSUME(HexLiteral) },
    ]);
    this.CONSUME(Newline);
  });

  /**
   * A `fetch` section — the block of microword record literals that
   * pull the operand's bytes out of the instruction stream. Parses
   * via the statement grammar, so each line is a `statement` (usually
   * a bare microword construction like `IStreamRead { dest: modrm }`).
   */
  public fetchSection = this.RULE('fetchSection', () => {
    this.CONSUME(Fetch);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.statementBlock));
  });

  public routineDecl = this.RULE('routineDecl', () => {
    this.CONSUME(Routine);
    this.CONSUME(Identifier);
    this.OPTION1(() => this.SUBRULE(this.paramList));
    this.OPTION2(() => this.SUBRULE(this.returnType));
    this.OPTION3(() => {
      this.AT_LEAST_ONE(() => this.CONSUME(Newline));
      this.OPTION4(() => this.SUBRULE(this.routineBody));
    });
  });

  /**
   * Indented body of a routine declaration. Sections can appear in any
   * order and any subset; each section is one of:
   *
   *   description    — prose, inline `description: "..."` or block form
   *   references     — bulleted list of `- "string"` items
   *   entry          — opcode byte(s) that dispatch to this routine
   *   allow          — bracketed list of prefix-policy identifiers
   *   modifies       — bracketed list of modified flags / state names
   *   micro          — indented statement block (opaque for now)
   *
   * `description` is reused from the microword/operand body grammar.
   * `micro`'s body is still consumed opaquely via `block`; the other
   * sections surface as real CST nodes the walker can inspect.
   *
   * Per ALU_REVIEW.md, several fields that look like metadata
   * (`operands`, `operandSize`, `modifies`, `semantic`) are expected to
   * be derived by the build-time inference pass from the microcode
   * body rather than written by the author. The grammar accepts them
   * anyway so existing files keep parsing, and so that an author who
   * wants to provide them explicitly can.
   */
  public routineBody = this.RULE('routineBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.descriptionSection) },
        { ALT: () => this.SUBRULE(this.referencesSection) },
        { ALT: () => this.SUBRULE(this.entrySection) },
        { ALT: () => this.SUBRULE(this.allowSection) },
        { ALT: () => this.SUBRULE(this.modifiesSection) },
        { ALT: () => this.SUBRULE(this.microSection) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * `entry: expr` or `entry: expr, expr, ...` — the opcode byte (or
   * byte sequence for escape-prefixed instructions) that the decode
   * table will dispatch to this routine on. For now each item is a
   * plain expression; future work will grow a richer constraint
   * language for group-instruction refinements like `ModRM.reg = 0`.
   */
  public entrySection = this.RULE('entrySection', () => {
    this.CONSUME(Entry);
    this.CONSUME(Colon);
    this.SUBRULE(this.expression);
    this.MANY(() => {
      this.CONSUME(Comma);
      this.SUBRULE2(this.expression);
    });
    this.CONSUME(Newline);
  });

  /**
   * `allow: [lock, rep, repne, ...]` — policy list declaring which
   * instruction prefixes are legal in front of this routine. The list
   * is bracketed identifiers, possibly empty.
   */
  public allowSection = this.RULE('allowSection', () => {
    this.CONSUME(Allow);
    this.CONSUME(Colon);
    this.SUBRULE(this.identifierList);
    this.CONSUME(Newline);
  });

  /**
   * `modifies: [OF, SF, ZF, ...]` — list of architectural flags or
   * registers this routine writes. Same shape as `allow`. In practice
   * the build-time inference pass derives this from the microcode
   * body; the grammar accepts it for cases where the author wants to
   * state it explicitly.
   */
  public modifiesSection = this.RULE('modifiesSection', () => {
    this.CONSUME(Modifies);
    this.CONSUME(Colon);
    this.SUBRULE(this.identifierList);
    this.CONSUME(Newline);
  });

  /**
   * Generic bracketed identifier list used by `allow` and `modifies`.
   * Possibly empty. Trailing comma not supported.
   */
  public identifierList = this.RULE('identifierList', () => {
    this.CONSUME(LBracket);
    this.OPTION(() => {
      this.CONSUME(Identifier);
      this.MANY(() => {
        this.CONSUME(Comma);
        this.CONSUME2(Identifier);
      });
    });
    this.CONSUME(RBracket);
  });

  /**
   * `references` section: a `references` keyword on its own line
   * followed by an indented block of `- "string"` bullet items. Each
   * item is one reference (typically a citation like "Intel 80286
   * Programmer's Reference Manual, ADD").
   */
  public referencesSection = this.RULE('referencesSection', () => {
    this.CONSUME(References);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.referencesBody));
  });

  public referencesBody = this.RULE('referencesBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.referenceItem) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  public referenceItem = this.RULE('referenceItem', () => {
    this.CONSUME(Minus);
    this.CONSUME(StringLiteral);
  });

  /**
   * `micro` section: the statement block containing `call`, `fetch`,
   * `mux`, and microword record literals that make up the routine's
   * execution. Parses via the statement grammar.
   */
  public microSection = this.RULE('microSection', () => {
    this.CONSUME(Micro);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.statementBlock));
  });

  // =========================================================================
  // Unit and machine body grammar
  //
  // Unit bodies are sequences of `wires in` / `wires out` port
  // declarations interleaved with statements and anonymous
  // assignments (the `= expr` form used for single-output units like
  // `parity8`).
  //
  // Machine bodies add five more sections: `id NAME` (machine tag),
  // `description` (prose), `registers` (per-register declarations
  // with optional nested field blocks), `wires in` / `wires out`
  // (same as unit bodies), and `default` (initial-state assignments).
  // Everything else is statement-level and goes through the
  // `statement` rule.
  //
  // Sections can appear in any order; the grammar doesn't enforce a
  // conventional order. Semantic analysis can later warn on
  // non-standard orderings if we want that.
  // =========================================================================

  public unitBody = this.RULE('unitBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.wiresSection) },
        { ALT: () => this.SUBRULE(this.statement) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  public machineBody = this.RULE('machineBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.idSection) },
        { ALT: () => this.SUBRULE(this.descriptionSection) },
        { ALT: () => this.SUBRULE(this.registersSection) },
        { ALT: () => this.SUBRULE(this.wiresSection) },
        { ALT: () => this.SUBRULE(this.defaultSection) },
        { ALT: () => this.SUBRULE(this.statement) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * `id NAME` — a single-line machine tag. Used by `machine
   * executionUnit\n  id i286` to declare that executionUnit
   * represents the i286 target.
   */
  public idSection = this.RULE('idSection', () => {
    this.CONSUME(Id);
    this.CONSUME(Identifier);
    this.CONSUME(Newline);
  });

  /**
   * `wires in` or `wires out` section — an indented list of port
   * declarations. Each port is `NAME:type` or `*:type` (anonymous
   * output, used by units with a single unnamed return value like
   * `parity8` and `aluSelect`).
   */
  public wiresSection = this.RULE('wiresSection', () => {
    this.CONSUME(Wires);
    this.OR([
      { ALT: () => this.CONSUME(In) },
      { ALT: () => this.CONSUME(Out) },
    ]);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.wiresBody));
  });

  public wiresBody = this.RULE('wiresBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.wirePort) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  public wirePort = this.RULE('wirePort', () => {
    this.OR([
      { ALT: () => this.CONSUME(Identifier) },
      { ALT: () => this.CONSUME(Star) },
    ]);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
  });

  /**
   * `registers` section — an indented list of register entries. Each
   * entry is `NAME:type = initialValue` optionally followed by a
   * nested `field NAME:type @ offset` block (as in `executionUnit`'s
   * `modrm` sub-register, which has three nested fields).
   */
  public registersSection = this.RULE('registersSection', () => {
    this.CONSUME(Registers);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.registersBody));
  });

  public registersBody = this.RULE('registersBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.registerEntry) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * Register entry inside a `registers` block:
   *
   *   NAME:type                            // no initial value
   *   NAME:type = initialValueExpression
   *   NAME:type = initialValueExpression
   *     field subName:subType @ offset     // optional nested field block
   *     field subName:subType @ offset
   *
   * The entry consumes its trailing Newline plus any blank Newlines,
   * then optionally consumes an indented `field ...` block if one
   * follows. That way the outer `registersBody` MANY doesn't need a
   * peek for INDENT — the decision lives inside the entry itself.
   */
  public registerEntry = this.RULE('registerEntry', () => {
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
    this.OPTION(() => {
      this.CONSUME(Equal);
      this.SUBRULE(this.expression);
    });
    this.CONSUME(Newline);
    this.MANY(() => this.CONSUME2(Newline));
    this.OPTION2(() => this.SUBRULE(this.registerBody));
  });

  /**
   * `default` section — a block of initial-value assignments for the
   * machine's output wires (record literals, constants, etc.). Reuses
   * `statementBlock` so any statement form is parseable; semantic
   * analysis can restrict to assignments only if we want.
   */
  public defaultSection = this.RULE('defaultSection', () => {
    this.CONSUME(Default);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.statementBlock));
  });

  /** `<W:Width>` or `<W:Width, T:Type>` — unit/microword type parameters. */
  public typeParams = this.RULE('typeParams', () => {
    this.CONSUME(Less);
    this.SUBRULE(this.typeParam);
    this.MANY(() => {
      this.CONSUME(Comma);
      this.SUBRULE2(this.typeParam);
    });
    this.CONSUME(Greater);
  });

  public typeParam = this.RULE('typeParam', () => {
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
  });

  /** `(mod:u2, rm:u3)` — routine parameter list, possibly empty. */
  public paramList = this.RULE('paramList', () => {
    this.CONSUME(LParen);
    this.OPTION(() => {
      this.SUBRULE(this.param);
      this.MANY(() => {
        this.CONSUME(Comma);
        this.SUBRULE2(this.param);
      });
    });
    this.CONSUME(RParen);
  });

  public param = this.RULE('param', () => {
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
  });

  /**
   * `-> (ea:u16)` or `-> ()` — routine return type. The `typeRef` branch
   * allows an unparenthesized form but none of our sample files use it;
   * we keep it for symmetry with parameters.
   */
  public returnType = this.RULE('returnType', () => {
    this.CONSUME(Arrow);
    this.OR([
      {
        ALT: () => {
          this.CONSUME(LParen);
          this.OPTION(() => {
            this.SUBRULE(this.returnField);
            this.MANY(() => {
              this.CONSUME(Comma);
              this.SUBRULE2(this.returnField);
            });
          });
          this.CONSUME(RParen);
        },
      },
      { ALT: () => this.SUBRULE(this.typeRef) },
    ]);
  });

  public returnField = this.RULE('returnField', () => {
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.typeRef);
  });

  /**
   * Type reference: an identifier, optionally parameterized with an
   * expression in curly braces — `u{W+1}`, `Local{8}`, `u{W}`. Curly
   * braces (not angle brackets) are used for use-site type parameters
   * because expressions inside `<...>` collide with the `<`/`>`
   * relational operators (same problem that gave us `::<>` turbofish
   * at call sites). Declaration-site type parameters on units
   * (`unit alu<W:Width>`) and turbofish call sites
   * (`alu::<width>(...)`) keep `<...>` — they don't embed
   * arbitrary expressions, so the collision doesn't arise there.
   *
   * typeRef is only called from type positions (cast targets, field
   * type annotations, register type annotations, return-type
   * specifications, etc.) so there's no ambiguity with the
   * expression-level `constructOp` which also consumes `{...}`: those
   * are in different parse contexts and Chevrotain dispatches them
   * independently based on which rule is invoked.
   */
  public typeRef = this.RULE('typeRef', () => {
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.CONSUME(LBrace);
      this.SUBRULE(this.expression);
      this.CONSUME(RBrace);
    });
  });

  // =========================================================================
  // Opaque block skipper
  //
  // `block` consumes `INDENT ... OUTDENT`, recursively handling nested
  // blocks. The body's contents are matched by `anyBodyToken`, which is
  // an OR over every non-structural token so the skeleton doesn't have to
  // know the body grammar. When a real body grammar for a specific
  // declaration lands, that declaration's rule will call its own body
  // rule instead of `block`.
  // =========================================================================

  public block = this.RULE('block', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.SUBRULE(this.block) },
        { ALT: () => this.SUBRULE(this.anyBodyToken) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  // =========================================================================
  // Statement grammar
  //
  // Statements appear inside routine `micro` blocks, operand `fetch`
  // blocks, and microword `effect` blocks. The supported forms are:
  //
  //   callStmt          `call NAME(args) -> dest`
  //   fetchStmt         `fetch NAME`
  //   wireDeclStmt      `wire NAME[:type] = expr`
  //   ifStmt            `if EXPR\n  ... elif EXPR\n  ... else\n  ...`
  //   assertStmt        `assert EXPR`
  //   muxStmt           `mux EXPR\n  when V ...\n  ... else ...`
  //   exprOrAssignStmt  any expression, possibly followed by
  //                     `= expr` or `<- expr` for combinational and
  //                     register-next-state assignments respectively
  //
  // `exprOrAssignStmt` handles both bare expressions (like microword
  // record literals `AluMicro { ... }`) and assignments (`FLAGS <-
  // applyFlags(...)`). The parser can't distinguish them from the
  // first token alone — LHS and a bare expression both start with an
  // expression — so the rule parses an expression first, then checks
  // whether `=` or `<-` follows. Neither is an expression-level
  // operator, so the lookahead is unambiguous after the LHS parses.
  //
  // `statementBlock` is the shared indented block used by routine
  // micro, operand fetch, microword effect, and all control-flow arm
  // bodies. It's a MANY over `(Newline | statement)` so blank and
  // comment-only lines between statements are absorbed naturally.
  //
  // Not yet covered:
  //   * Unit and machine bodies (`wires in`/`wires out` sections,
  //     `registers` declarations, `default` clauses). These need new
  //     section markers on top of the current statement grammar.
  // =========================================================================

  public statement = this.RULE('statement', () => {
    this.OR([
      { ALT: () => this.SUBRULE(this.callStmt) },
      { ALT: () => this.SUBRULE(this.fetchStmt) },
      { ALT: () => this.SUBRULE(this.wireDeclStmt) },
      { ALT: () => this.SUBRULE(this.instanceStmt) },
      { ALT: () => this.SUBRULE(this.ifStmt) },
      { ALT: () => this.SUBRULE(this.assertStmt) },
      { ALT: () => this.SUBRULE(this.anonAssignStmt) },
      // `muxStmt` is intentionally NOT here — it's reachable via
      // `exprOrAssignStmt → expression → primary → muxStmt`, which
      // also makes it work as a value-producing form inside wire RHS
      // expressions like `wire raw = mux op when X: expr when Y: expr`.
      { ALT: () => this.SUBRULE(this.exprOrAssignStmt) },
    ]);
  });

  /**
   * Indented block of statements, shared by routine micro sections,
   * operand fetch sections, and mux arm bodies. Newlines between
   * statements (and blank lines) are consumed by the inner OR.
   */
  public statementBlock = this.RULE('statementBlock', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.statement) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  /**
   * Routine call directive: `call NAME(args) -> dest`. The call target
   * is parsed as a full expression so the function-call syntax
   * (`eaCalc(ModRM.mod, ModRM.rm)`) is handled by the existing
   * postfixExpr grammar. Argumentless calls like `call fetchModRm`
   * parse as `call <Identifier>` with no callOp.
   *
   * The optional `-> dest` clause captures the routine's return value
   * into a target. A target is either a single identifier (`-> ea`)
   * or a parenthesized tuple (`-> (a, b)`).
   */
  public callStmt = this.RULE('callStmt', () => {
    this.CONSUME(Call);
    this.SUBRULE(this.expression);
    this.OPTION(() => {
      this.CONSUME(Arrow);
      this.SUBRULE(this.callReturnTarget);
    });
  });

  public callReturnTarget = this.RULE('callReturnTarget', () => {
    this.OR([
      { ALT: () => this.CONSUME(Identifier) },
      {
        ALT: () => {
          this.CONSUME(LParen);
          this.OPTION(() => {
            this.CONSUME2(Identifier);
            this.MANY(() => {
              this.CONSUME(Comma);
              this.CONSUME3(Identifier);
            });
          });
          this.CONSUME(RParen);
        },
      },
    ]);
  });

  /**
   * Operand fetch directive: `fetch NAME`. Inlines the named operand's
   * fetch block at the inliner level. No arguments, no return binding.
   */
  public fetchStmt = this.RULE('fetchStmt', () => {
    this.CONSUME(Fetch);
    this.CONSUME(Identifier);
  });

  /**
   * Mux (switch-like) statement:
   *
   *   mux EXPR
   *     when PATTERN ...
   *     when PATTERN ...
   *     else ...
   *
   * Each arm's body is either an inline `: statement` form or a block
   * form with indented statements. Arms can appear in any order; the
   * `else` arm is optional (and typically last).
   */
  public muxStmt = this.RULE('muxStmt', () => {
    this.CONSUME(Mux);
    this.SUBRULE(this.expression);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.CONSUME(Indent);
    // Inside the mux body, accept newlines (blank or end-of-arm
    // terminators) interleaved with `when` / `else` arms. Block arms
    // don't leave a trailing Newline at this level (their
    // statementBlock consumes up to Outdent), but inline arms like
    // `when 0: ()` do leave a Newline, and that Newline needs to be
    // absorbed here before the next arm can fire.
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME2(Newline) },
        { ALT: () => this.SUBRULE(this.muxArm) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  public muxArm = this.RULE('muxArm', () => {
    this.OR([
      { ALT: () => this.SUBRULE(this.whenArm) },
      { ALT: () => this.SUBRULE(this.elseArm) },
    ]);
  });

  public whenArm = this.RULE('whenArm', () => {
    this.CONSUME(When);
    this.SUBRULE(this.expression);
    this.SUBRULE(this.muxArmBody);
  });

  public elseArm = this.RULE('elseArm', () => {
    this.CONSUME(Else);
    this.SUBRULE(this.muxArmBody);
  });

  public muxArmBody = this.RULE('muxArmBody', () => {
    this.OR([
      {
        ALT: () => {
          this.CONSUME(Colon);
          this.SUBRULE(this.statement);
        },
      },
      {
        ALT: () => {
          this.AT_LEAST_ONE(() => this.CONSUME(Newline));
          this.SUBRULE(this.statementBlock);
        },
      },
    ]);
  });

  /**
   * Expression-or-assignment statement. Parses an expression first; if
   * followed by `=` (combinational drive) or `<-` (register next-state),
   * consumes the operator and a right-hand-side expression. Otherwise
   * the parsed expression stands alone as a bare expression statement
   * — the common case for microword construction like `AluMicro {
   * ... }` or `Retire {}`.
   *
   * Assignment forms parsed here include:
   *   `FLAGS <- applyFlags(FLAGS, r.flags)`
   *   `busRequest.valid = 1`
   *   `dest <- prefetch.byte`
   *   `out.empty = qCount == 0`
   *
   * The LHS is any postfix expression, which is loose enough to accept
   * whatever combination of identifier, member access, index, and cast
   * the author wants. Semantic analysis validates whether the LHS is a
   * legal assignment target.
   */
  public exprOrAssignStmt = this.RULE('exprOrAssignStmt', () => {
    this.SUBRULE(this.expression);
    this.OPTION(() => {
      this.OR([
        { ALT: () => this.CONSUME(Equal) },
        { ALT: () => this.CONSUME(BackArrow) },
      ]);
      this.SUBRULE2(this.expression);
    });
  });

  /**
   * Wire declaration: `wire NAME = expr` or `wire NAME:type = expr`.
   * Binds a local intermediate name inside a body. The optional type
   * annotation is a simple typeRef (an identifier) for now;
   * parameterized types like `u{W+1}` need typeRef to grow in a later
   * slice when unit/machine bodies land.
   */
  public wireDeclStmt = this.RULE('wireDeclStmt', () => {
    this.CONSUME(Wire);
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.CONSUME(Colon);
      this.SUBRULE(this.typeRef);
    });
    this.CONSUME(Equal);
    this.SUBRULE(this.expression);
  });

  /**
   * If / elif / else statement. The then branch is a required
   * statementBlock; zero or more `elif` branches follow, and an
   * optional `else` branch closes it. Each branch's body is its own
   * indented statementBlock, so indent tracking naturally scopes
   * nested ifs to their closest containing block.
   */
  public ifStmt = this.RULE('ifStmt', () => {
    this.CONSUME(If);
    this.SUBRULE(this.expression);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.SUBRULE(this.statementBlock);
    this.MANY(() => this.SUBRULE(this.elifBranch));
    this.OPTION(() => this.SUBRULE(this.elseBranch));
  });

  public elifBranch = this.RULE('elifBranch', () => {
    this.CONSUME(Elif);
    this.SUBRULE(this.expression);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.SUBRULE(this.statementBlock);
  });

  public elseBranch = this.RULE('elseBranch', () => {
    this.CONSUME(Else);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.SUBRULE(this.statementBlock);
  });

  /**
   * Assert statement: `assert EXPR`. Expected to hold at runtime; the
   * semantic pass and downstream generators decide how violations are
   * reported (build-time panic, runtime trap, etc.).
   */
  public assertStmt = this.RULE('assertStmt', () => {
    this.CONSUME(Assert);
    this.SUBRULE(this.expression);
  });

  /**
   * Anonymous output assignment: `= expr`. Used by units with a
   * single unnamed output (the `*:type` form) like `parity8` — the
   * bare `= ~(v[0] ^ ...)` line at the end of the unit body assigns
   * the expression to the unnamed output. At the grammar level it's
   * just a leading `=` followed by an expression; semantic analysis
   * binds it to the output declared in `wires out *:type`.
   */
  public anonAssignStmt = this.RULE('anonAssignStmt', () => {
    this.CONSUME(Equal);
    this.SUBRULE(this.expression);
  });

  /**
   * Named instance declaration:
   *
   *   instance NAME : unit                        ; header only, no pins
   *   instance NAME : unit { p: expr, p: expr }   ; inline pinned ports
   *   instance NAME : unit                        ; block pinned ports
   *     p = expr
   *     p = expr
   *
   * The `decl` after the colon is a `typeRef`, so turbofish
   * instantiations like `alu::<u8>` land naturally — `typeRef`
   * already accepts an identifier optionally followed by curly-brace
   * type parameters (`Local{8}`). Turbofish (`::<...>`) works
   * through a separate pathway that we'll grow later if needed;
   * for now the block pin form handles the common cases cleanly.
   *
   * The port binding list uses `name = expr` syntax in both forms
   * (inline and block). Inline entries are comma-separated; block
   * entries are newline-separated. An instance with no pinned ports
   * is legal — all ports must then be driven at statement level.
   *
   * See core/ALU_UNITS_MACHINES.md for the semantic model this
   * parses against.
   */
  public instanceStmt = this.RULE('instanceStmt', () => {
    this.CONSUME(Instance);
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.instanceTypeRef);
    this.OPTION(() => {
      this.OR([
        // Inline form: `{ p: expr, p: expr }`
        {
          ALT: () => {
            this.CONSUME(LBrace);
            this.OPTION1(() => {
              this.SUBRULE(this.instancePortBinding);
              this.MANY(() => {
                this.CONSUME(Comma);
                this.SUBRULE2(this.instancePortBinding);
              });
            });
            this.CONSUME(RBrace);
          },
        },
        // Block form: indented `p = expr` lines.
        {
          ALT: () => {
            this.AT_LEAST_ONE(() => this.CONSUME(Newline));
            this.OPTION2(() => this.SUBRULE(this.instancePortBody));
          },
        },
      ]);
    });
  });

  /**
   * The `: type` reference after an instance name. Handles both the
   * bare-unit form (`instance foo : myUnit`) and the turbofish
   * type-parameterized form (`instance foo : alu::<u8>`). The
   * turbofish path is a thin wrapper around the call-form's
   * turbofish rule but without the call arguments.
   */
  public instanceTypeRef = this.RULE('instanceTypeRef', () => {
    this.CONSUME(Identifier);
    this.OPTION(() => {
      this.CONSUME(DoubleColon);
      this.CONSUME(Less);
      this.SUBRULE(this.typeRef);
      this.MANY(() => {
        this.CONSUME(Comma);
        this.SUBRULE2(this.typeRef);
      });
      this.CONSUME(Greater);
    });
  });

  /**
   * A single port binding inside an instance's inline or block form.
   * Inline form uses `name: expr` (colon, reusing record-literal
   * shape) and block form uses `name = expr`. Both are represented
   * by the same rule with an OR on the separator — semantic analysis
   * doesn't care which form was used.
   */
  public instancePortBinding = this.RULE('instancePortBinding', () => {
    this.CONSUME(Identifier);
    this.OR([
      { ALT: () => this.CONSUME(Colon) },
      { ALT: () => this.CONSUME(Equal) },
    ]);
    this.SUBRULE(this.expression);
  });

  /**
   * Indented block of port bindings inside an instance's block form.
   * Each line is an `instancePortBinding` using `=` as the
   * separator. Blank and comment-only lines between bindings are
   * absorbed by the inner OR.
   */
  public instancePortBody = this.RULE('instancePortBody', () => {
    this.CONSUME(Indent);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Newline) },
        { ALT: () => this.SUBRULE(this.instancePortBinding) },
      ]);
    });
    this.CONSUME(Outdent);
  });

  // =========================================================================
  // Expression grammar
  //
  // Standard C-family precedence, lowest-binding at the top:
  //
  //   logicalOr  →  logicalAnd  →  bitOr  →  bitXor  →  bitAnd  →
  //   equality   →  relational  →  shift  →  additive  →  multiplicative →
  //   unary      →  postfix     →  primary
  //
  // Postfix operators (left-associative, freely composable):
  //
  //   memberOp   `.ident`
  //   callOp     `(args)`
  //   indexOp    `[expr]` or `[hi:lo]`
  //   castOp     `:Type`   — requires `TightColon` (no whitespace around
  //                          the `:`), which is how cast avoids colliding
  //                          with other uses of `:` in the grammar
  //
  // The ternary `cond ? a : b` lives above `logicalOrExpr` at the top of
  // the precedence ladder and is right-associative. The separator must
  // be a loose `Colon` (i.e. have whitespace around it); fully-tight
  // `a?b:c` is a known edge case — the cast rule greedily consumes the
  // inner `:` and the ternary fails to find its separator. Real code
  // always writes `a ? b : c` with spaces, and that parses cleanly.
  //
  // Type-parameterized calls use a Rust-style turbofish: `alu::<W>(...)`.
  // The `::` before the `<` disambiguates from `alu < W > (...)` which
  // would otherwise be a sequence of comparisons. Declarations
  // (`unit alu<W:Width>`) keep their bare-angle-bracket syntax because
  // they're in a distinct syntactic position with no ambiguity.
  //
  // `readyClause` is the main parser site that currently uses the
  // expression grammar. Operand and microword bodies with a `fetch` or
  // `effect` block still use the opaque `block` skipper.
  // =========================================================================

  public expression = this.RULE('expression', () => {
    this.SUBRULE(this.ternaryExpr);
  });

  /**
   * Ternary conditional expression: `cond ? a : b`. Right-associative, so
   * `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.
   *
   * The condition is parsed at `logicalOrExpr` (one level tighter), so
   * binary operators bind tighter than the ternary — `a || b ? c : d`
   * is `(a || b) ? c : d`, not `a || (b ? c : d)`.
   *
   * Both branches recurse back into `ternaryExpr` so they can themselves
   * be full expressions, including nested ternaries.
   */
  public ternaryExpr = this.RULE('ternaryExpr', () => {
    this.SUBRULE(this.logicalOrExpr);
    this.OPTION(() => {
      this.CONSUME(Question);
      this.SUBRULE2(this.ternaryExpr);
      this.CONSUME(Colon);
      this.SUBRULE3(this.ternaryExpr);
    });
  });

  public logicalOrExpr = this.RULE('logicalOrExpr', () => {
    this.SUBRULE(this.logicalAndExpr);
    this.MANY(() => {
      this.CONSUME(OrOr);
      this.SUBRULE2(this.logicalAndExpr);
    });
  });

  public logicalAndExpr = this.RULE('logicalAndExpr', () => {
    this.SUBRULE(this.bitOrExpr);
    this.MANY(() => {
      this.CONSUME(AndAnd);
      this.SUBRULE2(this.bitOrExpr);
    });
  });

  public bitOrExpr = this.RULE('bitOrExpr', () => {
    this.SUBRULE(this.bitXorExpr);
    this.MANY(() => {
      this.CONSUME(Pipe);
      this.SUBRULE2(this.bitXorExpr);
    });
  });

  public bitXorExpr = this.RULE('bitXorExpr', () => {
    this.SUBRULE(this.bitAndExpr);
    this.MANY(() => {
      this.CONSUME(Caret);
      this.SUBRULE2(this.bitAndExpr);
    });
  });

  public bitAndExpr = this.RULE('bitAndExpr', () => {
    this.SUBRULE(this.equalityExpr);
    this.MANY(() => {
      this.CONSUME(Amp);
      this.SUBRULE2(this.equalityExpr);
    });
  });

  public equalityExpr = this.RULE('equalityExpr', () => {
    this.SUBRULE(this.relationalExpr);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(EqualEqual) },
        { ALT: () => this.CONSUME(NotEqual) },
      ]);
      this.SUBRULE2(this.relationalExpr);
    });
  });

  public relationalExpr = this.RULE('relationalExpr', () => {
    this.SUBRULE(this.shiftExpr);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(LessEqual) },
        { ALT: () => this.CONSUME(GreaterEqual) },
        { ALT: () => this.CONSUME(Less) },
        { ALT: () => this.CONSUME(Greater) },
      ]);
      this.SUBRULE2(this.shiftExpr);
    });
  });

  public shiftExpr = this.RULE('shiftExpr', () => {
    this.SUBRULE(this.additiveExpr);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(ShiftLeft) },
        { ALT: () => this.CONSUME(ShiftRight) },
      ]);
      this.SUBRULE2(this.additiveExpr);
    });
  });

  public additiveExpr = this.RULE('additiveExpr', () => {
    this.SUBRULE(this.multiplicativeExpr);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Plus) },
        { ALT: () => this.CONSUME(Minus) },
      ]);
      this.SUBRULE2(this.multiplicativeExpr);
    });
  });

  public multiplicativeExpr = this.RULE('multiplicativeExpr', () => {
    this.SUBRULE(this.unaryExpr);
    this.MANY(() => {
      this.OR([
        { ALT: () => this.CONSUME(Star) },
        { ALT: () => this.CONSUME(Slash) },
      ]);
      this.SUBRULE2(this.unaryExpr);
    });
  });

  /**
   * Unary prefix operators: `!expr`, `~expr`, `-expr`. Any of these can
   * stack (`--x`, `!!x`) via the recursive call.
   */
  public unaryExpr = this.RULE('unaryExpr', () => {
    this.OR([
      {
        ALT: () => {
          this.OR2([
            { ALT: () => this.CONSUME(Bang) },
            { ALT: () => this.CONSUME(Tilde) },
            { ALT: () => this.CONSUME(Minus) },
          ]);
          this.SUBRULE(this.unaryExpr);
        },
      },
      { ALT: () => this.SUBRULE2(this.postfixExpr) },
    ]);
  });

  /**
   * Postfix operator chain: member access, function call, index/slice,
   * cast, and constructor (record literal after an identifier). All
   * left-associative and freely composable — `a.b[3]():u8` is four
   * postfix ops applied in order.
   *
   * Cast specifically consumes `TightColon`, not `Colon`, so a ternary
   * separator (which is always loose `Colon`) can't be mistaken for a
   * cast. See the `TightColon` token declaration for the full story.
   *
   * Function calls support an optional Rust-style turbofish type-arg
   * prefix: `f::<T>(...)`. The bare form `f<T>(...)` is not accepted —
   * the `::` disambiguates from a `<` comparison. See `callOp` below.
   *
   * The `constructOp` (`Name { field: value, ... }`) is how microword
   * construction like `AluMicro { op: add, width: u8 }` parses as a
   * single expression rather than an identifier followed by a bare
   * record literal on the same line.
   */
  public postfixExpr = this.RULE('postfixExpr', () => {
    this.SUBRULE(this.primaryExpr);
    this.MANY({
      // MANY-level GATE decides whether to continue looping. Putting
      // the cast check here (rather than on the OR alternative) means
      // that when LA(1)=TightColon but LA(2) is not an identifier
      // (e.g., inside a bit slice `[7:0]`), MANY exits cleanly instead
      // of trying to enter the cast alt and raising NoViableAlt. The
      // unconsumed TightColon then falls through to indexOp's slice
      // branch, which consumes it via the Colon category.
      GATE: () => {
        const la1 = this.LA(1);
        if (tokenMatcher(la1, Dot)) return true;
        if (tokenMatcher(la1, LParen)) return true;
        if (tokenMatcher(la1, DoubleColon)) return true;
        if (tokenMatcher(la1, LBracket)) return true;
        if (tokenMatcher(la1, LBrace)) return true;
        if (tokenMatcher(la1, TightColon)) {
          return tokenMatcher(this.LA(2), Identifier);
        }
        return false;
      },
      DEF: () => {
        this.OR([
          { ALT: () => this.SUBRULE2(this.memberOp) },
          { ALT: () => this.SUBRULE3(this.callOp) },
          { ALT: () => this.SUBRULE4(this.indexOp) },
          { ALT: () => this.SUBRULE5(this.castOp) },
          { ALT: () => this.SUBRULE6(this.constructOp) },
        ]);
      },
    });
  });

  public memberOp = this.RULE('memberOp', () => {
    this.CONSUME(Dot);
    // After a `.`, the member name is an identifier (usually) or one
    // of a handful of contextual keywords that are only reserved at
    // specific grammar positions. `fetch` is the only real-world
    // collision today — `ExecuteState.fetch` references the `fetch`
    // variant of the ExecuteState enum, where `fetch` is the same
    // word as the routine-level `fetch` directive. We resolve the
    // collision here by accepting either token as a member name.
    // Extend this OR when new collisions surface.
    this.OR([
      { ALT: () => this.CONSUME(Identifier) },
      { ALT: () => this.CONSUME(Fetch) },
    ]);
  });

  /**
   * Function call: `name(arg, arg, ...)`, with an optional Rust-style
   * turbofish type-argument prefix: `name::<T>(arg)` or
   * `name::<T, U>(arg)`. The `::` disambiguates type arguments from
   * `<` comparison — `name<T>(x)` without the `::` parses as
   * `(name < T)` followed by `> (x)`, not as a parameterized call.
   *
   * Declarations (`unit alu<W:Width>`) don't need the turbofish because
   * the `<` there is unambiguously part of the declaration syntax.
   */
  public callOp = this.RULE('callOp', () => {
    this.OPTION(() => {
      this.CONSUME(DoubleColon);
      this.CONSUME(Less);
      this.SUBRULE(this.typeRef);
      this.MANY(() => {
        this.CONSUME(Comma);
        this.SUBRULE2(this.typeRef);
      });
      this.CONSUME(Greater);
    });
    this.CONSUME(LParen);
    this.OPTION2(() => {
      this.SUBRULE3(this.expression);
      this.MANY2(() => {
        this.CONSUME2(Comma);
        this.SUBRULE4(this.expression);
      });
    });
    this.CONSUME(RParen);
  });

  /**
   * Index / bit-slice: `expr[i]` for a single bit or `expr[hi:lo]` for a
   * range. Both the lo and hi positions are themselves expressions, so
   * `raw[W+1]` and `disp[7:0]` both parse naturally.
   */
  public indexOp = this.RULE('indexOp', () => {
    this.CONSUME(LBracket);
    this.SUBRULE(this.expression);
    this.OPTION(() => {
      this.CONSUME(Colon);
      this.SUBRULE2(this.expression);
    });
    this.CONSUME(RBracket);
  });

  /**
   * Cast: `expr:Type`. The `TightColon` token only matches a colon with
   * no whitespace on either side, so loose `:` (record fields, ternary
   * separators, declaration annotations with spaces) don't collide. The
   * type reference is a plain identifier for now; parameterized types
   * like `Local<8>` come with the type-parameterized-call story.
   */
  public castOp = this.RULE('castOp', () => {
    this.CONSUME(TightColon);
    this.SUBRULE(this.typeRef);
  });

  /**
   * Constructor / record-literal postfix: `Name { field: value, ... }`.
   * Structurally identical to the primary-form `recordLiteral` but
   * attached as a postfix op so `Name { ... }` parses as one
   * expression rather than `Name` followed by a bare `{ ... }`.
   */
  public constructOp = this.RULE('constructOp', () => {
    this.CONSUME(LBrace);
    this.OPTION(() => {
      this.SUBRULE(this.recordField);
      this.MANY(() => {
        this.CONSUME(Comma);
        this.SUBRULE2(this.recordField);
      });
    });
    this.CONSUME(RBrace);
  });

  /**
   * Primary expressions — the atoms the rest of the precedence ladder
   * builds on. Numeric and string literals, identifiers, parenthesized
   * sub-expressions, record literals, and mux expressions.
   *
   * `muxStmt` appears here so it can be used in both statement
   * contexts (`mux x when 0 ...`) and expression contexts (`wire raw
   * = mux op when ...`). The arm bodies accept either inline
   * expressions or indented statement blocks; semantic analysis
   * decides what's legal in each calling context.
   */
  public primaryExpr = this.RULE('primaryExpr', () => {
    this.OR([
      { ALT: () => this.CONSUME(DecimalLiteral) },
      { ALT: () => this.CONSUME(HexLiteral) },
      { ALT: () => this.CONSUME(StringLiteral) },
      { ALT: () => this.CONSUME(Identifier) },
      { ALT: () => this.SUBRULE(this.parenExpr) },
      { ALT: () => this.SUBRULE(this.recordLiteral) },
      { ALT: () => this.SUBRULE(this.muxStmt) },
    ]);
  });

  /**
   * Parenthesized expression, possibly empty. Empty parens `()` parse
   * as a "unit value" — semantically a no-op in contexts like
   * `when 0: ()` where an empty statement is needed. Semantic
   * analysis can restrict empty parens to those specific positions.
   */
  public parenExpr = this.RULE('parenExpr', () => {
    this.CONSUME(LParen);
    this.OPTION(() => this.SUBRULE(this.expression));
    this.CONSUME(RParen);
  });

  /**
   * Record literal: `{ field: value, field: value, ... }`. Possibly
   * empty. Fields are name/expression pairs in any order. Used in
   * microword construction (`AluMicro { op: add, width: u8, ... }`) and
   * anywhere a structured value is needed.
   */
  public recordLiteral = this.RULE('recordLiteral', () => {
    this.CONSUME(LBrace);
    this.OPTION(() => {
      this.SUBRULE(this.recordField);
      this.MANY(() => {
        this.CONSUME(Comma);
        this.SUBRULE2(this.recordField);
      });
    });
    this.CONSUME(RBrace);
  });

  public recordField = this.RULE('recordField', () => {
    this.CONSUME(Identifier);
    this.CONSUME(Colon);
    this.SUBRULE(this.expression);
  });

  // =========================================================================
  // Opaque token skippers (used by declarations still on the skeleton)
  // =========================================================================

  /**
   * Consume any single token that can appear inside a block body, except
   * the structural Indent / Outdent markers. Newlines inside blocks are
   * part of body content at this layer — once real body grammars arrive,
   * they'll interpret newlines as statement terminators.
   */
  public anyBodyToken = this.RULE('anyBodyToken', () => {
    this.OR([
      // Identifiers and literals
      { ALT: () => this.CONSUME(Identifier) },
      { ALT: () => this.CONSUME(HexLiteral) },
      { ALT: () => this.CONSUME(DecimalLiteral) },
      { ALT: () => this.CONSUME(StringLiteral) },
      // Keywords (can appear inside bodies as nested statements,
      // section markers, mux/when/if control flow, call/fetch directives,
      // and as field names that aren't yet reserved context-free).
      { ALT: () => this.CONSUME(Unit) },
      { ALT: () => this.CONSUME(Machine) },
      { ALT: () => this.CONSUME(Instance) },
      { ALT: () => this.CONSUME(Register) },
      { ALT: () => this.CONSUME(Enum) },
      { ALT: () => this.CONSUME(Bundle) },
      { ALT: () => this.CONSUME(Union) },
      { ALT: () => this.CONSUME(Microword) },
      { ALT: () => this.CONSUME(Operand) },
      { ALT: () => this.CONSUME(Routine) },
      { ALT: () => this.CONSUME(Call) },
      { ALT: () => this.CONSUME(Fetch) },
      { ALT: () => this.CONSUME(Field) },
      { ALT: () => this.CONSUME(Wire) },
      { ALT: () => this.CONSUME(Wires) },
      { ALT: () => this.CONSUME(Registers) },
      { ALT: () => this.CONSUME(Default) },
      // Soft-keyword section markers (`In`, `Out`, `Id`, `Fields`,
      // `Ready`, `Effect`, `Entry`, `Allow`, `Modifies`, `References`,
      // `Micro`) are matched by the `Identifier` alternative above via
      // the category mechanism; listing them here explicitly would
      // cause ambiguous-alternative errors in Chevrotain's
      // self-analysis.
      { ALT: () => this.CONSUME(Description) },
      { ALT: () => this.CONSUME(Mux) },
      { ALT: () => this.CONSUME(When) },
      { ALT: () => this.CONSUME(If) },
      { ALT: () => this.CONSUME(Elif) },
      { ALT: () => this.CONSUME(Else) },
      { ALT: () => this.CONSUME(Assert) },
      // Multi-char operators
      { ALT: () => this.CONSUME(Arrow) },
      { ALT: () => this.CONSUME(BackArrow) },
      { ALT: () => this.CONSUME(EqualEqual) },
      { ALT: () => this.CONSUME(NotEqual) },
      { ALT: () => this.CONSUME(LessEqual) },
      { ALT: () => this.CONSUME(GreaterEqual) },
      { ALT: () => this.CONSUME(AndAnd) },
      { ALT: () => this.CONSUME(OrOr) },
      { ALT: () => this.CONSUME(ShiftLeft) },
      { ALT: () => this.CONSUME(ShiftRight) },
      { ALT: () => this.CONSUME(DotDot) },
      // Single-char punctuation and operators
      { ALT: () => this.CONSUME(LBrace) },
      { ALT: () => this.CONSUME(RBrace) },
      { ALT: () => this.CONSUME(LParen) },
      { ALT: () => this.CONSUME(RParen) },
      { ALT: () => this.CONSUME(LBracket) },
      { ALT: () => this.CONSUME(RBracket) },
      { ALT: () => this.CONSUME(Colon) },
      { ALT: () => this.CONSUME(Comma) },
      { ALT: () => this.CONSUME(Dot) },
      { ALT: () => this.CONSUME(At) },
      { ALT: () => this.CONSUME(Equal) },
      { ALT: () => this.CONSUME(Less) },
      { ALT: () => this.CONSUME(Greater) },
      { ALT: () => this.CONSUME(Plus) },
      { ALT: () => this.CONSUME(Minus) },
      { ALT: () => this.CONSUME(Star) },
      { ALT: () => this.CONSUME(Slash) },
      { ALT: () => this.CONSUME(Pipe) },
      { ALT: () => this.CONSUME(Amp) },
      { ALT: () => this.CONSUME(Caret) },
      { ALT: () => this.CONSUME(Tilde) },
      { ALT: () => this.CONSUME(Bang) },
      { ALT: () => this.CONSUME(Question) },
      // Newlines are part of body content (statement separators).
      { ALT: () => this.CONSUME(Newline) },
    ]);
  });

}

/**
 * Shared parser instance. Chevrotain parsers are stateful but can be
 * re-used across parses by assigning a new `input` before calling the
 * entry rule.
 */
export const machineParser = new MachineParser();

// MachineParser — the Chevrotain CstParser for .machine files.
//
// First-slice grammar: recognize every top-level declaration, capture its
// name and (for unit/routine) signature, and skip the body. Body grammars
// land incrementally in follow-up passes — the block rule just walks
// INDENT...OUTDENT pairs and consumes whatever's inside.

import { CstParser } from 'chevrotain';
import {
  allTokens,
  // Keywords
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
  Field,
  Description,
  Fields,
  Ready,
  Effect,
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
  // Single-char operators and punctuation
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
  // then optionally consumes a Newline followed by an indented block. The
  // block's contents are consumed opaquely by `block` / `anyBodyToken` —
  // real body grammars will replace this in later slices.
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
   * Does not yet handle the bit-range form `field[HI:LO] NAME:TYPE` used
   * by registers declared inside `machine` bodies (e.g. executionUnit's
   * `modrm` sub-register); that lives in machine-body grammar, which is
   * still covered by the opaque `block` fallback.
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
    this.CONSUME(Identifier);
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
    this.OPTION2(() => this.SUBRULE(this.headerTerminator));
  });

  public machineDecl = this.RULE('machineDecl', () => {
    this.CONSUME(Machine);
    this.CONSUME(Identifier);
    this.OPTION(() => this.SUBRULE(this.headerTerminator));
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
   *   ready             — single-line readiness expression (opaque)
   *   effect            — indented block of statements (opaque)
   *
   * Only `fields` is interpreted by the walker today. The expression and
   * statement grammars for `ready` and `effect` arrive in a later slice.
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
   * A `ready: <expression>` clause. The expression is an arbitrary
   * single-line sub-language (`1`, `prefetch.valid`, `busResponse.done`,
   * boolean combinations, etc.); we don't yet have an expression grammar
   * so we consume tokens opaquely up to the terminating Newline.
   */
  public readyClause = this.RULE('readyClause', () => {
    this.CONSUME(Ready);
    this.CONSUME(Colon);
    this.MANY(() => this.SUBRULE(this.anyInlineToken));
    this.CONSUME(Newline);
  });

  /**
   * An `effect` section: keyword followed by an indented block of
   * statements. The statement grammar lives in a later slice; today the
   * block is consumed opaquely via the general `block` skipper.
   */
  public effectSection = this.RULE('effectSection', () => {
    this.CONSUME(Effect);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.block));
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
   *   description    — prose (inline or block), opaque
   *   size: N        — a single numeric literal (byte count)
   *   fields         — typed fields with optional @ offset (operand slots)
   *   fetch          — block of microword record literals, opaque
   *
   * `description` and `fields` are reused directly from the microword
   * body grammar — they share the same rule.
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
   * pull the operand's bytes out of the instruction stream. The inner
   * block is consumed opaquely via the general `block` skipper; a later
   * slice will replace this with real statement grammar once expressions
   * and record literals are in place.
   */
  public fetchSection = this.RULE('fetchSection', () => {
    this.CONSUME(Fetch);
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.block));
  });

  public routineDecl = this.RULE('routineDecl', () => {
    this.CONSUME(Routine);
    this.CONSUME(Identifier);
    this.OPTION1(() => this.SUBRULE(this.paramList));
    this.OPTION2(() => this.SUBRULE(this.returnType));
    this.OPTION3(() => this.SUBRULE(this.headerTerminator));
  });

  // =========================================================================
  // Shared pieces
  // =========================================================================

  /**
   * After a declaration header, the source can end in three ways:
   *   1. Newline(+) + INDENT block  → declaration with a body
   *   2. Newline(+) only            → one-liner, next declaration follows
   *   3. EOF                        → last declaration has no trailing newline
   *
   * This rule handles cases 1 and 2 — the OPTION wrapper on each callsite
   * handles case 3 by not entering this rule at all.
   *
   * The `AT_LEAST_ONE` on Newlines covers the case where blank lines or
   * comment-only lines sit between the header and the body: each such
   * line produces a Newline token in the stream (the lexer strips the
   * comment body and horizontal whitespace), so we may see several in a
   * row before the block's INDENT arrives.
   */
  public headerTerminator = this.RULE('headerTerminator', () => {
    this.AT_LEAST_ONE(() => this.CONSUME(Newline));
    this.OPTION(() => this.SUBRULE(this.block));
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
   * First-slice type reference: just an identifier. No parameterization
   * (`Local<8>`), no interpolation (`u{W+1}`), no slice types yet. These
   * only appear in declaration signatures right now, which for ALU_NEW
   * are always simple names (`u8`, `u16`, `Width`, `AluOp`, etc.).
   */
  public typeRef = this.RULE('typeRef', () => {
    this.CONSUME(Identifier);
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
      { ALT: () => this.CONSUME(Description) },
      { ALT: () => this.CONSUME(Fields) },
      { ALT: () => this.CONSUME(Ready) },
      { ALT: () => this.CONSUME(Effect) },
      { ALT: () => this.CONSUME(Mux) },
      { ALT: () => this.CONSUME(When) },
      { ALT: () => this.CONSUME(If) },
      { ALT: () => this.CONSUME(Elif) },
      { ALT: () => this.CONSUME(Else) },
      { ALT: () => this.CONSUME(Assert) },
      // Multi-char operators
      { ALT: () => this.CONSUME(Arrow) },
      { ALT: () => this.CONSUME(BackArrow) },
      { ALT: () => this.CONSUME(ColonEqual) },
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

  /**
   * Same set of alternatives as `anyBodyToken` except Newline is
   * excluded. Used by `readyClause` to consume the single-line opaque
   * expression up to (but not including) the terminating Newline. When
   * expression grammar lands, `readyClause` swaps `anyInlineToken` for
   * a proper expression rule and this helper can be deleted.
   */
  public anyInlineToken = this.RULE('anyInlineToken', () => {
    this.OR([
      { ALT: () => this.CONSUME(Identifier) },
      { ALT: () => this.CONSUME(HexLiteral) },
      { ALT: () => this.CONSUME(DecimalLiteral) },
      { ALT: () => this.CONSUME(StringLiteral) },
      { ALT: () => this.CONSUME(Unit) },
      { ALT: () => this.CONSUME(Machine) },
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
      { ALT: () => this.CONSUME(Description) },
      { ALT: () => this.CONSUME(Fields) },
      { ALT: () => this.CONSUME(Ready) },
      { ALT: () => this.CONSUME(Effect) },
      { ALT: () => this.CONSUME(Mux) },
      { ALT: () => this.CONSUME(When) },
      { ALT: () => this.CONSUME(If) },
      { ALT: () => this.CONSUME(Elif) },
      { ALT: () => this.CONSUME(Else) },
      { ALT: () => this.CONSUME(Assert) },
      { ALT: () => this.CONSUME(Arrow) },
      { ALT: () => this.CONSUME(BackArrow) },
      { ALT: () => this.CONSUME(ColonEqual) },
      { ALT: () => this.CONSUME(EqualEqual) },
      { ALT: () => this.CONSUME(NotEqual) },
      { ALT: () => this.CONSUME(LessEqual) },
      { ALT: () => this.CONSUME(GreaterEqual) },
      { ALT: () => this.CONSUME(AndAnd) },
      { ALT: () => this.CONSUME(OrOr) },
      { ALT: () => this.CONSUME(ShiftLeft) },
      { ALT: () => this.CONSUME(ShiftRight) },
      { ALT: () => this.CONSUME(DotDot) },
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
    ]);
  });
}

/**
 * Shared parser instance. Chevrotain parsers are stateful but can be
 * re-used across parses by assigning a new `input` before calling the
 * entry rule.
 */
export const machineParser = new MachineParser();

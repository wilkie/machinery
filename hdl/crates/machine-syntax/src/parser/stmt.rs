//! Statement syntax, for every body kind (`rtl`, `model`, unit, routine,
//! `isa` semantics, tests). Which statements are legal where is checked
//! semantically, not here.

use super::{PResult, Parser};
use crate::ast::*;
use crate::span::Span;
use crate::token::{Keyword, Punct, TokenKind};

/// Words that open a `name { ... }` block inside tests and conformance
/// declarations.
const NAMED_BLOCK_WORDS: &[&str] = &["given", "drive", "expect", "clocks"];

impl Parser {
    /// `{ stmt* }`, with per-statement recovery.
    pub(crate) fn parse_block(&mut self) -> PResult<Block> {
        let start = self.expect_punct(Punct::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            if self.at_punct(Punct::RBrace) {
                let end = self.advance().span;
                return Ok(Block {
                    stmts,
                    span: start.to(end),
                });
            }
            if self.at_eof() {
                return Err(self.error_expected("`}`"));
            }
            let before = self.pos;
            let stmt_start = self.cur_span();
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(_) => {
                    self.recover_in_block();
                    stmts.push(Stmt {
                        kind: StmtKind::Error,
                        span: stmt_start.to(self.prev_span()),
                    });
                    if self.pos == before && !self.at_punct(Punct::RBrace) {
                        self.advance();
                    }
                }
            }
        }
    }

    fn mk(&self, kind: StmtKind, start: Span) -> Stmt {
        Stmt {
            kind,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_stmt(&mut self) -> PResult<Stmt> {
        let docs = self.take_docs();
        let start = self.cur_span();
        match self.peek_kind().clone() {
            TokenKind::Keyword(kw) => match kw {
                Keyword::Let => self.parse_let(start),
                Keyword::If => {
                    let s = self.parse_if_stmt()?;
                    Ok(s)
                }
                Keyword::Match => {
                    self.advance();
                    let scrutinee = self.parse_expr_no_struct()?;
                    let arms = self.parse_match_arms()?;
                    Ok(self.mk(StmtKind::Match { scrutinee, arms }, start))
                }
                Keyword::For => {
                    self.advance();
                    let var = self.expect_ident()?;
                    self.expect_kw(Keyword::In)?;
                    let range = self.parse_expr_no_struct()?;
                    let body = self.parse_block()?;
                    Ok(self.mk(StmtKind::For { var, range, body }, start))
                }
                Keyword::Loop => {
                    self.advance();
                    let body = self.parse_block()?;
                    Ok(self.mk(StmtKind::Loop(body), start))
                }
                Keyword::While => {
                    self.advance();
                    let cond = self.parse_expr_no_struct()?;
                    let body = self.parse_block()?;
                    Ok(self.mk(StmtKind::While { cond, body }, start))
                }
                Keyword::Break => {
                    self.advance();
                    self.expect_semi()?;
                    Ok(self.mk(StmtKind::Break, start))
                }
                Keyword::Continue => {
                    self.advance();
                    self.expect_semi()?;
                    Ok(self.mk(StmtKind::Continue, start))
                }
                Keyword::Return => {
                    self.advance();
                    let value = if self.at_punct(Punct::Semi) {
                        None
                    } else {
                        Some(self.parse_expr()?)
                    };
                    self.expect_semi()?;
                    Ok(self.mk(StmtKind::Return(value), start))
                }
                _ => self.parse_expr_stmt(start),
            },
            TokenKind::Ident(word) => self.parse_word_stmt(&word, docs, start),
            _ => self.parse_expr_stmt(start),
        }
    }

    fn parse_let(&mut self, start: Span) -> PResult<Stmt> {
        self.expect_kw(Keyword::Let)?;
        let pattern = self.parse_let_pattern()?;
        let ty = if self.eat_punct(Punct::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_punct(Punct::Eq)?;
        let value = self.parse_expr()?;
        self.expect_semi()?;
        Ok(self.mk(StmtKind::Let { pattern, ty, value }, start))
    }

    fn parse_let_pattern(&mut self) -> PResult<LetPattern> {
        if self.at_punct(Punct::Underscore) {
            let s = self.advance().span;
            return Ok(LetPattern::Wildcard(s));
        }
        if self.at_punct(Punct::LParen) {
            let items =
                self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
                    p.parse_let_pattern()
                })?;
            return Ok(LetPattern::Tuple(items));
        }
        Ok(LetPattern::Name(self.expect_ident()?))
    }

    fn parse_if_stmt(&mut self) -> PResult<Stmt> {
        let start = self.expect_kw(Keyword::If)?;
        let cond = self.parse_expr_no_struct()?;
        let then_branch = self.parse_block()?;
        // An `else { }` arm is represented as a block-expression statement;
        // `else if` nests another `If` statement.
        let else_branch = if self.eat_kw(Keyword::Else) {
            if self.at_kw(Keyword::If) {
                Some(Box::new(self.parse_if_stmt()?))
            } else {
                let b = self.parse_block()?;
                let span = b.span;
                Some(Box::new(Stmt {
                    kind: StmtKind::Expr(Expr {
                        kind: ExprKind::Block(b),
                        span,
                    }),
                    span,
                }))
            }
        } else {
            None
        };
        Ok(self.mk(
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            },
            start,
        ))
    }

    /// `expr;`, `lhs = rhs;`, `lhs <- rhs @ann;`. A block-ending expression
    /// (`match`, `if`, or the tail of a block) needs no `;`.
    fn parse_expr_stmt(&mut self, start: Span) -> PResult<Stmt> {
        let e = self.parse_expr()?;
        if self.eat_punct(Punct::Eq) {
            let value = self.parse_expr()?;
            self.expect_semi()?;
            return Ok(self.mk(StmtKind::Assign { target: e, value }, start));
        }
        if self.eat_punct(Punct::LArrow) {
            let value = self.parse_expr()?;
            let mut annotations = Vec::new();
            while self.at_punct(Punct::At) {
                let a_start = self.advance().span;
                let name = self.expect_ident()?;
                let args = if self.at_punct(Punct::LParen) {
                    self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
                        p.parse_expr()
                    })?
                } else {
                    Vec::new()
                };
                annotations.push(Annotation {
                    name,
                    args,
                    span: a_start.to(self.prev_span()),
                });
            }
            self.expect_semi()?;
            return Ok(self.mk(
                StmtKind::Schedule {
                    target: e,
                    value,
                    annotations,
                },
                start,
            ));
        }
        let block_like = matches!(
            e.kind,
            ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Block(_)
        );
        if self.eat_punct(Punct::Semi) || block_like || self.at_punct(Punct::RBrace) {
            return Ok(self.mk(StmtKind::Expr(e), start));
        }
        Err(self.error_expected("`;`"))
    }

    /// Statements introduced by a contextual word. Falls back to an
    /// expression statement when the word is just an identifier.
    fn parse_word_stmt(&mut self, word: &str, docs: Docs, start: Span) -> PResult<Stmt> {
        let next_is_ident = self.nth_is_ident(1);
        let next_is_lbrace = self.nth_is_punct(1, Punct::LBrace);
        let decl_like = next_is_ident && self.nth_is_punct(2, Punct::Colon);
        match word {
            "default" if next_is_lbrace => {
                self.advance();
                let body = self.parse_block()?;
                Ok(self.mk(StmtKind::Default(body), start))
            }
            "cycle" if next_is_lbrace => {
                self.advance();
                let body = self.parse_block()?;
                Ok(self.mk(StmtKind::Cycle(body), start))
            }
            "on" if next_is_ident => {
                self.advance();
                let target = self.parse_path_dotted()?;
                let clock = self.parse_clock_ref()?;
                let params = if self.at_punct(Punct::LParen) {
                    Some(self.parse_params()?)
                } else {
                    None
                };
                let ret = if self.eat_punct(Punct::RArrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                let body = self.parse_block()?;
                Ok(self.mk(
                    StmtKind::On {
                        target,
                        clock,
                        params,
                        ret,
                        body,
                    },
                    start,
                ))
            }
            "wait" if self.nth_is_word(1, "until") => {
                self.advance();
                self.advance();
                let cond = self.parse_expr()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::WaitUntil(cond), start))
            }
            "halt" if self.nth_is_punct(1, Punct::Semi) => {
                self.advance();
                self.advance();
                Ok(self.mk(StmtKind::Halt, start))
            }
            "retire" if self.nth_is_punct(1, Punct::Semi) => {
                self.advance();
                self.advance();
                Ok(self.mk(StmtKind::Retire, start))
            }
            "fault" if next_is_ident => {
                self.advance();
                let e = self.parse_postfix()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Fault(e), start))
            }
            "raise" if next_is_ident => {
                self.advance();
                let e = self.parse_postfix()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Raise(e), start))
            }
            "start" if next_is_ident => {
                self.advance();
                let e = self.parse_postfix()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Start(e), start))
            }
            "dispatch" => {
                self.advance();
                let scrutinee = self.parse_expr_no_struct()?;
                self.expect_word("via")?;
                let table = self.parse_path_colons()?;
                let mut otherwise = None;
                if self.at_punct(Punct::LBrace) {
                    self.expect_punct(Punct::LBrace)?;
                    while !self.at_punct(Punct::RBrace) {
                        if self.at_eof() {
                            return Err(self.error_expected("`}`"));
                        }
                        self.expect_word("otherwise")?;
                        self.expect_punct(Punct::FatArrow)?;
                        otherwise = Some(self.parse_expr()?);
                        self.expect_semi()?;
                    }
                    self.expect_punct(Punct::RBrace)?;
                } else {
                    self.expect_semi()?;
                }
                Ok(self.mk(
                    StmtKind::Dispatch {
                        scrutinee,
                        table,
                        otherwise,
                    },
                    start,
                ))
            }
            "emit" if next_is_ident => {
                self.advance();
                let target = self.parse_postfix()?;
                let value = if self.eat_punct(Punct::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Emit { target, value }, start))
            }
            "reg" if decl_like => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                let clock = self.parse_clock_ref()?;
                let init = if self.eat_punct(Punct::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_semi()?;
                Ok(self.mk(
                    StmtKind::Reg {
                        docs,
                        name,
                        ty,
                        clock,
                        init,
                    },
                    start,
                ))
            }
            "wire" if decl_like => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                let init = if self.eat_punct(Punct::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Wire { name, ty, init }, start))
            }
            "state" if decl_like => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                let init = if self.eat_punct(Punct::Eq) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_semi()?;
                Ok(self.mk(StmtKind::State { name, ty, init }, start))
            }
            "mem" if decl_like => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let ty = self.parse_type()?;
                let mut options = Vec::new();
                if self.at_punct(Punct::LBrace) {
                    options = self.parse_delimited(
                        Punct::LBrace,
                        Punct::RBrace,
                        &[Punct::Comma, Punct::Semi],
                        |p| {
                            let key = p.expect_ident()?;
                            p.expect_punct(Punct::Colon)?;
                            Ok(match key.name.as_str() {
                                "ports" => MemOption::Ports(p.parse_expr()?),
                                "read" => MemOption::Read(p.expect_ident()?),
                                "write" => MemOption::Write(p.expect_ident()?),
                                "init" => MemOption::Init(p.parse_expr()?),
                                _ => {
                                    let v = if p.at_punct(Punct::Comma) || p.at_punct(Punct::RBrace)
                                    {
                                        None
                                    } else {
                                        Some(p.parse_expr()?)
                                    };
                                    MemOption::Other(key, v)
                                }
                            })
                        },
                    )?;
                }
                self.eat_punct(Punct::Semi);
                Ok(self.mk(
                    StmtKind::Mem {
                        docs,
                        name,
                        ty,
                        options,
                    },
                    start,
                ))
            }
            "instance" if decl_like => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect_punct(Punct::Colon)?;
                let component = self.parse_type()?;
                let clock = self.parse_clock_ref()?;
                let connections = if self.at_punct(Punct::LBrace) {
                    self.parse_field_inits(true)?.0
                } else {
                    Vec::new()
                };
                self.eat_punct(Punct::Semi);
                Ok(self.mk(
                    StmtKind::Instance {
                        name,
                        component,
                        clock,
                        connections,
                    },
                    start,
                ))
            }
            "routine" if next_is_ident => {
                let r = self.parse_routine(docs, false)?;
                Ok(self.mk(StmtKind::Routine(r), start))
            }
            "istream" if self.nth_is_word(1, "routine") => {
                self.advance();
                let r = self.parse_routine(docs, true)?;
                Ok(self.mk(StmtKind::Routine(r), start))
            }
            "sequencer" if next_is_ident => {
                self.advance();
                let name = self.expect_ident()?;
                let clock = self.parse_clock_ref()?;
                let body = self.parse_block()?;
                let span = start.to(self.prev_span());
                Ok(self.mk(
                    StmtKind::Sequencer(SequencerDecl {
                        docs,
                        name,
                        clock,
                        body,
                        span,
                    }),
                    start,
                ))
            }
            "unit" if next_is_ident => {
                let u = self.parse_unit(docs)?;
                Ok(self.mk(StmtKind::Unit(u), start))
            }
            "macro" if next_is_ident => {
                self.advance();
                let name = self.expect_ident()?;
                let params = self.parse_params()?;
                let body = self.parse_block()?;
                Ok(self.mk(
                    StmtKind::Macro {
                        docs,
                        name,
                        params,
                        body,
                    },
                    start,
                ))
            }
            "architectural" if next_is_lbrace => {
                self.advance();
                let block = self.parse_block()?;
                let mut mappings = Vec::new();
                let mut at = None;
                for s in block.stmts {
                    match s.kind {
                        StmtKind::Assign { target, value } => mappings.push((target, value)),
                        StmtKind::At(id) => at = Some(id),
                        StmtKind::Error => {}
                        _ => {
                            self.error(
                                "E0102",
                                s.span,
                                "only `ISA_NAME = rtl_expr;` mappings and `at <event>;` are allowed in `architectural`",
                            );
                        }
                    }
                }
                Ok(self.mk(StmtKind::Architectural { mappings, at }, start))
            }
            "at" if next_is_ident && self.nth_is_punct(2, Punct::Semi) => {
                self.advance();
                let ev = self.expect_ident()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::At(ev), start))
            }
            w if NAMED_BLOCK_WORDS.contains(&w) && next_is_lbrace => {
                let name = self.expect_ident()?;
                let body = self.parse_block()?;
                Ok(self.mk(StmtKind::NamedBlock { name, body }, start))
            }
            "run" => {
                self.advance();
                let spec = if self.at_word("instruction") {
                    self.advance();
                    let name = self.parse_path_colons()?;
                    let operands = if self.at_punct(Punct::LBrace) {
                        self.parse_field_inits(false)?.0
                    } else {
                        Vec::new()
                    };
                    self.expect_semi()?;
                    RunSpec::Instruction { name, operands }
                } else if self.at_word("bytes") {
                    self.advance();
                    let e = self.parse_expr()?;
                    self.expect_semi()?;
                    RunSpec::Bytes(e)
                } else if self.at_punct(Punct::LBrace) {
                    RunSpec::Block(self.parse_block()?)
                } else {
                    let count = self.parse_expr()?;
                    self.expect_word("cycles")?;
                    let clock = if self.eat_word("of") {
                        Some(self.expect_ident()?)
                    } else {
                        None
                    };
                    self.expect_semi()?;
                    RunSpec::Cycles { count, clock }
                };
                Ok(self.mk(StmtKind::Run(spec), start))
            }
            "stimulus" if next_is_ident => {
                self.advance();
                let e = if self.at_word("vectors")
                    && matches!(self.peek_nth(1).kind, TokenKind::Str(_))
                {
                    let callee = self.expect_ident()?;
                    let s = self.expect_str()?;
                    let span = callee.span.to(s.span);
                    Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(Expr {
                                span: callee.span,
                                kind: ExprKind::Path(Path::single(callee)),
                            }),
                            generics: Vec::new(),
                            args: vec![Arg {
                                name: None,
                                span: s.span,
                                value: Expr {
                                    kind: ExprKind::Str(s.value),
                                    span: s.span,
                                },
                            }],
                        },
                        span,
                    }
                } else {
                    self.parse_expr()?
                };
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Stimulus(e), start))
            }
            "compare" if next_is_ident => {
                self.advance();
                let mut words = Vec::new();
                let mut signals = None;
                let mut within = None;
                while self.at_ident() && !self.at_word("within") {
                    words.push(self.expect_ident()?);
                }
                if self.at_punct(Punct::LBrace) {
                    signals = Some(self.parse_delimited(
                        Punct::LBrace,
                        Punct::RBrace,
                        &[Punct::Comma],
                        |p| p.parse_expr(),
                    )?);
                }
                if self.eat_word("within") {
                    let n = self.parse_expr()?;
                    self.expect_word("cycles")?;
                    self.expect_word("of")?;
                    let clk = self.expect_ident()?;
                    within = Some((n, clk));
                }
                self.expect_semi()?;
                Ok(self.mk(
                    StmtKind::Compare {
                        words,
                        signals,
                        within,
                    },
                    start,
                ))
            }
            "report" if next_is_ident => {
                self.advance();
                let what = self.expect_ident()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Report(what), start))
            }
            "begins" if self.nth_is_word(1, "when") => {
                self.advance();
                self.advance();
                let cond = self.parse_expr()?;
                self.expect_semi()?;
                Ok(self.mk(StmtKind::BeginsWhen(cond), start))
            }
            "responds" if self.nth_is_word(1, "with") => {
                self.advance();
                self.advance();
                let names =
                    self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
                        p.expect_ident()
                    })?;
                let body = self.parse_block()?;
                Ok(self.mk(StmtKind::RespondsWith { names, body }, start))
            }
            "hold" if next_is_ident => {
                self.advance();
                let target = self.parse_postfix()?;
                self.expect_punct(Punct::Eq)?;
                let value = self.parse_binary(super::expr::PREC_ABOVE_PIPE)?;
                self.expect_kw(Keyword::For)?;
                let cycles = self.parse_postfix()?;
                self.expect_word("cycles")?;
                self.expect_semi()?;
                Ok(self.mk(
                    StmtKind::Hold {
                        target,
                        value,
                        cycles,
                    },
                    start,
                ))
            }
            "drive" if next_is_ident => {
                self.advance();
                let target = self.parse_postfix()?;
                self.expect_punct(Punct::Eq)?;
                let value = self.parse_postfix()?;
                self.expect_word("on")?;
                let mut when = Vec::new();
                while self.at_ident() {
                    when.push(self.expect_ident()?);
                }
                self.expect_semi()?;
                Ok(self.mk(
                    StmtKind::Drive {
                        target,
                        value,
                        when,
                    },
                    start,
                ))
            }
            "sample" if next_is_ident => {
                self.advance();
                let target = self.parse_postfix()?;
                self.expect_word("on")?;
                let mut when = Vec::new();
                while self.at_ident() {
                    when.push(self.expect_ident()?);
                }
                self.expect_semi()?;
                Ok(self.mk(StmtKind::Sample { target, when }, start))
            }
            _ => self.parse_expr_stmt(start),
        }
    }

    /// `routine name(params) (-> Type)? (@ clk)? { body }`; the `routine`
    /// word is at the cursor.
    pub(crate) fn parse_routine(&mut self, docs: Docs, istream: bool) -> PResult<RoutineDecl> {
        let start = self.expect_word("routine")?;
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        let ret = if self.eat_punct(Punct::RArrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let clock = self.parse_clock_ref()?;
        let body = self.parse_block()?;
        Ok(RoutineDecl {
            docs,
            istream,
            name,
            params,
            ret,
            clock,
            body,
            span: start.to(self.prev_span()),
        })
    }
}

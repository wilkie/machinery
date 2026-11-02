//! Expression syntax: precedence climbing over the table in SPEC.md §5.

use super::{PResult, Parser};
use crate::ast::*;
use crate::token::{Keyword, Punct, TokenKind};

/// Binding power of binary operators; higher binds tighter.
fn binary_op(kind: &TokenKind) -> Option<(BinaryOp, u8)> {
    let p = match kind {
        TokenKind::Punct(p) => *p,
        _ => return None,
    };
    Some(match p {
        Punct::Star => (BinaryOp::Mul, 10),
        Punct::Slash => (BinaryOp::Div, 10),
        Punct::Percent => (BinaryOp::Rem, 10),
        Punct::Plus => (BinaryOp::Add, 9),
        Punct::Minus => (BinaryOp::Sub, 9),
        Punct::Shl => (BinaryOp::Shl, 8),
        Punct::Shr => (BinaryOp::Shr, 8),
        Punct::Amp => (BinaryOp::BitAnd, 7),
        Punct::Caret => (BinaryOp::BitXor, 6),
        Punct::Pipe => (BinaryOp::BitOr, 5),
        Punct::EqEq => (BinaryOp::Eq, 4),
        Punct::Ne => (BinaryOp::Ne, 4),
        Punct::Lt => (BinaryOp::Lt, 4),
        Punct::Le => (BinaryOp::Le, 4),
        Punct::Gt => (BinaryOp::Gt, 4),
        Punct::Ge => (BinaryOp::Ge, 4),
        Punct::AmpAmp => (BinaryOp::And, 3),
        Punct::PipePipe => (BinaryOp::Or, 2),
        _ => return None,
    })
}

/// Precedence just above `|`, used for match patterns so `A | B` splits.
pub(crate) const PREC_ABOVE_PIPE: u8 = 6;
const PREC_LOWEST: u8 = 2;

impl Parser {
    /// Full expression: ranges over ternaries over binaries.
    pub(crate) fn parse_expr(&mut self) -> PResult<Expr> {
        let lo = self.parse_ternary()?;
        if self.at_punct(Punct::DotDot) || self.at_punct(Punct::DotDotEq) {
            let inclusive = self.advance().kind == TokenKind::Punct(Punct::DotDotEq);
            let hi = self.parse_ternary()?;
            let span = lo.span.to(hi.span);
            return Ok(Expr {
                kind: ExprKind::Range {
                    lo: Box::new(lo),
                    hi: Box::new(hi),
                    inclusive,
                },
                span,
            });
        }
        Ok(lo)
    }

    /// An expression in which `>`-family tokens are not operators (array
    /// lengths inside `[T; N]`, generic arguments).
    pub(crate) fn parse_expr_no_gt(&mut self) -> PResult<Expr> {
        self.no_gt_ops += 1;
        let r = self.parse_expr();
        self.no_gt_ops -= 1;
        r
    }

    /// An expression where `Name {` is not a bundle literal (`if`/`match`
    /// heads).
    pub(crate) fn parse_expr_no_struct(&mut self) -> PResult<Expr> {
        self.no_struct_literal += 1;
        let r = self.parse_expr();
        self.no_struct_literal -= 1;
        r
    }

    fn parse_ternary(&mut self) -> PResult<Expr> {
        let cond = self.parse_binary(PREC_LOWEST)?;
        if self.eat_punct(Punct::Question) {
            let a = self.parse_ternary()?;
            self.expect_punct(Punct::Colon)?;
            let b = self.parse_ternary()?;
            let span = cond.span.to(b.span);
            return Ok(Expr {
                kind: ExprKind::Ternary(Box::new(cond), Box::new(a), Box::new(b)),
                span,
            });
        }
        Ok(cond)
    }

    pub(crate) fn parse_binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let Some((op, prec)) = binary_op(self.peek_kind()) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            if self.no_gt_ops > 0 && matches!(op, BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Shr) {
                break;
            }
            self.advance();
            let rhs = self.parse_binary(prec + 1)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let op = match self.peek_kind() {
            TokenKind::Punct(Punct::Bang) => Some(UnaryOp::Not),
            TokenKind::Punct(Punct::Tilde) => Some(UnaryOp::BitNot),
            TokenKind::Punct(Punct::Minus) => Some(UnaryOp::Neg),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.advance().span;
            let inner = self.parse_unary()?;
            let span = start.to(inner.span);
            return Ok(Expr {
                kind: ExprKind::Unary(op, Box::new(inner)),
                span,
            });
        }
        self.parse_postfix()
    }

    /// Primary followed by `.f`, `[i]`, `[hi:lo]`, `(args)`, `as T`.
    pub(crate) fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.at_punct(Punct::Dot) && self.nth_is_ident(1) {
                self.advance();
                let field = self.expect_ident()?;
                let span = e.span.to(field.span);
                // `mem.read<16>(a)`: a method with a width (SPEC 11.2d).
                let generic = self.at_punct(Punct::Lt)
                    && self.peek().span.start == field.span.end
                    && matches!(self.peek_nth(1).kind, TokenKind::Int(_))
                    && matches!(self.peek_nth(2).kind, TokenKind::Punct(Punct::Gt))
                    && matches!(self.peek_nth(3).kind, TokenKind::Punct(Punct::LParen));
                e = Expr {
                    kind: ExprKind::Field(Box::new(e), field),
                    span,
                };
                if generic {
                    let generics = self.parse_generic_args()?;
                    let args = self.parse_args()?;
                    let span = e.span.to(self.prev_span());
                    e = Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(e),
                            generics,
                            args,
                        },
                        span,
                    };
                }
            } else if self.at_punct(Punct::LBracket) {
                self.advance();
                let first = self.parse_expr()?;
                if self.eat_punct(Punct::Colon) {
                    let lo = self.parse_expr()?;
                    let end = self.expect_punct(Punct::RBracket)?;
                    let span = e.span.to(end);
                    e = Expr {
                        kind: ExprKind::Slice(Box::new(e), Box::new(first), Box::new(lo)),
                        span,
                    };
                } else {
                    let end = self.expect_punct(Punct::RBracket)?;
                    let span = e.span.to(end);
                    e = Expr {
                        kind: ExprKind::Index(Box::new(e), Box::new(first)),
                        span,
                    };
                }
            } else if self.at_punct(Punct::LParen) {
                let args = self.parse_args()?;
                let span = e.span.to(self.prev_span());
                e = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(e),
                        generics: Vec::new(),
                        args,
                    },
                    span,
                };
            } else if self.at_kw(Keyword::As) {
                self.advance();
                let ty = self.parse_type()?;
                let span = e.span.to(ty.span);
                e = Expr {
                    kind: ExprKind::Cast(Box::new(e), ty),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// `( arg, name: arg, ... )`
    pub(crate) fn parse_args(&mut self) -> PResult<Vec<Arg>> {
        self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
            p.parse_arg()
        })
    }

    fn parse_arg(&mut self) -> PResult<Arg> {
        let start = self.cur_span();
        let name = if self.at_ident() && self.nth_is_punct(1, Punct::Colon) {
            let n = self.expect_ident()?;
            self.expect_punct(Punct::Colon)?;
            Some(n)
        } else {
            None
        };
        // Arguments may be bundle literals even inside `if` heads.
        let saved = self.no_struct_literal;
        self.no_struct_literal = 0;
        let value = self.parse_expr();
        self.no_struct_literal = saved;
        let value = value?;
        let span = start.to(value.span);
        Ok(Arg { name, value, span })
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let tok = self.peek().clone();
        let start = tok.span;
        match tok.kind {
            TokenKind::Int(lit) => {
                self.advance();
                if let TokenKind::Ident(w) = self.peek_kind() {
                    if let Some(unit) = Unit::from_word(w) {
                        let unit_span = self.advance().span;
                        return Ok(Expr {
                            kind: ExprKind::Quantity {
                                value: lit.value as f64,
                                text: lit.value.to_string(),
                                unit,
                            },
                            span: start.to(unit_span),
                        });
                    }
                }
                Ok(Expr {
                    kind: ExprKind::Int {
                        value: lit.value,
                        suffix: lit.suffix,
                    },
                    span: start,
                })
            }
            TokenKind::Float(v) => {
                self.advance();
                let text = v.to_string();
                match self.peek_kind().clone() {
                    TokenKind::Ident(w) if Unit::from_word(&w).is_some() => {
                        let unit_span = self.advance().span;
                        Ok(Expr {
                            kind: ExprKind::Quantity {
                                value: v,
                                text,
                                unit: Unit::from_word(&w).unwrap(),
                            },
                            span: start.to(unit_span),
                        })
                    }
                    _ => Err(self.error(
                        "E0101",
                        start,
                        "a number with a fractional part must be followed by a unit (`MHz`, `ns`, ...)",
                    )),
                }
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Str(s),
                    span: start,
                })
            }
            TokenKind::Keyword(Keyword::True) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(true),
                    span: start,
                })
            }
            TokenKind::Keyword(Keyword::False) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(false),
                    span: start,
                })
            }
            TokenKind::Punct(Punct::Underscore) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Wildcard,
                    span: start,
                })
            }
            TokenKind::Punct(Punct::LParen) => self.parse_paren_or_tuple(),
            TokenKind::Punct(Punct::LBracket) => self.parse_array_lit(),
            TokenKind::Punct(Punct::LBrace) => {
                let block = self.parse_block()?;
                let span = block.span;
                Ok(Expr {
                    kind: ExprKind::Block(block),
                    span,
                })
            }
            TokenKind::Keyword(Keyword::Match) => self.parse_match_expr(),
            TokenKind::Keyword(Keyword::If) => self.parse_if_expr(),
            TokenKind::Ident(ref w) => {
                // Contextual expression keywords.
                if w == "call" && self.nth_is_ident(1) {
                    self.advance();
                    let callee = self.parse_postfix()?;
                    let span = start.to(callee.span);
                    return Ok(Expr {
                        kind: ExprKind::RoutineCall(Box::new(callee)),
                        span,
                    });
                }
                if w == "fetch" && self.nth_is_ident(1) {
                    self.advance();
                    let operand = self.parse_path_colons()?;
                    let span = start.to(operand.span);
                    return Ok(Expr {
                        kind: ExprKind::Fetch(operand),
                        span,
                    });
                }
                let mut path = self.parse_path_colons()?;
                // `r<16>` written without spaces and not called: an instance
                // of a family (SPEC 11.2d).
                if self.at_punct(Punct::Lt)
                    && !matches!(self.peek_nth(3).kind, TokenKind::Punct(Punct::LParen))
                {
                    if let Some(last) = path.segments.pop() {
                        let last = self.fuse_width(last);
                        path.span = path.span.to(last.span);
                        path.segments.push(last);
                    }
                }
                // Generic call: `Alu<8>(...)`. Speculative because `<` may be
                // a comparison.
                if self.at_punct(Punct::Lt) {
                    let p2 = path.clone();
                    if let Some(e) = self.speculate(|p| {
                        let generics = p.parse_generic_args().ok()?;
                        if !p.at_punct(Punct::LParen) {
                            return None;
                        }
                        let args = p.parse_args().ok()?;
                        let span = p2.span.to(p.prev_span());
                        Some(Expr {
                            kind: ExprKind::Call {
                                callee: Box::new(Expr {
                                    span: p2.span,
                                    kind: ExprKind::Path(p2.clone()),
                                }),
                                generics,
                                args,
                            },
                            span,
                        })
                    }) {
                        return Ok(e);
                    }
                }
                // Bundle literal: `Name { field: value, ... }`.
                if self.at_punct(Punct::LBrace)
                    && self.no_struct_literal == 0
                    && Self::is_type_like(&path)
                {
                    let (fields, rest_default) = self.parse_field_inits(false)?;
                    let span = path.span.to(self.prev_span());
                    return Ok(Expr {
                        kind: ExprKind::BundleLit {
                            ty: path,
                            fields,
                            rest_default,
                        },
                        span,
                    });
                }
                let span = path.span;
                Ok(Expr {
                    kind: ExprKind::Path(path),
                    span,
                })
            }
            _ => Err(self.error_expected("an expression")),
        }
    }

    /// Bundle literals require a type-like (capitalised) last segment, so
    /// `default {`, `expect {` and other lowercase block words never parse as
    /// literals.
    fn is_type_like(path: &Path) -> bool {
        path.last()
            .name
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
    }

    fn parse_paren_or_tuple(&mut self) -> PResult<Expr> {
        let start = self.cur_span();
        let saved = self.no_struct_literal;
        self.no_struct_literal = 0;
        let result = (|| {
            self.expect_punct(Punct::LParen)?;
            if self.at_punct(Punct::RParen) {
                let end = self.advance().span;
                return Ok(Expr {
                    kind: ExprKind::Tuple(Vec::new()),
                    span: start.to(end),
                });
            }
            let first = self.parse_arg()?;
            if self.at_punct(Punct::RParen) && first.name.is_none() {
                let end = self.advance().span;
                let mut e = first.value;
                e.span = start.to(end);
                return Ok(e);
            }
            let mut elems = vec![first];
            while self.eat_punct(Punct::Comma) {
                if self.at_punct(Punct::RParen) {
                    break;
                }
                elems.push(self.parse_arg()?);
            }
            let end = self.expect_punct(Punct::RParen)?;
            Ok(Expr {
                kind: ExprKind::Tuple(elems),
                span: start.to(end),
            })
        })();
        self.no_struct_literal = saved;
        result
    }

    fn parse_array_lit(&mut self) -> PResult<Expr> {
        let start = self.expect_punct(Punct::LBracket)?;
        if self.at_punct(Punct::RBracket) {
            let end = self.advance().span;
            return Ok(Expr {
                kind: ExprKind::ArrayLit(Vec::new()),
                span: start.to(end),
            });
        }
        let first = self.parse_expr()?;
        if self.eat_punct(Punct::Semi) {
            let count = self.parse_expr()?;
            let end = self.expect_punct(Punct::RBracket)?;
            return Ok(Expr {
                kind: ExprKind::ArrayRepeat(Box::new(first), Box::new(count)),
                span: start.to(end),
            });
        }
        let mut elems = vec![first];
        while self.eat_punct(Punct::Comma) {
            if self.at_punct(Punct::RBracket) {
                break;
            }
            elems.push(self.parse_expr()?);
        }
        let end = self.expect_punct(Punct::RBracket)?;
        Ok(Expr {
            kind: ExprKind::ArrayLit(elems),
            span: start.to(end),
        })
    }

    pub(crate) fn parse_match_expr(&mut self) -> PResult<Expr> {
        let start = self.expect_kw(Keyword::Match)?;
        let scrutinee = self.parse_expr_no_struct()?;
        let arms = self.parse_match_arms()?;
        Ok(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: start.to(self.prev_span()),
        })
    }

    /// `{ pat | pat => body, ... }`. A block body needs no trailing comma.
    pub(crate) fn parse_match_arms(&mut self) -> PResult<Vec<MatchArm>> {
        self.expect_punct(Punct::LBrace)?;
        let mut arms = Vec::new();
        loop {
            if self.eat_punct(Punct::RBrace) {
                return Ok(arms);
            }
            if self.at_eof() {
                return Err(self.error_expected("`}`"));
            }
            let arm_start = self.cur_span();
            let mut patterns = vec![self.parse_pattern()?];
            while self.eat_punct(Punct::Pipe) {
                patterns.push(self.parse_pattern()?);
            }
            self.expect_punct(Punct::FatArrow)?;
            let body = if self.at_punct(Punct::LBrace) {
                ArmBody::Block(self.parse_block()?)
            } else {
                ArmBody::Expr(self.parse_expr()?)
            };
            let span = arm_start.to(self.prev_span());
            let is_block = matches!(body, ArmBody::Block(_));
            arms.push(MatchArm {
                patterns,
                body,
                span,
            });
            if self.eat_punct(Punct::Comma) {
                continue;
            }
            if self.at_punct(Punct::RBrace) {
                continue;
            }
            if is_block {
                continue;
            }
            return Err(self.error_expected("`,` or `}` after match arm"));
        }
    }

    fn parse_pattern(&mut self) -> PResult<Pattern> {
        let start = self.cur_span();
        if self.at_punct(Punct::Underscore) {
            self.advance();
            return Ok(Pattern {
                kind: PatternKind::Wildcard,
                span: start,
            });
        }
        self.no_struct_literal += 1;
        let lo = self.parse_binary(PREC_ABOVE_PIPE);
        self.no_struct_literal -= 1;
        let lo = lo?;
        if self.at_punct(Punct::DotDot) || self.at_punct(Punct::DotDotEq) {
            let inclusive = self.advance().kind == TokenKind::Punct(Punct::DotDotEq);
            self.no_struct_literal += 1;
            let hi = self.parse_binary(PREC_ABOVE_PIPE);
            self.no_struct_literal -= 1;
            let hi = hi?;
            let span = start.to(hi.span);
            return Ok(Pattern {
                kind: PatternKind::Range { lo, hi, inclusive },
                span,
            });
        }
        let span = lo.span;
        Ok(Pattern {
            kind: PatternKind::Expr(lo),
            span,
        })
    }

    /// `if c { } else if c { } else { }` in expression position.
    pub(crate) fn parse_if_expr(&mut self) -> PResult<Expr> {
        let start = self.expect_kw(Keyword::If)?;
        let cond = self.parse_expr_no_struct()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.eat_kw(Keyword::Else) {
            if self.at_kw(Keyword::If) {
                Some(Box::new(self.parse_if_expr()?))
            } else {
                let b = self.parse_block()?;
                let span = b.span;
                Some(Box::new(Expr {
                    kind: ExprKind::Block(b),
                    span,
                }))
            }
        } else {
            None
        };
        Ok(Expr {
            kind: ExprKind::If {
                cond: Box::new(cond),
                then_branch,
                else_branch,
            },
            span: start.to(self.prev_span()),
        })
    }
}

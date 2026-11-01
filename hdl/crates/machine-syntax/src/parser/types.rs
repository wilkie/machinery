//! Type syntax.

use super::{PResult, Parser};
use crate::ast::*;
use crate::token::{Punct, TokenKind};

impl Parser {
    pub(crate) fn parse_type(&mut self) -> PResult<Type> {
        let start = self.cur_span();
        let kind = match self.peek_kind().clone() {
            TokenKind::Punct(Punct::LBracket) => {
                self.advance();
                let elem = self.parse_type()?;
                self.expect_punct(Punct::Semi)?;
                let len = self.parse_expr_no_gt()?;
                self.expect_punct(Punct::RBracket)?;
                TypeKind::Array(Box::new(elem), Box::new(len))
            }
            TokenKind::Punct(Punct::LParen) => {
                let elems =
                    self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
                        let name = if p.at_ident() && p.nth_is_punct(1, Punct::Colon) {
                            let n = p.expect_ident()?;
                            p.expect_punct(Punct::Colon)?;
                            Some(n)
                        } else {
                            None
                        };
                        let ty = p.parse_type()?;
                        Ok(TupleElem { name, ty })
                    })?;
                TypeKind::Tuple(elems)
            }
            TokenKind::Ident(name) => {
                if name == "bool" {
                    self.advance();
                    TypeKind::Bool
                } else if name == "width" {
                    self.advance();
                    TypeKind::Width
                } else if let Some(k) = Self::parse_sized_int_name(&name) {
                    let tok = self.advance();
                    let (signed, width) = k;
                    let w = Expr {
                        kind: ExprKind::Int {
                            value: width as u128,
                            suffix: None,
                        },
                        span: tok.span,
                    };
                    if signed {
                        TypeKind::Signed(Box::new(w))
                    } else {
                        TypeKind::Unsigned(Box::new(w))
                    }
                } else if (name == "u" || name == "i") && self.nth_is_punct(1, Punct::Lt) {
                    self.advance();
                    self.expect_punct(Punct::Lt)?;
                    let w = self.parse_expr_no_gt()?;
                    self.expect_gt()?;
                    if name == "i" {
                        TypeKind::Signed(Box::new(w))
                    } else {
                        TypeKind::Unsigned(Box::new(w))
                    }
                } else {
                    let path = self.parse_path_colons()?;
                    let generics = if self.at_punct(Punct::Lt) {
                        self.parse_generic_args()?
                    } else {
                        Vec::new()
                    };
                    TypeKind::Named(path, generics)
                }
            }
            _ => return Err(self.error_expected("a type")),
        };
        Ok(Type {
            kind,
            span: start.to(self.prev_span()),
        })
    }

    /// `u8` → `Some((false, 8))`, `i16` → `Some((true, 16))`, else `None`.
    fn parse_sized_int_name(name: &str) -> Option<(bool, u32)> {
        let (first, rest) = name.split_at(1);
        let signed = match first {
            "u" => false,
            "i" => true,
            _ => return None,
        };
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) || rest.starts_with('0') {
            return None;
        }
        rest.parse::<u32>().ok().map(|w| (signed, w))
    }

    /// `<arg, arg>`: each argument is a type when it unambiguously looks like
    /// one, otherwise an expression (a bare name is an expression; semantic
    /// analysis resolves it).
    pub(crate) fn parse_generic_args(&mut self) -> PResult<Vec<GenericArg>> {
        self.expect_punct(Punct::Lt)?;
        self.no_gt_ops += 1;
        let result = (|| {
            let mut args = Vec::new();
            loop {
                if self.at_punct(Punct::Gt) || self.at_punct(Punct::Shr) {
                    break;
                }
                let arg = if self.looks_like_type_start() {
                    GenericArg::Type(self.parse_type()?)
                } else {
                    GenericArg::Expr(self.parse_expr()?)
                };
                args.push(arg);
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
            self.expect_gt()?;
            Ok(args)
        })();
        self.no_gt_ops -= 1;
        result
    }

    /// Conservative: `[`, `bool`, `uN`/`iN`, `u<`/`i<`, or a name followed by
    /// `<` (a nested generic type; comparisons are meaningless in generic
    /// arguments) or `::` (a namespaced type such as a bus role).
    fn looks_like_type_start(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Punct(Punct::LBracket) => true,
            TokenKind::Ident(n) => {
                n == "bool"
                    || Self::parse_sized_int_name(n).is_some()
                    || self.nth_is_punct(1, Punct::Lt)
                    || self.nth_is_punct(1, Punct::ColonColon)
            }
            _ => false,
        }
    }
}

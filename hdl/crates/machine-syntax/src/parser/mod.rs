//! Recursive-descent parser.
//!
//! The parser is total: it always produces a [`File`], substituting `Error`
//! nodes where it had to give up on a construct, and reports what went wrong
//! through [`Diagnostics`]. Recovery is at statement and declaration
//! granularity: on an error inside a block it skips to the next `;` or the
//! closing `}`; at the top level it skips to the next declaration keyword.
//!
//! Most keywords are contextual (see [`crate::token`]), so the parser
//! matches on identifier text where the grammar expects a particular word.

mod decl;
mod expr;
mod stmt;
mod types;

use crate::ast::*;
use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::lexer;
use crate::span::Span;
use crate::token::{Keyword, Punct, Token, TokenKind};

pub struct Parsed {
    pub file: File,
    pub diagnostics: Diagnostics,
}

/// Parse one source file.
pub fn parse(text: &str) -> Parsed {
    let lexed = lexer::lex(text);
    let mut parser = Parser::new(lexed.tokens);
    parser.diagnostics.extend(lexed.diagnostics);
    let file = parser.parse_file();
    Parsed {
        file,
        diagnostics: parser.diagnostics,
    }
}

/// Parse a standalone expression (for tools and tests). The whole input must
/// be one expression.
pub fn parse_expr_str(text: &str) -> (Option<Expr>, Diagnostics) {
    let lexed = lexer::lex(text);
    let mut parser = Parser::new(lexed.tokens);
    parser.diagnostics.extend(lexed.diagnostics);
    let expr = parser.parse_expr().ok();
    if expr.is_some() && !parser.at_eof() {
        parser.error_expected("end of input");
    }
    let expr = if parser.diagnostics.has_errors() {
        None
    } else {
        expr
    };
    (expr, parser.diagnostics)
}

/// Marker that a diagnostic has already been emitted for this failure.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParseError;

pub(crate) type PResult<T> = Result<T, ParseError>;

/// Words of the top-level grammar, used for recovery.
const TOP_LEVEL_WORDS: &[&str] = &[
    "enum",
    "bundle",
    "union",
    "unit",
    "component",
    "bus",
    "isa",
    "system",
    "test",
    "vectors",
    "conformance",
];

/// After this many errors the parser stops: further output would be noise,
/// and a bounded count guarantees bounded memory even if recovery misbehaves.
const MAX_ERRORS: usize = 200;

pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub(crate) diagnostics: Diagnostics,
    /// Set once `MAX_ERRORS` is reached; the parser then behaves as if at EOF.
    bailed: bool,
    /// When > 0, `Foo {` is not a bundle literal (we are in an `if`/`match`
    /// head where `{` opens the body).
    no_struct_literal: u32,
    /// When > 0, `>`, `>=` and `>>` are not binary operators (we are inside
    /// generic arguments).
    no_gt_ops: u32,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            pos: 0,
            diagnostics: Diagnostics::new(),
            bailed: false,
            no_struct_literal: 0,
            no_gt_ops: 0,
        }
    }

    // -- cursor ------------------------------------------------------------

    /// Index of the next non-doc-comment token at or after `self.pos`.
    fn skip_docs_from(&self, mut i: usize) -> usize {
        while matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::DocComment(_))
        ) {
            i += 1;
        }
        i.min(self.tokens.len() - 1)
    }

    pub(crate) fn peek(&self) -> &Token {
        &self.tokens[self.skip_docs_from(self.pos)]
    }

    pub(crate) fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    /// The `n`th non-doc token after the current one (`peek_nth(0) == peek()`).
    pub(crate) fn peek_nth(&self, n: usize) -> &Token {
        let mut i = self.skip_docs_from(self.pos);
        for _ in 0..n {
            if i + 1 >= self.tokens.len() {
                break;
            }
            i = self.skip_docs_from(i + 1);
        }
        &self.tokens[i]
    }

    pub(crate) fn advance(&mut self) -> Token {
        let i = self.skip_docs_from(self.pos);
        let tok = self.tokens[i].clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos = i + 1;
        } else {
            self.pos = i;
        }
        tok
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.bailed || matches!(self.peek_kind(), TokenKind::Eof)
    }

    /// Consume any doc comments at the cursor and return their text.
    pub(crate) fn take_docs(&mut self) -> Docs {
        let mut docs = Vec::new();
        while let Some(Token {
            kind: TokenKind::DocComment(text),
            ..
        }) = self.tokens.get(self.pos)
        {
            docs.push(text.clone());
            self.pos += 1;
        }
        docs
    }

    /// Span of the previously consumed token.
    pub(crate) fn prev_span(&self) -> Span {
        let mut i = self.pos;
        while i > 0 {
            i -= 1;
            if !matches!(self.tokens[i].kind, TokenKind::DocComment(_)) {
                return self.tokens[i].span;
            }
        }
        Span::DUMMY
    }

    pub(crate) fn cur_span(&self) -> Span {
        self.peek().span
    }

    /// Was the previously consumed token a `}`?
    pub(crate) fn prev_is_rbrace(&self) -> bool {
        let mut i = self.pos;
        while i > 0 {
            i -= 1;
            match &self.tokens[i].kind {
                TokenKind::DocComment(_) => continue,
                TokenKind::Punct(Punct::RBrace) => return true,
                _ => return false,
            }
        }
        false
    }

    // -- predicates ---------------------------------------------------------

    pub(crate) fn at_punct(&self, p: Punct) -> bool {
        matches!(self.peek_kind(), TokenKind::Punct(q) if *q == p)
    }

    pub(crate) fn nth_is_punct(&self, n: usize, p: Punct) -> bool {
        matches!(&self.peek_nth(n).kind, TokenKind::Punct(q) if *q == p)
    }

    pub(crate) fn at_kw(&self, k: Keyword) -> bool {
        matches!(self.peek_kind(), TokenKind::Keyword(q) if *q == k)
    }

    /// Is the next token the contextual word `w`?
    pub(crate) fn at_word(&self, w: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(s) if s == w)
    }

    pub(crate) fn nth_is_word(&self, n: usize, w: &str) -> bool {
        matches!(&self.peek_nth(n).kind, TokenKind::Ident(s) if s == w)
    }

    pub(crate) fn nth_is_ident(&self, n: usize) -> bool {
        matches!(&self.peek_nth(n).kind, TokenKind::Ident(_))
    }

    pub(crate) fn at_ident(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(_))
    }

    // -- consumers -----------------------------------------------------------

    pub(crate) fn eat_punct(&mut self, p: Punct) -> bool {
        if self.at_punct(p) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_kw(&mut self, k: Keyword) -> bool {
        if self.at_kw(k) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_word(&mut self, w: &str) -> bool {
        if self.at_word(w) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn expect_punct(&mut self, p: Punct) -> PResult<Span> {
        if self.at_punct(p) {
            Ok(self.advance().span)
        } else {
            Err(self.error_expected(&format!("`{}`", p.as_str())))
        }
    }

    /// Expect `>`; splits a `>>` token in two so nested generics close.
    pub(crate) fn expect_gt(&mut self) -> PResult<Span> {
        if self.at_punct(Punct::Gt) {
            return Ok(self.advance().span);
        }
        if self.at_punct(Punct::Shr) {
            let i = self.skip_docs_from(self.pos);
            let span = self.tokens[i].span;
            let first = Span::new(span.start, span.start + 1);
            self.tokens[i].kind = TokenKind::Punct(Punct::Gt);
            self.tokens[i].span = Span::new(span.start + 1, span.end);
            return Ok(first);
        }
        Err(self.error_expected("`>`"))
    }

    pub(crate) fn expect_kw(&mut self, k: Keyword) -> PResult<Span> {
        if self.at_kw(k) {
            Ok(self.advance().span)
        } else {
            Err(self.error_expected(&format!("`{}`", k.as_str())))
        }
    }

    pub(crate) fn expect_word(&mut self, w: &str) -> PResult<Span> {
        if self.at_word(w) {
            Ok(self.advance().span)
        } else {
            Err(self.error_expected(&format!("`{w}`")))
        }
    }

    pub(crate) fn expect_ident(&mut self) -> PResult<Ident> {
        match self.peek_kind() {
            TokenKind::Ident(_) => {
                let tok = self.advance();
                let name = match tok.kind {
                    TokenKind::Ident(s) => s,
                    _ => unreachable!(),
                };
                Ok(Ident {
                    name,
                    span: tok.span,
                })
            }
            _ => Err(self.error_expected("identifier")),
        }
    }

    pub(crate) fn expect_str(&mut self) -> PResult<StrLit> {
        match self.peek_kind() {
            TokenKind::Str(_) => {
                let tok = self.advance();
                let value = match tok.kind {
                    TokenKind::Str(s) => s,
                    _ => unreachable!(),
                };
                Ok(StrLit {
                    value,
                    span: tok.span,
                })
            }
            _ => Err(self.error_expected("string literal")),
        }
    }

    pub(crate) fn expect_semi(&mut self) -> PResult<()> {
        self.expect_punct(Punct::Semi).map(|_| ())
    }

    // -- errors ---------------------------------------------------------------

    pub(crate) fn error(
        &mut self,
        code: &'static str,
        span: Span,
        msg: impl Into<String>,
    ) -> ParseError {
        if self.bailed {
            return ParseError;
        }
        self.diagnostics.push(Diagnostic::error(code, span, msg));
        if self.diagnostics.error_count() >= MAX_ERRORS {
            self.bailed = true;
            self.diagnostics.push(Diagnostic::error(
                "E0199",
                span,
                format!("too many errors ({MAX_ERRORS}); stopping"),
            ));
            // Behave as if the input ended here.
            self.pos = self.tokens.len() - 1;
        }
        ParseError
    }

    pub(crate) fn error_expected(&mut self, what: &str) -> ParseError {
        let found = self.peek_kind().describe();
        let span = self.cur_span();
        self.error("E0100", span, format!("expected {what}, found {found}"))
    }

    pub(crate) fn warning(&mut self, code: &'static str, span: Span, msg: impl Into<String>) {
        self.diagnostics.push(Diagnostic::warning(code, span, msg));
    }

    // -- recovery -------------------------------------------------------------

    /// Skip tokens until a `;` (consumed) or an unmatched `}` (not consumed)
    /// at the current nesting level, or EOF. A stray `)` or `]` at this level
    /// can never be matched by the enclosing block, so it is consumed; this
    /// guarantees the caller's loop makes progress.
    pub(crate) fn recover_in_block(&mut self) {
        let mut depth = 0i32;
        loop {
            match self.peek_kind() {
                TokenKind::Eof => return,
                TokenKind::Punct(Punct::LBrace)
                | TokenKind::Punct(Punct::LParen)
                | TokenKind::Punct(Punct::LBracket) => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Punct(Punct::RBrace) => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        // Closed a nested block; a statement may end here.
                        return;
                    }
                }
                TokenKind::Punct(Punct::RParen) | TokenKind::Punct(Punct::RBracket) => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    self.advance();
                }
                TokenKind::Punct(Punct::Semi) if depth == 0 => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Parse items until the closing `}` of an already-opened block, calling
    /// `item` for each and recovering on failure. Always makes progress, so
    /// it terminates on any input. Consumes the `}`.
    pub(crate) fn parse_items_until_rbrace<T>(
        &mut self,
        mut item: impl FnMut(&mut Parser) -> PResult<T>,
    ) -> PResult<Vec<T>> {
        let mut items = Vec::new();
        loop {
            if self.at_punct(Punct::RBrace) {
                self.advance();
                return Ok(items);
            }
            if self.at_eof() {
                return Err(self.error_expected("`}`"));
            }
            let before = self.pos;
            match item(self) {
                Ok(v) => items.push(v),
                Err(_) => self.recover_in_block(),
            }
            if self.pos == before && !self.at_punct(Punct::RBrace) {
                self.advance();
            }
        }
    }

    /// Skip to the next top-level declaration word at brace depth zero.
    pub(crate) fn recover_top_level(&mut self) {
        let mut depth = 0i32;
        // Always make progress.
        if !self.at_eof() {
            if self.at_punct(Punct::LBrace) {
                depth += 1;
            }
            self.advance();
        }
        loop {
            match self.peek_kind() {
                TokenKind::Eof => return,
                TokenKind::Punct(Punct::LBrace) => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Punct(Punct::RBrace) => {
                    depth -= 1;
                    self.advance();
                    if depth <= 0 {
                        depth = 0;
                    }
                }
                TokenKind::Ident(w) if depth == 0 && TOP_LEVEL_WORDS.contains(&w.as_str()) => {
                    return
                }
                TokenKind::Keyword(Keyword::Use) | TokenKind::Keyword(Keyword::Const)
                    if depth == 0 =>
                {
                    return
                }
                TokenKind::DocComment(_) if depth == 0 => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Run `f` speculatively: on `None`, rewind the cursor and drop any
    /// diagnostics it produced.
    pub(crate) fn speculate<T>(&mut self, f: impl FnOnce(&mut Parser) -> Option<T>) -> Option<T> {
        let pos = self.pos;
        let diag_len = self.diagnostics.len();
        let tokens_snapshot: Option<Vec<Token>> = None;
        let _ = tokens_snapshot;
        let result = f(self);
        if result.is_none() {
            self.pos = pos;
            if self.diagnostics.len() != diag_len {
                let kept: Vec<_> = self
                    .diagnostics
                    .clone()
                    .into_vec()
                    .into_iter()
                    .take(diag_len)
                    .collect();
                let mut d = Diagnostics::new();
                for k in kept {
                    d.push(k);
                }
                self.diagnostics = d;
            }
        }
        result
    }

    // -- generic list helpers --------------------------------------------------

    /// Parse `open item (sep item)* sep? close`, tolerating a trailing
    /// separator. When an item fails (after reporting) or a separator is
    /// missing, the list skips to the next separator or the closer at this
    /// depth and continues, so one bad element does not lose the whole list.
    pub(crate) fn parse_delimited<T>(
        &mut self,
        open: Punct,
        close: Punct,
        seps: &[Punct],
        mut item: impl FnMut(&mut Parser) -> PResult<T>,
    ) -> PResult<Vec<T>> {
        self.expect_punct(open)?;
        let mut items = Vec::new();
        loop {
            if self.at_punct(close) {
                self.advance();
                return Ok(items);
            }
            if self.at_eof() {
                return Err(self.error_expected(&format!("`{}`", close.as_str())));
            }
            let before = self.pos;
            match item(self) {
                Ok(v) => items.push(v),
                Err(_) => {
                    self.skip_to_sep_or_close(close, seps);
                    if self.pos == before
                        && !self.at_punct(close)
                        && !seps.iter().any(|s| self.at_punct(*s))
                    {
                        self.advance();
                    }
                }
            }
            if self.at_punct(close) {
                self.advance();
                return Ok(items);
            }
            if self.at_eof() {
                return Err(self.error_expected(&format!("`{}`", close.as_str())));
            }
            let mut ate_sep = false;
            for &s in seps {
                if self.eat_punct(s) {
                    ate_sep = true;
                    break;
                }
            }
            if !ate_sep && self.prev_is_rbrace() {
                // A `}`-terminated item needs no separator, as in Rust.
                continue;
            }
            if !ate_sep {
                let names: Vec<String> = seps.iter().map(|s| format!("`{}`", s.as_str())).collect();
                self.error_expected(&format!("{} or `{}`", names.join(" or "), close.as_str()));
                self.skip_to_sep_or_close(close, seps);
                for &s in seps {
                    if self.eat_punct(s) {
                        break;
                    }
                }
            }
        }
    }

    /// Skip tokens until one of `seps` or `close` at the current depth (not
    /// consumed), or EOF.
    fn skip_to_sep_or_close(&mut self, close: Punct, seps: &[Punct]) {
        let mut depth = 0i32;
        loop {
            match self.peek_kind() {
                TokenKind::Eof => return,
                TokenKind::Punct(p) if depth == 0 && (*p == close || seps.contains(p)) => return,
                TokenKind::Punct(Punct::LBrace | Punct::LParen | Punct::LBracket) => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Punct(Punct::RBrace | Punct::RParen | Punct::RBracket) => {
                    if depth == 0 {
                        // An unmatched closer of another kind: let the
                        // enclosing construct deal with it.
                        return;
                    }
                    depth -= 1;
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// `( name: Type, ... )`
    pub(crate) fn parse_params(&mut self) -> PResult<Vec<Param>> {
        self.parse_delimited(Punct::LParen, Punct::RParen, &[Punct::Comma], |p| {
            p.parse_param()
        })
    }

    pub(crate) fn parse_param(&mut self) -> PResult<Param> {
        let name = self.expect_ident()?;
        self.expect_punct(Punct::Colon)?;
        let ty = self.parse_type()?;
        let span = name.span.to(ty.span);
        Ok(Param { name, ty, span })
    }

    /// `<const W: width, ...>` or nothing.
    pub(crate) fn parse_generic_params(&mut self) -> PResult<Vec<GenericParam>> {
        if !self.at_punct(Punct::Lt) {
            return Ok(Vec::new());
        }
        self.parse_delimited(Punct::Lt, Punct::Gt, &[Punct::Comma], |p| {
            let start = p.cur_span();
            if !p.eat_kw(Keyword::Const) {
                return Err(p.error_expected("`const` (only const generic parameters exist)"));
            }
            let name = p.expect_ident()?;
            p.expect_punct(Punct::Colon)?;
            let kind = p.parse_type()?;
            let span = start.to(kind.span);
            Ok(GenericParam { name, kind, span })
        })
    }

    /// `@ clk` or nothing.
    pub(crate) fn parse_clock_ref(&mut self) -> PResult<ClockRef> {
        if self.eat_punct(Punct::At) {
            Ok(Some(self.expect_ident()?))
        } else {
            Ok(None)
        }
    }

    /// `a::b::c`
    pub(crate) fn parse_path_colons(&mut self) -> PResult<Path> {
        let first = self.expect_ident()?;
        let mut span = first.span;
        let mut segments = vec![first];
        while self.at_punct(Punct::ColonColon) && self.nth_is_ident(1) {
            self.advance();
            let seg = self.expect_ident()?;
            span = span.to(seg.span);
            segments.push(seg);
        }
        Ok(Path { segments, span })
    }

    /// `a.b.c`
    pub(crate) fn parse_path_dotted(&mut self) -> PResult<Path> {
        let first = self.expect_ident()?;
        let mut span = first.span;
        let mut segments = vec![first];
        while self.at_punct(Punct::Dot) && self.nth_is_ident(1) {
            self.advance();
            let seg = self.expect_ident()?;
            span = span.to(seg.span);
            segments.push(seg);
        }
        Ok(Path { segments, span })
    }

    /// `{ name: value, name, ..default }` with `,` or `;` separators. `eq`
    /// selects `=` instead of `:` between name and value.
    pub(crate) fn parse_field_inits(&mut self, eq: bool) -> PResult<(Vec<FieldInit>, bool)> {
        let mut rest_default = false;
        let fields = self.parse_delimited(
            Punct::LBrace,
            Punct::RBrace,
            &[Punct::Comma, Punct::Semi],
            |p| {
                if p.at_punct(Punct::DotDot) {
                    let start = p.advance().span;
                    p.expect_word("default")?;
                    rest_default = true;
                    // Produce a dummy that the caller strips.
                    return Ok(FieldInit {
                        name: Ident {
                            name: "..default".into(),
                            span: start,
                        },
                        value: None,
                        span: start.to(p.prev_span()),
                    });
                }
                let name = p.expect_ident()?;
                let sep = if eq { Punct::Eq } else { Punct::Colon };
                let value = if p.eat_punct(sep) {
                    Some(p.parse_expr()?)
                } else {
                    None
                };
                let span = name.span.to(p.prev_span());
                Ok(FieldInit { name, value, span })
            },
        )?;
        let fields = fields
            .into_iter()
            .filter(|f| f.name.name != "..default")
            .collect();
        Ok((fields, rest_default))
    }

    // -- entry ----------------------------------------------------------------

    fn parse_file(&mut self) -> File {
        let start = self.cur_span();
        let mut decls = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            match self.parse_decl() {
                Ok(d) => decls.push(d),
                Err(_) => {
                    let err_start = self.tokens[before].span;
                    self.recover_top_level();
                    decls.push(Decl::Error(err_start.to(self.prev_span())));
                }
            }
            if self.pos == before && !self.at_eof() {
                // Guarantee progress.
                self.advance();
            }
        }
        File {
            decls,
            span: start.to(self.prev_span()),
        }
    }
}

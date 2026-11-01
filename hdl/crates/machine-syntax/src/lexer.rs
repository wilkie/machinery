//! Hand-written lexer.
//!
//! Produces a flat `Vec<Token>` ending in `Eof`. Lexical errors are reported
//! as diagnostics and the offending bytes skipped, so the parser always has a
//! token stream to work with.

use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::span::Span;
use crate::token::{IntLit, Keyword, Punct, Token, TokenKind};

pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Diagnostics,
}

pub fn lex(text: &str) -> Lexed {
    let mut lexer = Lexer {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        tokens: Vec::new(),
        diagnostics: Diagnostics::new(),
    };
    lexer.run();
    Lexed {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

struct Lexer<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Diagnostics,
}

impl<'a> Lexer<'a> {
    fn run(&mut self) {
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.bytes.len() {
                let p = self.pos as u32;
                self.tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(p, p),
                });
                return;
            }
            let start = self.pos;
            let b = self.bytes[self.pos];
            if b.is_ascii_alphabetic() || b == b'_' {
                self.lex_ident(start);
            } else if b.is_ascii_digit() {
                self.lex_number(start);
            } else if b == b'"' {
                self.lex_string(start);
            } else {
                self.lex_punct(start);
            }
        }
    }

    fn peek(&self, off: usize) -> u8 {
        self.bytes.get(self.pos + off).copied().unwrap_or(0)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start as u32, self.pos as u32),
        });
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.peek(0) == b'/' && self.peek(1) == b'/' {
                if self.peek(2) == b'/' && self.peek(3) != b'/' {
                    // Doc comment: keep it.
                    let start = self.pos;
                    self.pos += 3;
                    if self.peek(0) == b' ' {
                        self.pos += 1;
                    }
                    let text_start = self.pos;
                    while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                    let content = self.text[text_start..self.pos]
                        .trim_end_matches('\r')
                        .to_string();
                    self.push(TokenKind::DocComment(content), start);
                    continue;
                }
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                continue;
            }
            if self.peek(0) == b'/' && self.peek(1) == b'*' {
                let start = self.pos;
                self.pos += 2;
                loop {
                    if self.pos >= self.bytes.len() {
                        self.diagnostics.push(Diagnostic::error(
                            "E0001",
                            Span::new(start as u32, self.pos as u32),
                            "unterminated block comment",
                        ));
                        break;
                    }
                    if self.peek(0) == b'*' && self.peek(1) == b'/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
                continue;
            }
            return;
        }
    }

    fn lex_ident(&mut self, start: usize) {
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
        {
            self.pos += 1;
        }
        let word = &self.text[start..self.pos];
        let kind = if word == "_" {
            TokenKind::Punct(Punct::Underscore)
        } else if let Some(kw) = Keyword::from_word(word) {
            TokenKind::Keyword(kw)
        } else {
            TokenKind::Ident(word.to_string())
        };
        self.push(kind, start);
    }

    fn lex_number(&mut self, start: usize) {
        let radix = if self.peek(0) == b'0' {
            match self.peek(1) {
                b'x' | b'X' => 16,
                b'b' | b'B' => 2,
                b'o' | b'O' => 8,
                _ => 10,
            }
        } else {
            10
        };
        if radix != 10 {
            self.pos += 2;
        }
        let digits_start = self.pos;
        let mut value: u128 = 0;
        let mut overflow = false;
        let mut any_digit = false;
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b'_' {
                self.pos += 1;
                continue;
            }
            let d = match (c as char).to_digit(radix) {
                Some(d) => d,
                None => break,
            };
            // In base 10, a trailing `u8`/`i16` suffix starts with a letter, which
            // is not a digit, so it stops the loop naturally. In base 16, `b` and
            // `i` are not both digits (`b` is), so `0xFF_u8` works because `u` is
            // not a hex digit. `0xFFi8` would be ambiguous; require the underscore.
            any_digit = true;
            value = match value
                .checked_mul(radix as u128)
                .and_then(|v| v.checked_add(d as u128))
            {
                Some(v) => v,
                None => {
                    overflow = true;
                    0
                }
            };
            self.pos += 1;
        }
        if !any_digit {
            self.diagnostics.push(Diagnostic::error(
                "E0002",
                Span::new(start as u32, self.pos as u32),
                "number literal has no digits",
            ));
        }
        // Fractional part (decimal only): only for frequency/time quantities.
        if radix == 10 && self.peek(0) == b'.' && self.peek(1).is_ascii_digit() {
            self.pos += 1;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let cleaned: String = self.text[start..self.pos]
                .chars()
                .filter(|c| *c != '_')
                .collect();
            let v: f64 = cleaned.parse().unwrap_or(0.0);
            self.push(TokenKind::Float(v), start);
            return;
        }
        // Width suffix: `_u8`, `_i16`, or directly `u8` after decimal digits.
        let mut suffix = None;
        let save = self.pos;
        let mut p = self.pos;
        if p < self.bytes.len() && self.bytes[p] == b'_' {
            p += 1;
        }
        if p < self.bytes.len() && (self.bytes[p] == b'u' || self.bytes[p] == b'i') {
            let kind = self.bytes[p] as char;
            let mut q = p + 1;
            let wstart = q;
            while q < self.bytes.len() && self.bytes[q].is_ascii_digit() {
                q += 1;
            }
            if q > wstart
                && !(q < self.bytes.len()
                    && (self.bytes[q].is_ascii_alphanumeric() || self.bytes[q] == b'_'))
            {
                let width: u32 = self.text[wstart..q].parse().unwrap_or(0);
                if width == 0 || width > 128 {
                    self.diagnostics.push(Diagnostic::error(
                        "E0003",
                        Span::new(p as u32, q as u32),
                        format!("literal width suffix must be 1..=128, got {width}"),
                    ));
                }
                suffix = Some((kind, width));
                self.pos = q;
            }
        }
        if suffix.is_none() {
            self.pos = save;
        }
        if overflow {
            self.diagnostics.push(Diagnostic::error(
                "E0004",
                Span::new(start as u32, self.pos as u32),
                "integer literal does not fit in 128 bits",
            ));
        }
        let _ = digits_start;
        self.push(TokenKind::Int(IntLit { value, suffix }), start);
    }

    fn lex_string(&mut self, start: usize) {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            if self.pos >= self.bytes.len() || self.bytes[self.pos] == b'\n' {
                self.diagnostics.push(Diagnostic::error(
                    "E0005",
                    Span::new(start as u32, self.pos as u32),
                    "unterminated string literal",
                ));
                break;
            }
            let c = self.bytes[self.pos];
            if c == b'"' {
                self.pos += 1;
                break;
            }
            if c == b'\\' {
                let esc = self.peek(1);
                self.pos += 2;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            "E0006",
                            Span::new(self.pos as u32 - 2, self.pos as u32),
                            format!("unknown escape sequence `\\{}`", esc as char),
                        ));
                    }
                }
                continue;
            }
            // Copy one UTF-8 character.
            let ch = self.text[self.pos..].chars().next().unwrap();
            out.push(ch);
            self.pos += ch.len_utf8();
        }
        self.push(TokenKind::Str(out), start);
    }

    fn lex_punct(&mut self, start: usize) {
        let three = &self.bytes[self.pos..self.bytes.len().min(self.pos + 3)];
        let (punct, len) = match three {
            [b'.', b'.', b'=', ..] => (Punct::DotDotEq, 3),
            [b':', b':', ..] => (Punct::ColonColon, 2),
            [b'.', b'.', ..] => (Punct::DotDot, 2),
            [b'<', b'-', ..] => (Punct::LArrow, 2),
            [b'-', b'>', ..] => (Punct::RArrow, 2),
            [b'=', b'>', ..] => (Punct::FatArrow, 2),
            [b'&', b'&', ..] => (Punct::AmpAmp, 2),
            [b'|', b'|', ..] => (Punct::PipePipe, 2),
            [b'<', b'<', ..] => (Punct::Shl, 2),
            [b'>', b'>', ..] => (Punct::Shr, 2),
            [b'=', b'=', ..] => (Punct::EqEq, 2),
            [b'!', b'=', ..] => (Punct::Ne, 2),
            [b'<', b'=', ..] => (Punct::Le, 2),
            [b'>', b'=', ..] => (Punct::Ge, 2),
            [b'{', ..] => (Punct::LBrace, 1),
            [b'}', ..] => (Punct::RBrace, 1),
            [b'(', ..] => (Punct::LParen, 1),
            [b')', ..] => (Punct::RParen, 1),
            [b'[', ..] => (Punct::LBracket, 1),
            [b']', ..] => (Punct::RBracket, 1),
            [b'<', ..] => (Punct::Lt, 1),
            [b'>', ..] => (Punct::Gt, 1),
            [b',', ..] => (Punct::Comma, 1),
            [b';', ..] => (Punct::Semi, 1),
            [b':', ..] => (Punct::Colon, 1),
            [b'.', ..] => (Punct::Dot, 1),
            [b'=', ..] => (Punct::Eq, 1),
            [b'+', ..] => (Punct::Plus, 1),
            [b'-', ..] => (Punct::Minus, 1),
            [b'*', ..] => (Punct::Star, 1),
            [b'/', ..] => (Punct::Slash, 1),
            [b'%', ..] => (Punct::Percent, 1),
            [b'!', ..] => (Punct::Bang, 1),
            [b'~', ..] => (Punct::Tilde, 1),
            [b'&', ..] => (Punct::Amp, 1),
            [b'|', ..] => (Punct::Pipe, 1),
            [b'^', ..] => (Punct::Caret, 1),
            [b'?', ..] => (Punct::Question, 1),
            [b'@', ..] => (Punct::At, 1),
            _ => {
                let ch = self.text[self.pos..].chars().next().unwrap();
                self.pos += ch.len_utf8();
                self.diagnostics.push(Diagnostic::error(
                    "E0007",
                    Span::new(start as u32, self.pos as u32),
                    format!("unexpected character `{ch}`"),
                ));
                return;
            }
        };
        self.pos += len;
        self.push(TokenKind::Punct(punct), start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        let l = lex(src);
        assert!(l.diagnostics.is_empty(), "{:?}", l.diagnostics);
        l.tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn idents_keywords_and_contextual_words() {
        let k = kinds("component if size _ as");
        assert_eq!(k[0], TokenKind::Ident("component".into()));
        assert_eq!(k[1], TokenKind::Keyword(Keyword::If));
        assert_eq!(k[2], TokenKind::Ident("size".into()));
        assert_eq!(k[3], TokenKind::Punct(Punct::Underscore));
        assert_eq!(k[4], TokenKind::Keyword(Keyword::As));
        assert_eq!(k[5], TokenKind::Eof);
    }

    #[test]
    fn numbers_with_suffixes_and_bases() {
        let k = kinds("42 0xFF_u8 0b1010_0101 0o17 1_u1 7u16 33.333 14.318_18");
        assert_eq!(
            k[0],
            TokenKind::Int(IntLit {
                value: 42,
                suffix: None
            })
        );
        assert_eq!(
            k[1],
            TokenKind::Int(IntLit {
                value: 0xFF,
                suffix: Some(('u', 8))
            })
        );
        assert_eq!(
            k[2],
            TokenKind::Int(IntLit {
                value: 0b1010_0101,
                suffix: None
            })
        );
        assert_eq!(
            k[3],
            TokenKind::Int(IntLit {
                value: 0o17,
                suffix: None
            })
        );
        assert_eq!(
            k[4],
            TokenKind::Int(IntLit {
                value: 1,
                suffix: Some(('u', 1))
            })
        );
        assert_eq!(
            k[5],
            TokenKind::Int(IntLit {
                value: 7,
                suffix: Some(('u', 16))
            })
        );
        assert_eq!(k[6], TokenKind::Float(33.333));
        assert_eq!(k[7], TokenKind::Float(14.31818));
    }

    #[test]
    fn range_after_number_is_not_a_float() {
        let k = kinds("0x40..=0x43 0..6");
        assert_eq!(k[1], TokenKind::Punct(Punct::DotDotEq));
        assert_eq!(k[4], TokenKind::Punct(Punct::DotDot));
    }

    #[test]
    fn punctuation_longest_match() {
        let k = kinds("<- -> => ..= .. :: << >> <= >= == != && || < >");
        let expect = [
            Punct::LArrow,
            Punct::RArrow,
            Punct::FatArrow,
            Punct::DotDotEq,
            Punct::DotDot,
            Punct::ColonColon,
            Punct::Shl,
            Punct::Shr,
            Punct::Le,
            Punct::Ge,
            Punct::EqEq,
            Punct::Ne,
            Punct::AmpAmp,
            Punct::PipePipe,
            Punct::Lt,
            Punct::Gt,
        ];
        for (i, p) in expect.iter().enumerate() {
            assert_eq!(k[i], TokenKind::Punct(*p), "token {i}");
        }
    }

    #[test]
    fn comments_and_doc_comments() {
        let k = kinds("// plain\n/// Doc line\n/* block */ x //// not doc\n");
        assert_eq!(k[0], TokenKind::DocComment("Doc line".into()));
        assert_eq!(k[1], TokenKind::Ident("x".into()));
        assert_eq!(k[2], TokenKind::Eof);
    }

    #[test]
    fn strings_with_escapes() {
        let k = kinds(r#""a\"b\\c\n""#);
        assert_eq!(k[0], TokenKind::Str("a\"b\\c\n".into()));
    }

    #[test]
    fn reports_errors_and_continues() {
        let l = lex("x $ y \"open");
        assert_eq!(l.diagnostics.len(), 2);
        assert_eq!(l.tokens.len(), 4); // x y "open" Eof
    }
}

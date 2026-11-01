//! Tokens.
//!
//! Machine has few hard keywords. Most words that look like keywords
//! (`component`, `rtl`, `register`, `size`, `mode`, ...) are *contextual*:
//! the lexer produces an [`TokenKind::Ident`] and the parser recognises the
//! word where the grammar expects it. That keeps `size`, `data`, `mode`,
//! `clock` and friends usable as ordinary names, which hardware descriptions
//! need constantly. The hard keywords are those that start or shape
//! expressions and statements, where a contextual reading would be
//! ambiguous.

use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Keyword {
    If,
    Else,
    Match,
    Let,
    For,
    In,
    Loop,
    While,
    Break,
    Continue,
    Return,
    As,
    True,
    False,
    Const,
    Use,
}

impl Keyword {
    pub fn from_word(s: &str) -> Option<Keyword> {
        Some(match s {
            "if" => Keyword::If,
            "else" => Keyword::Else,
            "match" => Keyword::Match,
            "let" => Keyword::Let,
            "for" => Keyword::For,
            "in" => Keyword::In,
            "loop" => Keyword::Loop,
            "while" => Keyword::While,
            "break" => Keyword::Break,
            "continue" => Keyword::Continue,
            "return" => Keyword::Return,
            "as" => Keyword::As,
            "true" => Keyword::True,
            "false" => Keyword::False,
            "const" => Keyword::Const,
            "use" => Keyword::Use,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::Match => "match",
            Keyword::Let => "let",
            Keyword::For => "for",
            Keyword::In => "in",
            Keyword::Loop => "loop",
            Keyword::While => "while",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Return => "return",
            Keyword::As => "as",
            Keyword::True => "true",
            Keyword::False => "false",
            Keyword::Const => "const",
            Keyword::Use => "use",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Punct {
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Lt,
    Gt,
    Comma,
    Semi,
    Colon,
    ColonColon,
    Dot,
    DotDot,
    DotDotEq,
    Eq,
    LArrow,   // <-
    RArrow,   // ->
    FatArrow, // =>
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Tilde,
    Amp,
    Pipe,
    Caret,
    AmpAmp,
    PipePipe,
    Shl,
    Shr,
    EqEq,
    Ne,
    Le,
    Ge,
    Question,
    At,
    Underscore,
}

impl Punct {
    pub fn as_str(self) -> &'static str {
        match self {
            Punct::LBrace => "{",
            Punct::RBrace => "}",
            Punct::LParen => "(",
            Punct::RParen => ")",
            Punct::LBracket => "[",
            Punct::RBracket => "]",
            Punct::Lt => "<",
            Punct::Gt => ">",
            Punct::Comma => ",",
            Punct::Semi => ";",
            Punct::Colon => ":",
            Punct::ColonColon => "::",
            Punct::Dot => ".",
            Punct::DotDot => "..",
            Punct::DotDotEq => "..=",
            Punct::Eq => "=",
            Punct::LArrow => "<-",
            Punct::RArrow => "->",
            Punct::FatArrow => "=>",
            Punct::Plus => "+",
            Punct::Minus => "-",
            Punct::Star => "*",
            Punct::Slash => "/",
            Punct::Percent => "%",
            Punct::Bang => "!",
            Punct::Tilde => "~",
            Punct::Amp => "&",
            Punct::Pipe => "|",
            Punct::Caret => "^",
            Punct::AmpAmp => "&&",
            Punct::PipePipe => "||",
            Punct::Shl => "<<",
            Punct::Shr => ">>",
            Punct::EqEq => "==",
            Punct::Ne => "!=",
            Punct::Le => "<=",
            Punct::Ge => ">=",
            Punct::Question => "?",
            Punct::At => "@",
            Punct::Underscore => "_",
        }
    }
}

/// An integer literal. The value is kept as `u128`; widths above 128 bits
/// are not representable as literals (the type system caps widths at 128).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntLit {
    pub value: u128,
    /// Explicit width suffix (`0xFF_u8` → `Some(('u', 8))`).
    pub suffix: Option<(char, u32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Keyword(Keyword),
    Punct(Punct),
    Int(IntLit),
    /// A decimal number with a fractional part. Only meaningful before a
    /// unit (`33.333 MHz`); the parser rejects it elsewhere.
    Float(f64),
    Str(String),
    /// A `///` doc comment, with the leading `///` and one optional space
    /// removed. Attached by the parser to the following item.
    DocComment(String),
    Eof,
}

impl TokenKind {
    /// Short description for diagnostics ("`{`", "identifier", ...).
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(s) => format!("identifier `{s}`"),
            TokenKind::Keyword(k) => format!("keyword `{}`", k.as_str()),
            TokenKind::Punct(p) => format!("`{}`", p.as_str()),
            TokenKind::Int(_) => "integer literal".to_string(),
            TokenKind::Float(_) => "number".to_string(),
            TokenKind::Str(_) => "string literal".to_string(),
            TokenKind::DocComment(_) => "doc comment".to_string(),
            TokenKind::Eof => "end of file".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

//! Diagnostics: errors and warnings with stable codes, rendered as text for
//! people and as JSON lines for tools.
//!
//! Codes are of the form `E0001` (error) / `W0001` (warning). They are
//! allocated in blocks per phase so that a code never changes meaning:
//!
//! - `E00xx` lexical
//! - `E01xx` syntactic
//! - `E1xxx` semantic (reserved for `machine-sema`)

use std::fmt;

use crate::span::{SourceFile, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A secondary location attached to a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(code: &'static str, span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Warning,
            ..Diagnostic::error(code, span, message)
        }
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.labels.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Diagnostic {
        self.notes.push(note.into());
        self
    }

    /// Human-readable rendering, with the offending source line and a caret.
    pub fn render_text(&self, file: &SourceFile) -> String {
        let mut out = String::new();
        let lc = file.line_col(self.span.start);
        out.push_str(&format!(
            "{}[{}]: {}\n  --> {}:{}:{}\n",
            self.severity.as_str(),
            self.code,
            self.message,
            file.name,
            lc.line,
            lc.col
        ));
        let line = file.line_text(lc.line);
        let gutter = format!("{}", lc.line);
        out.push_str(&format!("{:>w$} |\n", "", w = gutter.len()));
        out.push_str(&format!("{} | {}\n", gutter, line));
        let end = if file.line_col(self.span.end).line == lc.line {
            file.line_col(self.span.end).col
        } else {
            line.chars().count() as u32 + 1
        };
        let width = end.saturating_sub(lc.col).max(1) as usize;
        out.push_str(&format!(
            "{:>w$} | {}{}\n",
            "",
            " ".repeat(lc.col as usize - 1),
            "^".repeat(width),
            w = gutter.len()
        ));
        for label in &self.labels {
            let llc = file.line_col(label.span.start);
            out.push_str(&format!(
                "{:>w$} = {}:{}: {}\n",
                "",
                llc.line,
                llc.col,
                label.message,
                w = gutter.len()
            ));
        }
        for note in &self.notes {
            out.push_str(&format!("{:>w$} = note: {}\n", "", note, w = gutter.len()));
        }
        out
    }

    /// One JSON object per diagnostic, on one line. Hand-rolled so the syntax
    /// crate has no dependencies; the shape is stable:
    /// `{"severity","code","message","file","line","col","end_line","end_col","labels":[...],"notes":[...]}`.
    pub fn render_json(&self, file: &SourceFile) -> String {
        let start = file.line_col(self.span.start);
        let end = file.line_col(self.span.end);
        let labels: Vec<String> = self
            .labels
            .iter()
            .map(|l| {
                let lc = file.line_col(l.span.start);
                format!(
                    "{{\"line\":{},\"col\":{},\"message\":{}}}",
                    lc.line,
                    lc.col,
                    json_string(&l.message)
                )
            })
            .collect();
        let notes: Vec<String> = self.notes.iter().map(|n| json_string(n)).collect();
        format!(
            "{{\"severity\":{},\"code\":{},\"message\":{},\"file\":{},\"line\":{},\"col\":{},\"end_line\":{},\"end_col\":{},\"labels\":[{}],\"notes\":[{}]}}",
            json_string(self.severity.as_str()),
            json_string(self.code),
            json_string(&self.message),
            json_string(&file.name),
            start.line,
            start.col,
            end.line,
            end.col,
            labels.join(","),
            notes.join(",")
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]: {} at {:?}",
            self.severity.as_str(),
            self.code,
            self.message,
            self.span
        )
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A sink for diagnostics produced by one phase over one file.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Diagnostics {
        Diagnostics::default()
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    /// Drops everything reported after the first `len` diagnostics (for a
    /// checker that tries one reading of a construct and then another).
    pub fn truncate(&mut self, len: usize) {
        self.items.truncate(len);
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_text_with_caret() {
        let file = SourceFile::new("t.machine", "enum A { X }\nbundle B { }\n");
        let d = Diagnostic::error("E0100", Span::new(13, 19), "bad bundle");
        let text = d.render_text(&file);
        assert!(text.contains("error[E0100]: bad bundle"));
        assert!(text.contains("t.machine:2:1"));
        assert!(text.contains("2 | bundle B { }"));
        assert!(text.contains("^^^^^^"));
    }

    #[test]
    fn renders_json_escaped() {
        let file = SourceFile::new("t.machine", "x\n");
        let d = Diagnostic::error("E0001", Span::new(0, 1), "quote \" here").with_note("n");
        let json = d.render_json(&file);
        assert!(json.starts_with('{'));
        assert!(json.contains("\"message\":\"quote \\\" here\""));
        assert!(json.contains("\"notes\":[\"n\"]"));
    }
}

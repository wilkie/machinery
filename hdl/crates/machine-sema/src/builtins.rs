//! Built-in functions callable from any body.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// `parity(x: u<N>) -> u1`: XOR of all bits (1 when odd).
    Parity,
    /// `popcount(x: u<N>) -> u<ceil(log2(N+1))>`
    Popcount,
    /// `clz(x: u<N>) -> u<ceil(log2(N+1))>`
    Clz,
    /// `ctz(x: u<N>) -> u<ceil(log2(N+1))>`
    Ctz,
    /// `concat(a, b, ...) -> u<sum of widths>`, first argument most significant.
    Concat,
    /// `replicate(x: u<N>, n) -> u<N*n>`
    Replicate,
    /// `assert(cond: bool, msg?: str)`
    Assert,
    /// `unreachable(msg?: str) -> !`
    Unreachable,
    /// `sync(x)`: cross-clock-domain read (rtl only; typed as identity).
    Sync,
    /// `restarted() -> bool` in a timing expression: this step starts at the
    /// address the previous one did (a repeated string instruction's later
    /// iterations).
    Restarted,
    /// `schedule(w0, step, w1, ..., decode: n) -> Schedule` in a timing
    /// expression (SPEC 11.2c).
    Schedule,
}

impl Builtin {
    pub fn from_name(name: &str) -> Option<Builtin> {
        Some(match name {
            "parity" => Builtin::Parity,
            "popcount" => Builtin::Popcount,
            "clz" => Builtin::Clz,
            "ctz" => Builtin::Ctz,
            "concat" => Builtin::Concat,
            "replicate" => Builtin::Replicate,
            "assert" => Builtin::Assert,
            "unreachable" => Builtin::Unreachable,
            "sync" => Builtin::Sync,
            "restarted" => Builtin::Restarted,
            "schedule" => Builtin::Schedule,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Parity => "parity",
            Builtin::Popcount => "popcount",
            Builtin::Clz => "clz",
            Builtin::Ctz => "ctz",
            Builtin::Concat => "concat",
            Builtin::Replicate => "replicate",
            Builtin::Assert => "assert",
            Builtin::Unreachable => "unreachable",
            Builtin::Sync => "sync",
            Builtin::Restarted => "restarted",
            Builtin::Schedule => "schedule",
        }
    }
}

/// Bits needed to hold values `0..=n`.
pub fn bits_for(n: u64) -> u32 {
    if n == 0 {
        1
    } else {
        64 - n.leading_zeros()
    }
}

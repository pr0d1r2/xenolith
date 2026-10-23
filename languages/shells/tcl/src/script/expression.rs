//! The substitutions an expression runs (`languages/shells/tcl` §I): what
//! `expr`, and an `if` or `while` condition, hands Tcl's expression
//! parser. Its operators are not Tcl words, so the body is not a script:
//! only its `$var` and `[cmd]` matter, at the top or inside a quoted
//! operand, while a braced operand is a literal string.

use xenolith_lang_api::{Error, LangId, Result, Span};

use super::{Part, Scan, place};

#[cfg(test)]
mod tests;

/// The substitutions of `body` read as an expression (`expr`, an `if` or
/// `while` condition): `$var` and `[cmd]`, at the top or inside a quoted
/// operand. A braced operand is a literal string and holds none.
///
/// # Errors
///
/// [`Error::Parse`] when a `[...]` inside does not parse, or the
/// expression ends inside a quote, bracket or brace.
pub fn expression(src: &str, body: Span) -> Result<Vec<Part>> {
    let mut scan = Scan::new(src, body);
    let mut parts = Vec::new();
    while let Some(b) = scan.peek() {
        match b {
            b'\\' => scan.escape(),
            b'{' => {
                if !scan.skip_braced() {
                    return Err(unclosed(src, scan.pos, "missing close-brace"));
                }
            }
            b'"' => {
                let open = scan.pos;
                scan.pos += 1;
                let quoted = scan.pieces(|s| s.peek() == Some(b'"'), false);
                if scan.peek() != Some(b'"') {
                    return Err(unclosed(src, open, "missing \""));
                }
                scan.pos += 1;
                parts.extend(quoted.into_iter().filter(substitutes));
            }
            b'$' | b'[' => match scan.substitution() {
                Some(part) => parts.push(part),
                // A `$` that names nothing is an operator's text.
                None => scan.pos += 1,
            },
            _ => scan.pos += 1,
        }
    }
    Ok(parts)
}

/// Whether a part is a substitution rather than literal bytes.
fn substitutes(part: &Part) -> bool {
    matches!(part, Part::Var { .. } | Part::Cmd { .. })
}

/// An error for an opener at byte `at` of `src` that never closes.
fn unclosed(src: &str, at: usize, message: &str) -> Error {
    let (line, col) = place(src, at, 1, 1);
    Error::parse(LangId::Tcl, format!("{line}:{col}: {message}"))
}

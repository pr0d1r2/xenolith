//! The ROUND TRIP: the laws that let extract and inline be one lens read
//! in two directions (`languages/api/src/lens:V34`).
//!
//! Each law is a pure function over a host and a fixture, returning the
//! break it found rather than panicking, so a language crate's harness
//! can name the fixture beside the law (`languages/api/src/lens` §I).

use crate::{Delim, Host};

/// The escape law (`languages/api/src/lens:V39`) over one fixture: `raw`
/// is text as it sits between the delimiters of `delim` in a host file.
///
/// With `body = unescape(raw)`, `unescape(escape(body))` must be `body`,
/// byte for byte. The law starts from a raw body rather than from any
/// string, because only a body the host could have produced has to
/// survive the trip: nix strips a common indent, so a body that still
/// has one never came out of a `''` string.
///
/// Every `Err` along the way is a break, [`crate::Error::Unsupported`]
/// included: a host missing `escape` fails the law instead of passing it
/// vacuously (`languages/api:V37`).
///
/// # Errors
///
/// A message naming the law, the step that broke and the body.
pub fn escape_law<H: Host + ?Sized>(
    host: &H,
    delim: &Delim,
    raw: &str,
) -> std::result::Result<(), String> {
    const LAW: &str = "languages/api/src/lens:V39";
    let body = host
        .unescape(delim, raw)
        .map_err(|e| format!("{LAW}: unescape of the fixture {raw:?} failed: {e}"))?;
    let escaped = host
        .escape(delim, &body)
        .map_err(|e| format!("{LAW}: escape of {body:?} failed: {e}"))?;
    let back = host
        .unescape(delim, &escaped)
        .map_err(|e| format!("{LAW}: unescape of escape({body:?}) = {escaped:?} failed: {e}"))?;
    if back == body {
        Ok(())
    } else {
        Err(format!(
            "{LAW}: unescape(escape(b)) != b for b = {body:?}: \
             escape(b) = {escaped:?} reads back as {back:?}"
        ))
    }
}

#[cfg(test)]
mod tests;

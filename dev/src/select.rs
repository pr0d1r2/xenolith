//! Which generated outputs a change can have invalidated (`dev:V345`).
//! RED: signatures only.

#[cfg(test)]
mod tests;

/// One generated output and the files it is rendered from.
#[derive(Debug)]
pub struct Generated {
    /// The output's name.
    pub name: &'static str,
    /// Its inputs.
    pub inputs: &'static [&'static str],
}

/// Every output `xenolith-dev` maintains.
pub const OUTPUTS: &[Generated] = &[];

/// Does one changed path match one declared input?
#[must_use]
pub fn matches(_pattern: &str, _path: &str) -> bool {
    false
}

/// The outputs worth comparing, given what changed.
#[must_use]
pub fn selected(_changed: &[String]) -> Vec<&'static str> {
    Vec::new()
}

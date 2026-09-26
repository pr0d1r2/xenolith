//! The extract engine: `xnl extract` as a library call (`src/extract`
//! §G).
//!
//! * [`place`] -- where each extract goes, field by field, and which
//!   layer decided it (`src/extract:V45`).
//! * [`diff`] -- unified diffs of whole files.
//! * [`write`] -- `--write`.

pub mod diff;
pub mod place;
pub mod write;

#[cfg(test)]
mod tests;

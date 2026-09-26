//! The graph engine: `xnl graph` as a library call (`src/graph:V7`).
//!
//! Which host loads which extract, and the two ways that can be wrong:
//! a load pointing at nothing (`dangling-load`) and an extract nothing
//! loads (`orphan-extract`). The parts live in their own modules:
//!
//! * [`roots`] -- where extracts live, so the orphan scan walks those
//!   directories and never the whole repository (`src/graph:V50`).
//! * [`resolve`] -- the file a load names, found without following a
//!   symlink (`src/graph:V72`).

pub mod resolve;
pub mod roots;

#[cfg(test)]
mod tests;

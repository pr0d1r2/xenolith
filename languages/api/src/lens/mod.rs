//! The ROUND TRIP: the laws that let extract and inline be one lens read
//! in two directions (`languages/api/src/lens:V34`).
//!
//! Each law is a pure function over a host and a fixture, returning the
//! break it found rather than panicking, so a language crate's harness
//! can name the fixture beside the law (`languages/api/src/lens` §I).

#[cfg(test)]
mod tests;

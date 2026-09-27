//! XML support for xenolith.
//!
//! XML is a host. The case this crate exists for is a launchd job: a text
//! property list whose `ProgramArguments` array runs `sh -c <script>`
//! (`languages/data/xml:V189`), where the script is a shell program kept
//! in a `<string>` element, entity-escaped, and seen by no shellcheck.
//!
//! The grammar is `tree-sitter-xml` from crates.io
//! (`languages/data/xml:R187`). Element text, entity references and CDATA
//! sections are all grammar nodes, so decoding a body is a walk over the
//! tree, never a scan for `&` (`languages:C4`).
//!
//! The extract direction is refused until `languages/data/xml:T190`
//! decides where a launchd job's load may point: launchd runs a job from
//! `/` unless it says otherwise, so a path relative to the plist is wrong
//! by default.

pub mod host;
mod launchd;
pub mod text;

pub use crate::host::XmlHost;
pub use crate::text::{escape, unescape};

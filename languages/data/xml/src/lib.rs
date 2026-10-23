//! XML support for xenolith.
//!
//! XML is a host. The case this crate exists for is a launchd job: a text
//! property list whose `ProgramArguments` array runs `sh -c <script>`
//! (`languages/data/xml:V189`), where the script is a shell program kept
//! in a `<string>` element, entity-escaped, and seen by no shellcheck.
//!
//! The reader is `quick-xml`, pure Rust ([`doc`]): it reports element
//! text, entity references and CDATA sections as separate events with
//! their byte positions, so decoding a body is a walk over those pieces,
//! never a scan for `&` (`languages:C4`). No C is compiled.
//!
//! The extract direction is refused until `languages/data/xml:T190`
//! decides where a launchd job's load may point: launchd runs a job from
//! `/` unless it says otherwise, so a path relative to the plist is wrong
//! by default.

mod doc;
pub mod host;
mod launchd;
pub mod text;

pub use crate::host::XmlHost;
pub use crate::text::{escape, unescape};

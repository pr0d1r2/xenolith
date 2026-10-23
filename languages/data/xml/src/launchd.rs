//! The launchd sink (`languages/data/xml:V189`).
//!
//! A launchd job is a property list whose top dict names what to run. With
//! `ProgramArguments` and no `Program` key, argv[0] IS the executable, so
//! an array reading `<sh|bash|zsh|dash> -c <script> ...` runs `<script>` as
//! a shell program -- the argv form of `languages/shells/shell:V139`, and
//! the one shape this module turns into a site. Its env is read from ITS
//! argv: the dialect is argv[0]'s basename, and with `-c` alone as
//! argv[1] no option is set.
//!
//! Every layer comes from the element tree [`crate::doc`] reads, and
//! dropping any of them leaves inert
//! data (`languages:V2`): the same array below the top dict, in a
//! document whose root is not `<plist>`, beside a `Program` key, or under
//! any other key. A shape V189 leaves open -- a flag bundle such as
//! `-lc`, a repeated key -- is no site either: a site built on a guess
//! about what launchd runs is worse than none.

use xenolith_lang_api::{Delim, DelimKind, GuestEnv, LangId, Site, Span};

use crate::doc::{Document, Element};
use crate::text::{decode, is_text};

#[cfg(test)]
mod tests;

/// The shells whose `-c` argument is a script (`languages/data/xml:V189`),
/// by argv[0]'s basename.
const SHELLS: &[&str] = &["bash", "dash", "sh", "zsh"];

/// The sink a site reports: the plist key that holds it.
const SINK: &str = "ProgramArguments";

/// Whether `element` is named `name`, as written.
fn named(element: &Element, name: &str) -> bool {
    element.name == name
}

/// The decoded text of an element holding text only; `None` for markup
/// or an entity with no value. `<x/>` and `<x></x>` both read as empty.
fn string_value(element: &Element, src: &str) -> Option<String> {
    decode(&element.pieces, src).ok()
}

/// The top dict: the only element of a document whose root is `<plist>`.
fn top_dict(document: &Document) -> Option<&Element> {
    let plist = document.root.as_ref().filter(|root| named(root, "plist"))?;
    match plist.children.as_slice() {
        [dict] if named(dict, "dict") => Some(dict),
        _ => None,
    }
}

/// The dict's entries as `(key, value)`, or `None` when it is not a
/// strict alternation of `<key>` and value.
fn entries<'d>(dict: &'d Element, src: &str) -> Option<Vec<(String, &'d Element)>> {
    let mut out = Vec::new();
    for pair in dict.children.chunks(2) {
        let [key, value] = pair else {
            return None;
        };
        if !named(key, "key") {
            return None;
        }
        out.push((string_value(key, src)?, value));
    }
    Some(out)
}

/// The script element of a `ProgramArguments` array, and argv[0]'s
/// basename, when the array reads `<shell> -c <script> ...`.
fn script<'d>(array: &'d Element, src: &str) -> Option<(&'d Element, String)> {
    let argv = &array.children;
    if argv.iter().any(|arg| !named(arg, "string")) {
        return None;
    }
    let [program, flag, script, ..] = argv.as_slice() else {
        return None;
    };
    let program = string_value(program, src)?;
    let shell = program.rsplit('/').next().unwrap_or(&program);
    if !SHELLS.contains(&shell) || string_value(flag, src)? != "-c" {
        return None;
    }
    Some((script, shell.to_owned()))
}

/// The site of a script element: its start tag, its text and its end
/// tag. `<string/>` has no text to report, and markup inside is no argv
/// word.
fn site(script: &Element, dialect: String) -> Option<Site> {
    let close = script.end_tag?;
    if !is_text(&script.pieces) {
        return None;
    }
    Some(Site {
        sink: SINK.to_owned(),
        guest: LangId::Shell,
        env: GuestEnv {
            dialect: Some(dialect),
            options: Vec::new(),
        },
        delim: Delim {
            kind: DelimKind::ArgvString,
            open: script.start_tag,
            body: Span::new(script.start_tag.end, close.start),
            close,
        },
        holes: Vec::new(),
    })
}

/// The launchd site of `document`, if any: at most one, the top dict's
/// `ProgramArguments` script (`languages/data/xml:V189`).
pub(crate) fn sites(document: &Document, src: &str) -> Vec<Site> {
    let found = || {
        let entries = entries(top_dict(document)?, src)?;
        if entries.iter().any(|(key, _)| key == "Program") {
            return None;
        }
        let arrays: Vec<&Element> = entries
            .iter()
            .filter(|(key, _)| key == SINK)
            .map(|(_, value)| *value)
            .collect();
        let [array] = arrays.as_slice() else {
            return None;
        };
        if !named(array, "array") {
            return None;
        }
        let (script, dialect) = script(array, src)?;
        site(script, dialect)
    };
    found().into_iter().collect()
}

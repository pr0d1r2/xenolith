//! The nix sink matrix: which strings hold shell, and under what env
//! (`languages/nix` §I).
//!
//! A string is a site only when its SYNTACTIC CONTEXT says so
//! (`languages/api/src/site:V38`). `''…''` under `script` is shell; the
//! same bytes under `description` are prose, under `text` of
//! `environment.etc` a config file, under a `let` binding a value nobody
//! has run yet. So every decision here reads the rnix tree around the
//! string node and never the string's contents (`languages:V2`).
//!
//! Three shapes of context, one per way nixpkgs hands a body to bash:
//!
//! - an ATTRIBUTE in an attribute set: `script`, `preStart`, `postStart`,
//!   `shellHook`, `ExecStart*`, `*Phase`;
//! - a POSITIONAL argument of a builder: the text of `writeShellScript*`,
//!   the body of `runCommand*`;
//! - an attribute of a builder's ARGUMENT set: `text` of
//!   `writeShellApplication`, which is the only place `text` means shell.

use rnix::{SyntaxKind, SyntaxNode};
use xenolith_lang_api::GuestEnv;

/// What kind of sink a string sits in, which decides the env its body
/// runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sink {
    /// NixOS service `script`, `preStart`, `postStart`.
    ServiceScript,
    /// A systemd `ExecStart`, `ExecStartPre` or `ExecStartPost` line.
    ExecStart,
    /// A dev shell's `shellHook`.
    ShellHook,
    /// The text of `writeShellScript` / `writeShellScriptBin`.
    WriteShellScript,
    /// The `text` of `writeShellApplication`.
    WriteShellApplication,
    /// The body of `runCommand` and its variants, or a stdenv `*Phase`.
    Stdenv,
}

impl Sink {
    /// The dialect and options nix itself wraps this body in
    /// (`languages/shell:V82`: the host declares, the prelude
    /// reproduces).
    ///
    /// Each row is what nixpkgs or NixOS WRITES around the body, not a
    /// house style. Reporting fewer options than are in force makes an
    /// extract that keeps running after a failure; reporting more makes
    /// one that exits where the inline body carried on.
    pub(crate) fn env(self) -> GuestEnv {
        let bash = |options: &[&str]| GuestEnv {
            dialect: Some("bash".to_owned()),
            options: options.iter().map(|o| (*o).to_owned()).collect(),
        };
        match self {
            // NixOS `makeJobScript`: `#! ${pkgs.runtimeShell} -e`.
            Sink::ServiceScript => bash(&["errexit"]),
            // A systemd exec line is NOT shell grammar
            // (`languages/nix:V69`); systemd splits it itself. Nothing is
            // established, and the classifier for it is `languages/nix:T71`.
            Sink::ExecStart => GuestEnv::default(),
            // `writeShellScript`: `#!${runtimeShell}` and nothing else.
            // `shellHook` is sourced into an interactive bash that
            // imposes no options on it either.
            Sink::ShellHook | Sink::WriteShellScript => bash(&[]),
            // `writeShellApplication`'s default `bashOptions`, and stdenv
            // `setup.sh`'s `set -eu` + `set -o pipefail`, which every
            // phase and every `runCommand` body runs under.
            Sink::WriteShellApplication | Sink::Stdenv => bash(&["errexit", "nounset", "pipefail"]),
        }
    }
}

/// The attribute names that are sinks wherever they appear in an
/// attribute set.
fn attr_sink(name: &str) -> Option<Sink> {
    // `ExecStart`, `ExecStartPre`, `ExecStartPost` -- a closed suffix set,
    // because those are the exec lines systemd has; `ExecStartFoo` is not
    // one of them and is nobody's shell.
    if let Some(suffix) = name.strip_prefix("ExecStart") {
        return matches!(suffix, "" | "Pre" | "Post").then_some(Sink::ExecStart);
    }
    match name {
        "script" | "preStart" | "postStart" => Some(Sink::ServiceScript),
        "shellHook" => Some(Sink::ShellHook),
        // `buildPhase`, `installPhase`, `checkPhase`, ... -- stdenv runs
        // every `*Phase` string through `eval`. A bare `Phase` is not a
        // phase name.
        _ if name
            .strip_suffix("Phase")
            .is_some_and(|stem| !stem.is_empty()) =>
        {
            Some(Sink::Stdenv)
        }
        _ => None,
    }
}

/// The builders whose POSITIONAL argument is a shell body, and which
/// argument it is (1-based, counting from the builder).
fn call_sink(callee: &str) -> Option<(Sink, usize)> {
    // `writeShellScript name text`, `writeShellScriptBin name text`.
    if callee == "writeShellScript" || callee == "writeShellScriptBin" {
        return Some((Sink::WriteShellScript, 2));
    }
    // `runCommand name env body`, and the variants with the same
    // signature: `runCommandLocal`, `runCommandCC`, `runCommandNoCC`.
    if callee.starts_with("runCommand") {
        return Some((Sink::Stdenv, 3));
    }
    None
}

/// The builder whose ARGUMENT SET holds a shell body under `text`.
const TEXT_BUILDER: &str = "writeShellApplication";

/// Every builder name that contributes a segment to a sink path.
fn is_builder(callee: &str) -> bool {
    callee == TEXT_BUILDER || call_sink(callee).is_some()
}

/// The sink a string node sits in, if any.
pub(crate) fn classify(string: &SyntaxNode) -> Option<Sink> {
    let parent = string.parent()?;
    match parent.kind() {
        SyntaxKind::NODE_ATTRPATH_VALUE => attr_value_sink(&parent),
        // `ExecStartPre = [ "a" "b" ]`: systemd takes a list of lines, and
        // each element is its own exec line.
        SyntaxKind::NODE_LIST => {
            let binding = parent.parent()?;
            if binding.kind() != SyntaxKind::NODE_ATTRPATH_VALUE {
                return None;
            }
            attr_value_sink(&binding).filter(|s| *s == Sink::ExecStart)
        }
        SyntaxKind::NODE_APPLY => {
            // The string must be the ARGUMENT, not the function position.
            if parent.first_child().as_ref() == Some(string) {
                return None;
            }
            let (callee, position) = apply_chain(&parent)?;
            let (sink, wanted) = call_sink(&callee)?;
            (position == wanted).then_some(sink)
        }
        _ => None,
    }
}

/// The sink for the value of `binding`, when `binding` lives in an
/// attribute set.
///
/// A `let` binding named `script` is a variable, not a sink: nothing runs
/// it until it is USED somewhere, and that use is where the site is.
fn attr_value_sink(binding: &SyntaxNode) -> Option<Sink> {
    let set = binding.parent()?;
    if set.kind() != SyntaxKind::NODE_ATTR_SET {
        return None;
    }
    let name = attr_segments(binding).pop()?;
    if let Some(sink) = attr_sink(&name) {
        return Some(sink);
    }
    // `text` is a sink ONLY as `writeShellApplication { text = …; }`.
    // `environment.etc."x".text` is a config file, and flagging it would
    // be the confident wrong answer `languages:V2` forbids.
    if name == "text" {
        let apply = set.parent()?;
        if apply.kind() == SyntaxKind::NODE_APPLY && apply.first_child().as_ref() != Some(&set) {
            let (callee, position) = apply_chain(&apply)?;
            return (callee == TEXT_BUILDER && position == 1)
                .then_some(Sink::WriteShellApplication);
        }
    }
    None
}

/// For an application node, the name of the function at the bottom of
/// its curried chain and how many arguments deep this node is.
///
/// `writeShellScript "n" body` parses as `Apply(Apply(f, "n"), body)`;
/// for the outer node this returns `("writeShellScript", 2)`.
fn apply_chain(apply: &SyntaxNode) -> Option<(String, usize)> {
    let mut depth = 1;
    let mut function = apply.first_child()?;
    while function.kind() == SyntaxKind::NODE_APPLY {
        depth += 1;
        function = function.first_child()?;
    }
    Some((callee_name(&function)?, depth))
}

/// The name a function expression calls by: `writeShellScript` for both
/// `pkgs.writeShellScript` and a bare `writeShellScript` under `with`.
fn callee_name(function: &SyntaxNode) -> Option<String> {
    match function.kind() {
        SyntaxKind::NODE_IDENT => Some(function.text().to_string()),
        SyntaxKind::NODE_SELECT => {
            let path = function
                .children()
                .find(|c| c.kind() == SyntaxKind::NODE_ATTRPATH)?;
            path.children().last().map(|attr| attr_name(&attr))
        }
        SyntaxKind::NODE_PAREN => callee_name(&function.first_child()?),
        _ => None,
    }
}

/// The dotted name a report points a reader back to: every enclosing
/// binding's attribute path, with each builder a string passes through
/// named as a segment of its own.
///
/// `systemd.services.foo.script`, `packages.hello.writeShellApplication.text`,
/// `p.writeShellScript`. Deterministic and from syntax only
/// (`languages/api/src/site:V43`), which is what placement will cut its
/// name from (`languages/nix:V53`).
pub(crate) fn sink_path(string: &SyntaxNode) -> String {
    let mut reversed: Vec<String> = Vec::new();
    for node in string.ancestors().skip(1) {
        match node.kind() {
            SyntaxKind::NODE_ATTRPATH_VALUE => {
                reversed.extend(attr_segments(&node).into_iter().rev());
            }
            // Only the OUTERMOST application of a curried chain names the
            // builder: its inner applications are its own function
            // position, and counting them would name it twice.
            SyntaxKind::NODE_APPLY if !is_function_of_parent_apply(&node) => {
                if let Some((callee, _)) = apply_chain(&node)
                    && is_builder(&callee)
                {
                    reversed.push(callee);
                }
            }
            _ => {}
        }
    }
    reversed.reverse();
    reversed.join(".")
}

/// Whether `node` is the function half of an enclosing application.
fn is_function_of_parent_apply(node: &SyntaxNode) -> bool {
    node.parent().is_some_and(|p| {
        p.kind() == SyntaxKind::NODE_APPLY && p.first_child().as_ref() == Some(node)
    })
}

/// The attribute names of a binding's path, `a.b.c` → `[a, b, c]`.
fn attr_segments(binding: &SyntaxNode) -> Vec<String> {
    binding
        .children()
        .find(|c| c.kind() == SyntaxKind::NODE_ATTRPATH)
        .map(|path| path.children().map(|attr| attr_name(&attr)).collect())
        .unwrap_or_default()
}

/// One attribute's name as a reader would write it: `foo` for `foo` and
/// for `"foo"`, and the source text for anything dynamic (`${x}`), which
/// has no static name to give.
fn attr_name(attr: &SyntaxNode) -> String {
    if attr.kind() == SyntaxKind::NODE_STRING
        && !attr
            .children()
            .any(|c| c.kind() == SyntaxKind::NODE_INTERPOL)
    {
        return attr
            .children_with_tokens()
            .filter(|t| t.kind() == SyntaxKind::TOKEN_STRING_CONTENT)
            .map(|t| t.to_string())
            .collect();
    }
    attr.text().to_string()
}

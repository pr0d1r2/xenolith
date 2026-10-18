//! Which recipe lines cannot be merged into one script unchanged
//! (`languages/ci/just:V180`).
//!
//! just runs each line in a FRESH shell, so a line that changes shell
//! state -- `cd`, `export`, an assignment, `set` -- changes nothing a
//! later line sees; merged into one script it changes everything after
//! it. Those merges are a judgement (`Judgment`), never mechanical.
//!
//! This crate has no shell grammar (`languages/api:V32`: a language crate
//! depends on its own grammar only), so the question is answered by
//! WORDS: the line is cut into words and operators outside quotes, and a
//! word in command position is compared against a closed list. The cut
//! errs one way only -- a `cd` inside quotes of an odd shape, or in a
//! subshell, is still read as a `cd` -- so its mistakes are refusals,
//! never a merge that changes what runs.

#[cfg(test)]
mod tests;

/// Builtins whose effect outlives their line: working dir, variables,
/// options, traps, functions and aliases -- plus `exit`, `exec` and
/// `return`, which end a merged script where just would only end one
/// line's shell.
const STATEFUL: &[&str] = &[
    ".", "alias", "builtin", "cd", "declare", "emulate", "eval", "exec", "exit", "export",
    "function", "local", "popd", "pushd", "readonly", "return", "set", "setopt", "shopt", "source",
    "trap", "typeset", "ulimit", "umask", "unalias", "unset", "unsetopt",
];

/// Words after which the next word is a command again.
const LEADERS: &[&str] = &[
    "!", "do", "elif", "else", "if", "then", "time", "until", "while", "{",
];

/// One piece of a line, outside quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// A word, quotes and all.
    Word(String),
    /// `;`, `&&`, `||`, `|`, `&`, `(` or `)`.
    Op(&'static str),
}

/// `line` cut into words and operators. Quotes (`'...'`, `"..."`), a `\`
/// escape and a `$(...)` stay inside their word; a `#` starting a word
/// ends the line.
pub(crate) fn tokens(line: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut chars = line.chars().peekable();
    let flush = |word: &mut String, out: &mut Vec<Token>| {
        if !word.is_empty() {
            out.push(Token::Word(std::mem::take(word)));
        }
    };
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => flush(&mut word, &mut out),
            '#' if word.is_empty() => break,
            '\\' => {
                word.push(c);
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            '\'' | '"' | '`' => {
                word.push(c);
                let mut escaped = false;
                for inner in chars.by_ref() {
                    word.push(inner);
                    if c != '\'' && inner == '\\' && !escaped {
                        escaped = true;
                        continue;
                    }
                    if inner == c && !escaped {
                        break;
                    }
                    escaped = false;
                }
            }
            '$' if chars.peek() == Some(&'(') => {
                word.push(c);
                let mut depth = 0usize;
                for inner in chars.by_ref() {
                    word.push(inner);
                    match inner {
                        '(' => depth += 1,
                        ')' if depth <= 1 => break,
                        ')' => depth -= 1,
                        _ => {}
                    }
                }
            }
            ';' | '(' | ')' => {
                flush(&mut word, &mut out);
                out.push(Token::Op(match c {
                    ';' => ";",
                    '(' => "(",
                    _ => ")",
                }));
            }
            '&' | '|' => {
                flush(&mut word, &mut out);
                let doubled = chars.peek() == Some(&c);
                if doubled {
                    chars.next();
                }
                out.push(Token::Op(match (c, doubled) {
                    ('&', true) => "&&",
                    ('&', false) => "&",
                    ('|', true) => "||",
                    _ => "|",
                }));
            }
            _ => word.push(c),
        }
    }
    flush(&mut word, &mut out);
    out
}

/// `NAME=...` or `NAME+=...`: an assignment word.
fn assignment(word: &str) -> bool {
    let Some(eq) = word.find('=') else {
        return false;
    };
    let name = word.get(..eq).unwrap_or_default();
    let name = name.strip_suffix('+').unwrap_or(name);
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `line` changes shell state a later line would read, or ends
/// the shell: a [`STATEFUL`] builtin in command position, a command made
/// of assignments only, or a function definition.
pub(crate) fn changes_state(line: &str) -> bool {
    let tokens = tokens(line);
    let mut at_command = true;
    let mut only_assignments = false;
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Op(op) => {
                if only_assignments {
                    return true;
                }
                if *op == "(" && tokens.get(i + 1) == Some(&Token::Op(")")) {
                    return true;
                }
                at_command = *op != ")";
                only_assignments = false;
            }
            Token::Word(word) if at_command => {
                if assignment(word) {
                    only_assignments = true;
                } else if LEADERS.contains(&word.as_str()) {
                    only_assignments = false;
                } else {
                    if STATEFUL.contains(&word.as_str()) {
                        return true;
                    }
                    at_command = false;
                    only_assignments = false;
                }
            }
            Token::Word(_) => {}
        }
    }
    only_assignments
}

/// Whether `line` holds `a; b`. Without `errexit` in the line's own
/// shell just ignores `a` failing and stops by `b` alone, while a merged
/// script under `set -e` stops at `a`.
pub(crate) fn sequences(line: &str) -> bool {
    tokens(line).contains(&Token::Op(";"))
}

/// Whether `line` fails in a way `set -e` does not stop at: an `a && b`
/// list or a `! a` pipeline. just stops after such a line when it fails;
/// a merged script runs on (bash: errexit ignores every `&&` element but
/// the last, and every `!`), unless the line is the script's last.
pub(crate) fn escapes_errexit(line: &str) -> bool {
    let tokens = tokens(line);
    let mut at_command = true;
    for token in &tokens {
        match token {
            Token::Op("&&") => return true,
            Token::Op(op) => at_command = *op != ")",
            Token::Word(word) if at_command && word == "!" => return true,
            Token::Word(word) if at_command => {
                at_command = LEADERS.contains(&word.as_str()) || assignment(word);
            }
            Token::Word(_) => {}
        }
    }
    false
}

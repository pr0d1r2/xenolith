//! Unified diffs of whole files, for `xnl extract` without `--write`
//! (`src/extract:C15`).
//!
//! Written here rather than taken from a crate (`src:C5`): the shape is
//! small -- a line-level longest common subsequence, then hunks with
//! three lines of context -- and the output must be byte-stable across
//! runs and platforms (`src:V11`), which a hand-rolled algorithm with no
//! heuristics guarantees by construction.

#[cfg(test)]
mod tests;

/// Lines of context around each change, as `diff -u` gives.
const CONTEXT: usize = 3;

/// Above this many cells the LCS table is not built and the differing
/// middle is shown as one removal and one addition: still a correct
/// diff, only a less minimal one, and memory stays bounded.
const MAX_CELLS: usize = 16_000_000;

/// One line of the diff body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op<'a> {
    Same(&'a str),
    Del(&'a str),
    Add(&'a str),
}

/// The unified diff turning `old` into `new`, headed `--- {from}` and
/// `+++ {to}`; empty when the texts are equal.
#[must_use]
pub fn unified(from: &str, to: &str, old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }
    let old_lines: Vec<&str> = old.split_inclusive('\n').collect();
    let new_lines: Vec<&str> = new.split_inclusive('\n').collect();
    let ops = script(&old_lines, &new_lines);
    let mut out = format!("--- {from}\n+++ {to}\n");
    for (start, end) in hunks(&ops) {
        let slice = ops.get(start..end).unwrap_or_default();
        let (old_at, new_at) = position(&ops, start);
        let old_len = slice.iter().filter(|op| !matches!(op, Op::Add(_))).count();
        let new_len = slice.iter().filter(|op| !matches!(op, Op::Del(_))).count();
        out.push_str("@@ -");
        out.push_str(&range(old_at, old_len));
        out.push_str(" +");
        out.push_str(&range(new_at, new_len));
        out.push_str(" @@\n");
        for op in slice {
            let (mark, line) = match op {
                Op::Same(line) => (' ', line),
                Op::Del(line) => ('-', line),
                Op::Add(line) => ('+', line),
            };
            out.push(mark);
            out.push_str(line);
            if !line.ends_with('\n') {
                out.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    out
}

/// `start,len` as `diff -u` spells it: 1-based, the line BEFORE the
/// hunk when it is empty, and `,1` left out.
fn range(at: usize, len: usize) -> String {
    match len {
        0 => format!("{at},0"),
        1 => format!("{}", at + 1),
        _ => format!("{},{len}", at + 1),
    }
}

/// Lines of `old` and `new` before op `index`.
fn position(ops: &[Op<'_>], index: usize) -> (usize, usize) {
    ops.iter()
        .take(index)
        .fold((0, 0), |(old, new), op| match op {
            Op::Same(_) => (old + 1, new + 1),
            Op::Del(_) => (old + 1, new),
            Op::Add(_) => (old, new + 1),
        })
}

/// The op ranges each hunk covers: every change with up to
/// [`CONTEXT`] unchanged lines either side, changes closer than twice
/// that merged into one hunk.
fn hunks(ops: &[Op<'_>]) -> Vec<(usize, usize)> {
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| !matches!(op, Op::Same(_)))
        .map(|(i, _)| i)
        .collect();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for at in changed {
        let start = at.saturating_sub(CONTEXT);
        let end = (at + 1 + CONTEXT).min(ops.len());
        match out.last_mut() {
            Some((_, last)) if start <= *last => *last = end,
            _ => out.push((start, end)),
        }
    }
    out
}

/// The edit script from `a` to `b`: shared head and tail kept, the
/// middle aligned by longest common subsequence.
fn script<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Op<'a>> {
    let head = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let rest_a = a.get(head..).unwrap_or_default();
    let rest_b = b.get(head..).unwrap_or_default();
    let tail = rest_a
        .iter()
        .rev()
        .zip(rest_b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let mid_a = rest_a.get(..rest_a.len() - tail).unwrap_or_default();
    let mid_b = rest_b.get(..rest_b.len() - tail).unwrap_or_default();
    let mut ops: Vec<Op<'a>> = a.iter().take(head).map(|l| Op::Same(l)).collect();
    ops.extend(middle(mid_a, mid_b));
    ops.extend(rest_a.iter().skip(mid_a.len()).map(|l| Op::Same(l)));
    ops
}

/// The aligned middle: an LCS table when it is small enough, else every
/// old line removed and every new one added.
fn middle<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Op<'a>> {
    let width = new.len() + 1;
    if old.len().saturating_mul(width) > MAX_CELLS {
        let mut ops: Vec<Op<'a>> = old.iter().map(|l| Op::Del(l)).collect();
        ops.extend(new.iter().map(|l| Op::Add(l)));
        return ops;
    }
    // lcs[row * width + col] = LCS length of old[row..] and new[col..].
    let mut lcs = vec![0usize; (old.len() + 1) * width];
    for row in (0..old.len()).rev() {
        for col in (0..new.len()).rev() {
            let value = if old.get(row) == new.get(col) {
                lcs.get((row + 1) * width + col + 1).copied().unwrap_or(0) + 1
            } else {
                let down = lcs.get((row + 1) * width + col).copied().unwrap_or(0);
                let right = lcs.get(row * width + col + 1).copied().unwrap_or(0);
                down.max(right)
            };
            if let Some(cell) = lcs.get_mut(row * width + col) {
                *cell = value;
            }
        }
    }
    walk(old, new, |row, col| {
        lcs.get(row * width + col).copied().unwrap_or(0)
    })
}

/// Read the edit script off the LCS table `at`: a shared line is kept,
/// otherwise the side whose skip keeps the longer subsequence moves.
fn walk<'a>(old: &[&'a str], new: &[&'a str], at: impl Fn(usize, usize) -> usize) -> Vec<Op<'a>> {
    let mut ops = Vec::with_capacity(old.len() + new.len());
    let (mut row, mut col) = (0, 0);
    while row < old.len() || col < new.len() {
        match (old.get(row), new.get(col)) {
            (Some(gone), Some(came)) if gone == came => {
                ops.push(Op::Same(gone));
                row += 1;
                col += 1;
            }
            (Some(gone), Some(_)) if at(row + 1, col) >= at(row, col + 1) => {
                ops.push(Op::Del(gone));
                row += 1;
            }
            (Some(gone), None) => {
                ops.push(Op::Del(gone));
                row += 1;
            }
            (_, Some(came)) => {
                ops.push(Op::Add(came));
                col += 1;
            }
            (None, None) => break,
        }
    }
    ops
}

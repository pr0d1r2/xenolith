//! Reading what a tool found: its machine-readable output into
//! `{line, col, code, severity, message}` (`src/lint:V92`, `src/lint` §I
//! findings).
//!
//! A tool is read by the name its `LintCmd.format` gives, never by
//! sniffing the text: the same `[]` is a clean shellcheck run and
//! meaningless from anything else. Whatever is not a known shape answers
//! `None`, and the engine keeps the raw tail instead, so a finding is
//! never dropped because its tool changed format.

use serde_json::Value;
use xenolith_lang_api::Format;

#[cfg(test)]
mod tests;

/// One thing a tool found, where it found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// 1-based line; 0 when the tool named no place in the file.
    pub line: usize,
    /// 1-based column, counting characters.
    pub col: usize,
    /// The tool's rule id: `SC2086`, `F401`; empty when it gave none.
    pub code: String,
    /// The tool's own severity word.
    pub severity: String,
    /// What the tool said.
    pub message: String,
}

/// A parsed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The findings, sorted (line, col, code, message) (`src:V11`).
    pub findings: Vec<Finding>,
    /// Columns count a tab to the next stop of 8, as shellcheck's
    /// `--format=json` does; [`detab`] turns them into characters.
    pub tab_stops: bool,
}

/// Read `stdout` as `format` says, or `None`: not a machine-readable
/// format, a tool with no parser here, or text that is not its shape.
#[must_use]
pub fn parse(format: &Format, stdout: &str) -> Option<Parsed> {
    let (findings, tab_stops) = match format {
        Format::Sarif => (sarif(&json(stdout)?)?, false),
        Format::Json("shellcheck") => shellcheck(&json(stdout)?)?,
        Format::Json("ruff") => (ruff(&json(stdout)?)?, false),
        Format::Json("sqlfluff") => (sqlfluff(&json(stdout)?)?, false),
        Format::Json(_) | Format::Raw => return None,
    };
    let mut findings = findings;
    findings.sort_by(|a, b| {
        (a.line, a.col, &a.code, &a.message).cmp(&(b.line, b.col, &b.code, &b.message))
    });
    Some(Parsed {
        findings,
        tab_stops,
    })
}

/// Turn tab-stop columns into character columns against `text`, the
/// file the tool read. A finding past the file's end, or on a line with
/// no tab, is left as it is.
pub fn detab(findings: &mut [Finding], text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    for finding in findings {
        let Some(line) = finding.line.checked_sub(1).and_then(|i| lines.get(i)) else {
            continue;
        };
        if !line.contains('\t') {
            continue;
        }
        let mut display = 1;
        let mut chars = 0;
        for c in line.chars() {
            if display >= finding.col {
                break;
            }
            display = if c == '\t' {
                (display - 1) / 8 * 8 + 9
            } else {
                display + 1
            };
            chars += 1;
        }
        finding.col = chars + 1;
    }
}

fn json(stdout: &str) -> Option<Value> {
    serde_json::from_str(stdout).ok()
}

/// A field as a count: a non-negative integer.
fn count(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
}

/// A field as text, empty when absent or not a string.
fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// shellcheck `--format=json` (an array) or `json1` (`{comments}`),
/// and whether columns are tab stops -- json's are, json1's are not.
fn shellcheck(value: &Value) -> Option<(Vec<Finding>, bool)> {
    let (items, tab_stops) = match value {
        Value::Array(items) => (items, true),
        Value::Object(_) => (value.get("comments")?.as_array()?, false),
        _ => return None,
    };
    let findings = items
        .iter()
        .map(|item| {
            Some(Finding {
                line: count(item, "line")?,
                col: count(item, "column")?,
                code: format!("SC{}", item.get("code")?.as_u64()?),
                severity: text(item, "level"),
                message: text(item, "message"),
            })
        })
        .collect::<Option<Vec<Finding>>>()?;
    Some((findings, tab_stops))
}

/// ruff `--output-format json`: an array of diagnostics; ruff has no
/// severity, and every diagnostic fails the check.
fn ruff(value: &Value) -> Option<Vec<Finding>> {
    value
        .as_array()?
        .iter()
        .map(|item| {
            let at = item.get("location")?;
            Some(Finding {
                line: count(at, "row")?,
                col: count(at, "column")?,
                code: text(item, "code"),
                severity: "error".to_owned(),
                message: text(item, "message"),
            })
        })
        .collect()
}

/// sqlfluff `--format json`: per file, its violations, positions under
/// either the old (`line_no`) or new (`start_line_no`) spelling.
fn sqlfluff(value: &Value) -> Option<Vec<Finding>> {
    let mut findings = Vec::new();
    for file in value.as_array()? {
        for item in file.get("violations")?.as_array()? {
            let warning = item.get("warning").and_then(Value::as_bool) == Some(true);
            findings.push(Finding {
                line: count(item, "start_line_no").or_else(|| count(item, "line_no"))?,
                col: count(item, "start_line_pos").or_else(|| count(item, "line_pos"))?,
                code: text(item, "code"),
                severity: if warning { "warning" } else { "error" }.to_owned(),
                message: text(item, "description"),
            });
        }
    }
    Some(findings)
}

/// SARIF 2.1.0: every result of every run, placed at its first
/// location's region -- line 0 when it has none, column 1 when the
/// region names no column, `warning` when it names no level (SARIF's
/// own defaults).
fn sarif(value: &Value) -> Option<Vec<Finding>> {
    let mut findings = Vec::new();
    for run in value.get("runs")?.as_array()? {
        let Some(results) = run.get("results").and_then(Value::as_array) else {
            continue;
        };
        for result in results {
            let region = result.pointer("/locations/0/physicalLocation/region");
            let level = result.get("level").and_then(Value::as_str);
            findings.push(Finding {
                line: region.and_then(|r| count(r, "startLine")).unwrap_or(0),
                col: region.and_then(|r| count(r, "startColumn")).unwrap_or(1),
                code: text(result, "ruleId"),
                severity: level.unwrap_or("warning").to_owned(),
                message: result
                    .pointer("/message/text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
    }
    Some(findings)
}

//! Findings parsers: the mirror of `src/lint/findings.rs` (`src:C139`).
//!
//! Pinned: each tool's JSON read into `{line, col, code, severity,
//! message}` (`src/lint:V92`, `src/lint` §I findings), shellcheck's tab
//! stops undone, the order findings come out in, and everything that is
//! not a known shape answering `None` so the raw tail is kept.

use xenolith_lang_api::Format;

use super::{Finding, Parsed, detab, parse};

fn finding(line: usize, col: usize, code: &str, severity: &str, message: &str) -> Finding {
    Finding {
        line,
        col,
        code: code.to_owned(),
        severity: severity.to_owned(),
        message: message.to_owned(),
    }
}

fn parsed(format: &Format, stdout: &str) -> Parsed {
    parse(format, stdout).unwrap_or_else(|| panic!("not parsed: {stdout}"))
}

// ---------------------------------------------------------------------
// shellcheck
// ---------------------------------------------------------------------

const SHELLCHECK: &str = r#"[{"file":"a.sh","line":3,"endLine":3,"column":14,
"endColumn":18,"level":"info","code":2086,"message":"Double quote.","fix":null}]"#;

#[test]
fn shellcheck_json_is_read_with_its_tab_stops_flagged() {
    let got = parsed(&Format::Json("shellcheck"), SHELLCHECK);
    assert_eq!(
        got.findings,
        [finding(3, 14, "SC2086", "info", "Double quote.")]
    );
    assert!(
        got.tab_stops,
        "--format=json counts a tab as up to 8 columns"
    );
}

#[test]
fn shellcheck_json1_is_read_as_it_counts() {
    let json1 = format!(r#"{{"comments":{SHELLCHECK}}}"#);
    let got = parsed(&Format::Json("shellcheck"), &json1);
    assert_eq!(
        got.findings,
        [finding(3, 14, "SC2086", "info", "Double quote.")]
    );
    assert!(!got.tab_stops, "json1 counts a tab as one column");
}

#[test]
fn shellcheck_with_nothing_to_say_is_parsed_and_empty() {
    let got = parsed(&Format::Json("shellcheck"), "[]\n");
    assert!(got.findings.is_empty());
}

// ---------------------------------------------------------------------
// ruff
// ---------------------------------------------------------------------

#[test]
fn ruff_json_reads_location_and_code() {
    let stdout = r#"[
      {"code":"F401","message":"`os` imported but unused","filename":"a.py",
       "location":{"row":1,"column":8},"end_location":{"row":1,"column":10}},
      {"code":null,"message":"SyntaxError: bad","filename":"a.py",
       "location":{"row":2,"column":1},"end_location":{"row":2,"column":2}}
    ]"#;
    let got = parsed(&Format::Json("ruff"), stdout);
    assert_eq!(
        got.findings,
        [
            finding(1, 8, "F401", "error", "`os` imported but unused"),
            finding(2, 1, "", "error", "SyntaxError: bad"),
        ]
    );
    assert!(!got.tab_stops);
}

// ---------------------------------------------------------------------
// sqlfluff
// ---------------------------------------------------------------------

#[test]
fn sqlfluff_json_reads_both_position_spellings_and_the_warning_flag() {
    let stdout = r#"[{"filepath":"a.sql","violations":[
      {"start_line_no":4,"start_line_pos":2,"code":"LT01","description":"spacing",
       "name":"layout.spacing","warning":false},
      {"line_no":1,"line_pos":1,"code":"AM04","description":"star","warning":true}
    ]}]"#;
    let got = parsed(&Format::Json("sqlfluff"), stdout);
    assert_eq!(
        got.findings,
        [
            finding(1, 1, "AM04", "warning", "star"),
            finding(4, 2, "LT01", "error", "spacing"),
        ]
    );
}

// ---------------------------------------------------------------------
// SARIF
// ---------------------------------------------------------------------

#[test]
fn sarif_reads_every_result_of_every_run() {
    let stdout = r#"{"version":"2.1.0","runs":[
      {"results":[{"ruleId":"R1","level":"error","message":{"text":"one"},
        "locations":[{"physicalLocation":{"region":{"startLine":5,"startColumn":3}}}]}]},
      {"results":[{"ruleId":"R2","message":{"text":"two"},
        "locations":[{"physicalLocation":{"region":{"startLine":2}}}]},
                  {"ruleId":"R3","message":{"text":"nowhere"}}]}
    ]}"#;
    let got = parsed(&Format::Sarif, stdout);
    assert_eq!(
        got.findings,
        [
            finding(0, 1, "R3", "warning", "nowhere"),
            finding(2, 1, "R2", "warning", "two"),
            finding(5, 3, "R1", "error", "one"),
        ]
    );
}

// ---------------------------------------------------------------------
// what is not a finding list
// ---------------------------------------------------------------------

#[test]
fn raw_an_unknown_tool_and_garbage_are_not_parsed() {
    assert_eq!(parse(&Format::Raw, "[]"), None);
    assert_eq!(parse(&Format::Json("statix-next"), "[]"), None);
    assert_eq!(parse(&Format::Json("shellcheck"), "In a.sh line 3:"), None);
    assert_eq!(parse(&Format::Json("shellcheck"), ""), None);
    assert_eq!(parse(&Format::Json("ruff"), r#"{"not":"a list"}"#), None);
    assert_eq!(parse(&Format::Sarif, "[]"), None);
}

#[test]
fn a_finding_missing_its_line_is_unparseable_rather_than_dropped() {
    let stdout = r#"[{"level":"info","code":1,"message":"m"}]"#;
    assert_eq!(parse(&Format::Json("shellcheck"), stdout), None);
}

#[test]
fn findings_come_out_sorted_by_line_col_code_message() {
    let stdout = r#"[
      {"line":2,"column":1,"level":"info","code":2,"message":"b"},
      {"line":1,"column":5,"level":"info","code":9,"message":"z"},
      {"line":2,"column":1,"level":"info","code":1,"message":"a"}
    ]"#;
    let got = parsed(&Format::Json("shellcheck"), stdout);
    let codes: Vec<&str> = got.findings.iter().map(|f| f.code.as_str()).collect();
    assert_eq!(codes, ["SC9", "SC1", "SC2"]);
}

// ---------------------------------------------------------------------
// tab stops
// ---------------------------------------------------------------------

#[test]
fn detab_turns_a_tab_stop_column_into_a_character_column() {
    let text = "#!/bin/bash\n\tfoo=$1\n\techo $foo\n";
    let mut findings = vec![finding(3, 14, "SC2086", "info", "m")];
    detab(&mut findings, text);
    assert_eq!(findings.first().map(|f| f.col), Some(7));
}

#[test]
fn detab_leaves_a_line_without_tabs_and_a_line_past_the_end() {
    let text = "echo $a\n";
    let mut findings = vec![
        finding(1, 6, "SC2086", "info", "m"),
        finding(9, 4, "SC1000", "info", "m"),
    ];
    detab(&mut findings, text);
    let cols: Vec<usize> = findings.iter().map(|f| f.col).collect();
    assert_eq!(cols, [6, 4]);
}

#[test]
fn detab_counts_tabs_mid_line_to_the_next_stop() {
    // `a\tb`: `a` at 1, the tab runs 2..=8, `b` at 9 -- char 3.
    let mut findings = vec![finding(1, 9, "X", "info", "m")];
    detab(&mut findings, "a\tb\n");
    assert_eq!(findings.first().map(|f| f.col), Some(3));
}

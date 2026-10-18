//! `xenolith-tcl-syntax FILE...`: Tcl's `info complete` over files, the
//! tcl host's check (`languages/shells/tcl:V198`). A shim: the logic and
//! its tests live in the library's `syntax` module.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1);
    ExitCode::from(xenolith_lang_tcl::syntax::run(
        args,
        &mut std::io::stdout().lock(),
    ))
}

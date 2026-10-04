//! `xenolith-dev`: a shim over [`xenolith_dev::main`] (`src:C139`), so every
//! branch of the tool lives where a unit test can reach it.

fn main() -> std::process::ExitCode {
    xenolith_dev::main()
}

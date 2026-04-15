//! Binary entrypoint for `fsel`.

fn main() -> std::process::ExitCode {
    match fsel::run() {
        Ok(code) => code,
        Err(error) => {
            fsel::cleanup_after_error();
            eprintln!("{error:?}");
            std::process::ExitCode::from(1)
        }
    }
}

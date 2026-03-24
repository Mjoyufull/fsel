//! Binary entrypoint for `fsel`.

fn main() {
    if let Err(error) = fsel::run() {
        fsel::cleanup_after_error();
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}

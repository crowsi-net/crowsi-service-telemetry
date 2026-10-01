#![forbid(unsafe_code)]

mod cli;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Err(error) = cli::run(&args) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

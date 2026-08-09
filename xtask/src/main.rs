use std::env;

fn main() -> std::process::ExitCode {
    xtask::run_cli(env::args().skip(1).collect())
}

mod array;
mod cli;
mod demo;
mod operator;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}

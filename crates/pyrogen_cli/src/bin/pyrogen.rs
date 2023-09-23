use pyrogen_cli::print_message;
use std::process::ExitCode;

pub fn main() -> ExitCode {
    print_message();
    0.into()
}

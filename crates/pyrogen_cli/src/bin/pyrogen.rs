use pyrogen_cli::{parse_args, print_message};
use std::process::ExitCode;

pub fn main() -> ExitCode {
    print_message();
    parse_args();
    0.into()
}

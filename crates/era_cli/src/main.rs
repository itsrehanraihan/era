//! `era` command-line entry point.
//!
//! M0 stub: only reports version and which milestone each command arrives in.
//! No compiler functionality exists yet.

use std::process::ExitCode;

const USAGE: &str = "\
era — design-first native app toolchain (M0 preview)

USAGE:
    era <command>

COMMANDS:
    new <name>       Create a project            (planned: M1)
    check            Validate a project          (planned: M1-M3)
    preview          Preview screens             (planned: M4)
    build android    Build an Android debug APK  (planned: M5)
    --version        Print version
    --help           Print this help";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("--version" | "-V") => {
            println!("era {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(cmd @ ("new" | "check" | "preview" | "build")) => {
            eprintln!("error: `era {cmd}` is not implemented yet (see MILESTONES.md)");
            ExitCode::from(2)
        }
        Some(other) => {
            eprintln!("error: unknown command `{other}`\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

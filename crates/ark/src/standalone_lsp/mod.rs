//
// mod.rs
//
// Copyright (C) 2026 Posit Software, PBC. All rights reserved.
//
//

use std::io::Read;

use anyhow::anyhow;

mod sidecar;

/// Parsed options for the `ark lsp` subcommand.
pub struct Options {
    /// File to log to. When `None`, logs go to stderr (never stdout, which
    /// carries the LSP frames).
    pub log_file: Option<String>,

    /// Arguments passed through to R after a `--` separator.
    pub r_args: Vec<String>,
}

/// Outcome of parsing the `ark lsp` arguments.
pub enum ParsedArgs {
    /// `--help` was requested; the caller should print usage and exit 0.
    Help,

    /// The arguments parsed into a set of options to run with.
    Options(Options),
}

/// Parse the arguments that follow `ark lsp`.
///
/// A pure function over the argument vector so it can be unit tested without
/// spawning a process. Recognizes `--log FILE`, `--help`, and a `--`
/// separator after which the remaining arguments are passed through to R.
/// Any other argument is an error.
pub fn parse_args(args: Vec<String>) -> anyhow::Result<ParsedArgs> {
    let mut log_file: Option<String> = None;
    let mut r_args: Vec<String> = Vec::new();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" => return Ok(ParsedArgs::Help),
            "--log" => {
                let Some(file) = args.next() else {
                    return Err(anyhow!(
                        "A log file must be specified when using the `--log` argument."
                    ));
                };
                log_file = Some(file);
            },
            "--" => {
                // Consume the rest of the arguments for passthrough delivery to R
                r_args.extend(args.by_ref());
                break;
            },
            other => {
                return Err(anyhow!("Argument '{other}' unknown."));
            },
        }
    }

    Ok(ParsedArgs::Options(Options { log_file, r_args }))
}

/// Run the standalone LSP server.
///
/// Boots a full ark kernel as a child process and connects to it as a minimal
/// Jupyter frontend. The LSP wiring lands in a later task; for now the bridge
/// simply keeps the kernel alive until stdin reaches EOF, then shuts it down
/// cleanly.
pub fn run(options: Options) -> anyhow::Result<()> {
    let mut sidecar = sidecar::Sidecar::boot(options.log_file.as_deref())?;
    log::info!("kernel ready (kernel pid: {})", sidecar.child_pid());

    // With no LSP frames to serve yet, block until the frontend closes our
    // stdin, discarding anything sent in the meantime.
    wait_for_stdin_eof();

    log::info!("stdin closed; shutting down kernel");
    sidecar.shutdown()
}

/// Block until stdin reaches EOF, discarding any bytes read.
fn wait_for_stdin_eof() {
    let mut stdin = std::io::stdin().lock();
    let mut buffer = [0u8; 4096];

    loop {
        match stdin.read(&mut buffer) {
            Ok(0) => return,
            Ok(_) => continue,
            Err(err) => {
                log::warn!("Error reading stdin: {err:?}");
                return;
            },
        }
    }
}

/// Print usage for the `ark lsp` subcommand to stdout.
pub fn print_help() {
    println!("Ark {}, an R Kernel.", crate::BUILD_VERSION);
    print!(
        r#"
Usage: ark lsp [OPTIONS]

Run the Ark R language server as a standalone process, communicating over
stdio. Logs go to stderr (or a file); stdout is reserved for LSP frames.

Available options:

--log FILE                   Log to the given file (if not specified, stderr
                             will be used)
-- arg1 arg2 ...             Set the argument list to pass to R
--help                       Print this help message

"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_options(args: &[&str]) -> Options {
        let args = args.iter().map(|arg| arg.to_string()).collect();
        match parse_args(args) {
            Ok(ParsedArgs::Options(options)) => options,
            _ => panic!("Expected parsed options"),
        }
    }

    #[test]
    fn test_parse_args_empty() {
        let options = parse_options(&[]);
        assert_eq!(options.log_file, None);
        assert_eq!(options.r_args, Vec::<String>::new());
    }

    #[test]
    fn test_parse_args_log() {
        let options = parse_options(&["--log", "ark.log"]);
        assert_eq!(options.log_file, Some(String::from("ark.log")));
        assert_eq!(options.r_args, Vec::<String>::new());
    }

    #[test]
    fn test_parse_args_log_missing_value() {
        let args = vec![String::from("--log")];
        assert!(parse_args(args).is_err());
    }

    #[test]
    fn test_parse_args_help() {
        let args = vec![String::from("--help")];
        assert!(matches!(parse_args(args), Ok(ParsedArgs::Help)));
    }

    #[test]
    fn test_parse_args_help_wins_over_later_args() {
        let args = vec![String::from("--help"), String::from("--nope")];
        assert!(matches!(parse_args(args), Ok(ParsedArgs::Help)));
    }

    #[test]
    fn test_parse_args_passthrough() {
        let options = parse_options(&["--", "--vanilla", "foo"]);
        assert_eq!(options.r_args, vec![
            String::from("--vanilla"),
            String::from("foo")
        ]);
    }

    #[test]
    fn test_parse_args_log_and_passthrough() {
        let options = parse_options(&["--log", "ark.log", "--", "--vanilla"]);
        assert_eq!(options.log_file, Some(String::from("ark.log")));
        assert_eq!(options.r_args, vec![String::from("--vanilla")]);
    }

    #[test]
    fn test_parse_args_unknown() {
        let args = vec![String::from("--nope")];
        assert!(parse_args(args).is_err());
    }
}

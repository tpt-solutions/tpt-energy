//! The `tpt-nrg convert` subcommand.
//!
//! Three modes share one argument set:
//!
//! - **Convert** (default) — read a case, write it in another format.
//! - **`--diff`** — compare two cases field by field and exit non-zero when
//!   they disagree, which is how a format migration is validated.
//! - **`--round-trip`** — read a case, write it back in the same format, read
//!   the result, and report what the writer lost.

use std::path::Path;

use crate::{emit, load, resolve_format, CliError, ConvertArgs, FormatArg, OutputFormat};

/// Handle `tpt-nrg convert` in whichever mode was requested.
pub fn run(args: &ConvertArgs) -> Result<(), CliError> {
    if args.diff.is_some() {
        diff_command(args)
    } else if args.round_trip {
        round_trip_command(args)
    } else {
        rewrite_command(args)
    }
}

/// Read `INPUT` and write it out in the `--to` format.
fn rewrite_command(args: &ConvertArgs) -> Result<(), CliError> {
    let from = resolve_format(&args.input, args.from, tpt_nrg_interop::Format::Json)?;
    let to = args
        .to
        .map_or(tpt_nrg_interop::Format::Json, FormatArg::as_format);
    let system = load(&args.input, from)?;
    let out = tpt_nrg_interop::to_text(&system, to)
        .map_err(|e| CliError::Usage(format!("cannot write {to}: {e}")))?;
    emit(args.output.as_deref(), &out)
}

/// Compare `INPUT` against `AGAINST`.
fn diff_command(args: &ConvertArgs) -> Result<(), CliError> {
    let Some(against) = args.diff.as_deref() else {
        return Err(CliError::Usage(
            "--diff needs a second case file to compare against".to_string(),
        ));
    };
    if args.to.is_some() {
        return Err(CliError::Usage(
            "--to selects the output format of a rewrite; it cannot be combined with --diff"
                .to_string(),
        ));
    }
    let left_format = resolve_format(&args.input, args.from, tpt_nrg_interop::Format::Json)?;
    let right_format = resolve_format(against, args.against_from, tpt_nrg_interop::Format::Json)?;
    let left = read(&args.input)?;
    let right = read(against)?;
    let differences = tpt_nrg_interop::diff_text_with_tolerance(
        &left,
        left_format,
        &right,
        right_format,
        args.tolerance,
    )
    .map_err(|e| {
        CliError::Usage(format!(
            "cannot compare {} and {}: {e}",
            args.input.display(),
            against.display()
        ))
    })?;
    let subject = format!("{} and {}", args.input.display(), against.display());
    report(&differences, args.format, &subject)
}

/// Parse, re-emit, and re-parse `INPUT` in its own format.
fn round_trip_command(args: &ConvertArgs) -> Result<(), CliError> {
    if args.to.is_some() {
        return Err(CliError::Usage(
            "--to cannot be combined with --round-trip: a round trip writes the format it read"
                .to_string(),
        ));
    }
    let format = resolve_format(&args.input, args.from, tpt_nrg_interop::Format::Json)?;
    let text = read(&args.input)?;
    let differences = tpt_nrg_interop::diff::round_trip(&text, format)
        .map_err(|e| CliError::Usage(format!("cannot round trip {}: {e}", args.input.display())))?;
    let subject = format!("{} as {format}", args.input.display());
    report(&differences, args.format, &subject)
}

/// Read a file, reporting a usage error rather than an I/O error: a missing
/// case file is bad input, not a broken tool.
fn read(path: &Path) -> Result<String, CliError> {
    std::fs::read_to_string(path)
        .map_err(|e| CliError::Usage(format!("cannot read {}: {e}", path.display())))
}

/// Print the difference list and pick the exit code.
fn report(
    differences: &[tpt_nrg_interop::Difference],
    format: OutputFormat,
    subject: &str,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Table => {
            if differences.is_empty() {
                println!("{subject}: no differences");
            } else {
                println!("{subject}: {} difference(s)", differences.len());
                for d in differences {
                    println!("  {:<8} {}", d.kind(), d);
                }
            }
        }
        OutputFormat::Json => {
            let report = serde_json::json!({
                "subject": subject,
                "identical": differences.is_empty(),
                "count": differences.len(),
                "differences": differences
                    .iter()
                    .map(|d| serde_json::json!({
                        "kind": d.kind().as_str(),
                        "path": d.path(),
                        "left": d.left(),
                        "right": d.right(),
                    }))
                    .collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            );
        }
    }
    if differences.is_empty() {
        Ok(())
    } else {
        // A difference is a failed validation, not a usage error: the caller
        // asked a question and the answer was "they are not the same".
        Err(CliError::Analysis(format!(
            "{} difference(s) between {subject}",
            differences.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(argv: &[&str]) -> ConvertArgs {
        let cli = crate::Cli::try_parse_from(argv).expect("arguments parse");
        let crate::Command::Convert(parsed) = cli.command else {
            panic!("expected `convert`");
        };
        parsed
    }

    /// A two-bus case that is valid per `EnergySystem::validate`.
    const CASE: &str = r#"{
      "id": "case", "name": "Case", "base_mva": 100.0, "frequency_hz": 60.0,
      "buses": [
        {"id": 1, "name": "B1", "type": "Slack"},
        {"id": 2, "name": "B2", "type": "Pq", "load_mw": 10.0, "load_mvar": 5.0}
      ],
      "branches": [{"id": 1, "name": "L1", "from_bus": 1, "to_bus": 2,
                    "resistance_pu": 0.01, "reactance_pu": 0.1}],
      "generators": [{"id": 1, "name": "G1", "bus_id": 1, "type": "Thermal",
                      "p_max_mw": 100.0, "p_min_mw": 10.0}]
    }"#;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tpt-nrg-cli-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn a_diff_of_a_case_against_itself_succeeds() {
        let dir = temp_dir("diff-same");
        let a = dir.join("a.json");
        std::fs::write(&a, CASE).expect("write a");
        let args = parse(&[
            "tpt-nrg",
            "convert",
            a.to_str().expect("utf-8 path"),
            "--diff",
            a.to_str().expect("utf-8 path"),
        ]);
        assert!(run(&args).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_diff_reports_an_analysis_failure() {
        let dir = temp_dir("diff-changed");
        let a = dir.join("a.json");
        let b = dir.join("b.json");
        std::fs::write(&a, CASE).expect("write a");
        std::fs::write(&b, CASE.replace(r#""load_mw": 10.0"#, r#""load_mw": 12.5"#))
            .expect("write b");
        let args = parse(&[
            "tpt-nrg",
            "convert",
            a.to_str().expect("utf-8 path"),
            "--diff",
            b.to_str().expect("utf-8 path"),
        ]);
        assert!(matches!(run(&args), Err(CliError::Analysis(_))));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_lossless_round_trip_succeeds() {
        let dir = temp_dir("round-trip");
        let a = dir.join("a.json");
        std::fs::write(&a, CASE).expect("write a");
        let args = parse(&[
            "tpt-nrg",
            "convert",
            a.to_str().expect("utf-8 path"),
            "--round-trip",
        ]);
        assert!(run(&args).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_rewrite_still_writes_the_requested_format() {
        let dir = temp_dir("rewrite");
        let a = dir.join("a.json");
        let out = dir.join("a.m");
        std::fs::write(&a, CASE).expect("write a");
        let args = parse(&[
            "tpt-nrg",
            "convert",
            a.to_str().expect("utf-8 path"),
            "--to",
            "matpower",
            "-o",
            out.to_str().expect("utf-8 path"),
        ]);
        run(&args).expect("rewrite");
        let written = std::fs::read_to_string(&out).expect("read out");
        assert!(written.contains("mpc.baseMVA = 100;"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_unknown_extension_is_a_usage_error() {
        let dir = temp_dir("extension");
        let a = dir.join("a.txt");
        std::fs::write(&a, CASE).expect("write a");
        assert!(matches!(
            resolve_format(&a, None, tpt_nrg_interop::Format::Json),
            Err(CliError::Usage(_))
        ));
        // An explicit `--from` overrides the extension.
        assert_eq!(
            resolve_format(&a, Some(FormatArg::Json), tpt_nrg_interop::Format::Json)
                .expect("explicit format"),
            tpt_nrg_interop::Format::Json
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_extension_falls_back_to_the_documented_default() {
        let dir = temp_dir("no-extension");
        let a = dir.join("case");
        std::fs::write(&a, CASE).expect("write a");
        assert_eq!(
            resolve_format(&a, None, tpt_nrg_interop::Format::Json).expect("fallback"),
            tpt_nrg_interop::Format::Json
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}

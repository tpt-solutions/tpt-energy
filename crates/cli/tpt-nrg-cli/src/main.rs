//! `tpt-nrg` — command-line interface for TPT Energy.
//!
//! Four subcommands cover the common planning workflows:
//!
//! - `tpt-nrg run --system case.json --method newton-raphson --format table`
//!   solves a case and prints bus voltages, branch flows, and losses.
//! - `tpt-nrg convert --from matpower --to json case.m` rewrites a case in
//!   another exchange format.
//! - `tpt-nrg convert case.json --diff case.m` compares two cases field by
//!   field (via `tpt_nrg_interop::diff`) and exits non-zero when they differ,
//!   which is how a format migration is validated. `--round-trip` does the
//!   same for a single file written back in its own format.
//! - `tpt-nrg viz --system case.json --out diagram.svg` writes a single-line
//!   diagram and voltage/loading heatmap.
//! - `tpt-nrg new my-study` scaffolds a project that loads and solves a case.
//!
//! Every subcommand infers the input format from the file extension when
//! `--from` is omitted, so `case.m` and `case14.json` need no extra flags.
//!
//! Exit codes: `0` success, `1` the analysis failed, `2` bad input or usage.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod convert;
mod dispatch;
mod fault;
mod powerflow;
mod template;
mod viz;

/// Exit code for a usage or input error, as suggested by RFC 0006.
const EXIT_USAGE: u8 = 2;
/// Exit code for an analysis that ran but did not produce a result.
const EXIT_FAILED: u8 = 1;

/// `tpt-nrg` command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "tpt-nrg",
    version,
    about = "Power-systems analysis for TPT Energy",
    long_about = "Solve, convert, and visualise electrical energy systems.\n\n\
                  Accepts MATPOWER, PSS/E, CIM, YAML, CSV, and the native JSON \
                  format; writes any of them back out."
)]
struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    command: Command,
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Solve the system and report results.
    Run(RunArgs),
    /// Convert, compare, or round-trip a case between exchange formats.
    Convert(ConvertArgs),
    /// Render a single-line diagram and heatmap.
    Viz(VizArgs),
    /// Scaffold a new energy-system project.
    New(NewArgs),
}

/// Output rendering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Aligned columns for a terminal.
    Table,
    /// Machine-readable JSON.
    Json,
}

/// Exchange format, as a `clap` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FormatArg {
    /// Native TPT Energy JSON.
    Json,
    /// TPT Energy YAML.
    Yaml,
    /// Flat CSV table.
    Csv,
    /// MATPOWER `case*.m`.
    Matpower,
    /// PSS/E RAW.
    Psse,
    /// CIM / IEC 61970 RDF/XML.
    Cim,
}

impl FormatArg {
    /// The interop format this argument names.
    fn as_format(self) -> tpt_nrg_interop::Format {
        tpt_nrg_interop::Format::from(self)
    }
}

impl From<FormatArg> for tpt_nrg_interop::Format {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Json => Self::Json,
            FormatArg::Yaml => Self::Yaml,
            FormatArg::Csv => Self::Csv,
            FormatArg::Matpower => Self::Matpower,
            FormatArg::Psse => Self::Psse,
            FormatArg::Cim => Self::Cim,
        }
    }
}

/// Power-flow method, as a `clap` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum MethodArg {
    /// Full AC Newton-Raphson.
    NewtonRaphson,
    /// AC Gauss-Seidel.
    GaussSeidel,
    /// Fast decoupled (DC warm start plus Newton-Raphson refinement).
    FastDecoupled,
    /// Linear DC power flow.
    Dc,
}

impl From<MethodArg> for tpt_nrg_powerflow::PowerFlowMethod {
    fn from(value: MethodArg) -> Self {
        match value {
            MethodArg::NewtonRaphson => Self::NewtonRaphson,
            MethodArg::GaussSeidel => Self::GaussSeidel,
            MethodArg::FastDecoupled => Self::FastDecoupled,
            MethodArg::Dc => Self::DcPowerFlow,
        }
    }
}

/// Errors that abort a subcommand, tagged with the exit code they produce.
#[derive(Debug)]
enum CliError {
    /// Bad path, bad flag, or unparseable input.
    Usage(String),
    /// The analysis itself failed (non-convergence, infeasibility, ...).
    Analysis(String),
    /// An I/O failure.
    Io(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(m) | Self::Analysis(m) | Self::Io(m) => f.write_str(m),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Run(args) => run_command(&args),
        Command::Convert(args) => convert::run(&args),
        Command::Viz(args) => viz_command(&args),
        Command::New(args) => new_command(&args),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tpt-nrg: {e}");
            match e {
                CliError::Usage(_) => ExitCode::from(EXIT_USAGE),
                _ => ExitCode::from(EXIT_FAILED),
            }
        }
    }
}

/// Arguments to `tpt-nrg run`.
#[derive(Debug, Parser)]
struct RunArgs {
    /// System to analyse.
    #[arg(short, long, value_name = "FILE")]
    system: PathBuf,

    /// Format of the input file; inferred from the extension when omitted.
    #[arg(long, value_enum)]
    from: Option<FormatArg>,

    /// Solver to use.
    #[arg(short, long, value_enum, default_value = "newton-raphson")]
    method: MethodArg,

    /// Output style.
    #[arg(short, long, value_enum, default_value = "table")]
    format: OutputFormat,

    /// Converge tolerance in per-unit.
    #[arg(long, default_value_t = 1e-6)]
    tolerance: f64,

    /// Maximum solver iterations.
    #[arg(long, default_value_t = 50)]
    max_iterations: usize,

    /// System load in MW; runs an economic dispatch instead of a power flow.
    #[arg(long, value_name = "MW")]
    dispatch: Option<f64>,

    /// Run a 24-hour priority-list unit commitment instead of a power flow.
    #[arg(long, conflicts_with_all = ["dispatch", "fault_bus", "lcoe_capex"])]
    commit: bool,

    /// Fault bus id; runs a three-phase short-circuit study.
    #[arg(long, value_name = "BUS")]
    fault_bus: Option<usize>,

    /// Overnight capital cost in dollars; runs an LCOE calculation.
    #[arg(long, value_name = "DOLLAR", requires = "annual_energy_mwh")]
    lcoe_capex: Option<f64>,

    /// Annual energy production in MWh, required by `--lcoe-capex`.
    #[arg(long, value_name = "MWH", requires = "lcoe_capex")]
    annual_energy_mwh: Option<f64>,

    /// Annual fixed O&M in dollars per year, for `--lcoe-capex`.
    #[arg(
        long,
        value_name = "DOLLAR",
        default_value_t = 0.0,
        requires = "lcoe_capex"
    )]
    lcoe_fixed_om: f64,

    /// Discount rate, for `--lcoe-capex`.
    #[arg(
        long,
        value_name = "RATE",
        default_value_t = 0.07,
        requires = "lcoe_capex"
    )]
    discount_rate: f64,

    /// Project lifetime in years, for `--lcoe-capex`.
    #[arg(
        long,
        value_name = "YEARS",
        default_value_t = 25,
        requires = "lcoe_capex"
    )]
    lifetime_years: u32,
}

/// Arguments to `tpt-nrg convert`.
///
/// The subcommand has three modes, selected by the flags: a plain rewrite
/// (the default), `--diff FILE` against a second case, and `--round-trip`
/// against the case's own re-emission.
#[derive(Debug, Parser)]
#[command(after_help = "Examples:\n  \
    tpt-nrg convert case.m -o case.json\n  \
    tpt-nrg convert case.json --diff case.m\n  \
    tpt-nrg convert case.raw --diff case.m --against-from psse\n  \
    tpt-nrg convert case.m --round-trip --format json")]
struct ConvertArgs {
    /// Case file to read.
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Where to write the result; omit for standard output.
    #[arg(
        short,
        long,
        visible_alias = "out",
        value_name = "FILE",
        conflicts_with_all = ["diff", "round_trip"]
    )]
    output: Option<PathBuf>,

    /// Format of the input file; inferred from the extension when omitted.
    #[arg(long, value_enum)]
    from: Option<FormatArg>,

    /// Format to write; `json` when omitted.
    #[arg(short, long, value_enum)]
    to: Option<FormatArg>,

    /// Compare the input against a second case instead of writing a
    /// conversion, and exit non-zero when the two differ.
    #[arg(long, value_name = "FILE", conflicts_with = "round_trip")]
    diff: Option<PathBuf>,

    /// Format of the `--diff` file; inferred from its extension when omitted.
    #[arg(long, value_enum, requires = "diff")]
    against_from: Option<FormatArg>,

    /// Round-trip the input through its own format and report what the
    /// writer lost, exiting non-zero when anything changed.
    #[arg(long)]
    round_trip: bool,

    /// How to report the differences: a table, or JSON for scripting.
    #[arg(short, long, value_enum, default_value = "table")]
    format: OutputFormat,

    /// Relative-plus-absolute tolerance for floating-point fields.
    #[arg(
        long,
        default_value_t = tpt_nrg_interop::diff::DEFAULT_TOLERANCE,
        requires = "diff"
    )]
    tolerance: f64,
}

/// Arguments to `tpt-nrg new`.
#[derive(Debug, Parser)]
struct NewArgs {
    /// Project name; also the directory and crate name.
    #[arg(value_name = "NAME")]
    name: String,

    /// Directory to create the project in; defaults to the current one.
    #[arg(short, long, value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Overwrite an existing project directory.
    #[arg(short, long)]
    force: bool,

    /// Depend on this checkout by path instead of on crates.io, so the new
    /// project builds before the first release is published.
    #[arg(long, value_name = "REPO_ROOT")]
    local: Option<PathBuf>,
}

/// Arguments to `tpt-nrg viz`.
#[derive(Debug, Parser)]
struct VizArgs {
    /// System to draw.
    #[arg(short, long, value_name = "FILE")]
    system: PathBuf,

    /// Format of the input file; inferred from the extension when omitted.
    #[arg(long, value_enum)]
    from: Option<FormatArg>,

    /// Where to write the SVG; omit for standard output.
    #[arg(short, long, visible_alias = "out", value_name = "FILE")]
    out: Option<PathBuf>,

    /// Hide the legend.
    #[arg(long)]
    no_legend: bool,

    /// Hide per-branch MW flow labels.
    #[arg(long)]
    no_flow_labels: bool,
}

/// Decide the exchange format of `path`.
///
/// An explicit `--from` always wins. Otherwise the file extension decides,
/// so `case.m`, `case.raw`, `case.rdf`, `case.yaml`, `case.csv`, and
/// `case.json` all work without a flag. A path with no extension falls back
/// to `fallback`; a path whose extension is *not* a supported format is a
/// usage error rather than a silent guess, because the resulting parse error
/// would point at the wrong thing.
fn resolve_format(
    path: &Path,
    explicit: Option<FormatArg>,
    fallback: tpt_nrg_interop::Format,
) -> Result<tpt_nrg_interop::Format, CliError> {
    if let Some(format) = explicit {
        return Ok(format.as_format());
    }
    match tpt_nrg_interop::Format::from_path(path) {
        Ok(format) => Ok(format),
        Err(e) if path.extension().is_some() => Err(CliError::Usage(format!(
            "{e}; pass --from to name the format"
        ))),
        Err(_) => Ok(fallback),
    }
}

/// Load a system from disk in the given format.
fn load(
    path: &Path,
    format: tpt_nrg_interop::Format,
) -> Result<tpt_nrg_core::EnergySystem, CliError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CliError::Usage(format!("cannot read {}: {e}", path.display())))?;
    tpt_nrg_interop::from_text(&text, format)
        .map_err(|e| CliError::Usage(format!("cannot parse {}: {e}", path.display())))
}

/// Load a system whose format comes from `--from` or the file extension.
fn load_detected(
    path: &Path,
    explicit: Option<FormatArg>,
) -> Result<tpt_nrg_core::EnergySystem, CliError> {
    let format = resolve_format(path, explicit, tpt_nrg_interop::Format::Json)?;
    load(path, format)
}

/// Write `text` to a file, or to standard output when `path` is `None`.
fn emit(path: Option<&Path>, text: &str) -> Result<(), CliError> {
    match path {
        Some(p) => {
            std::fs::write(p, text)
                .map_err(|e| CliError::Io(format!("cannot write {}: {e}", p.display())))?;
            eprintln!("wrote {}", p.display());
        }
        None => print!("{text}"),
    }
    Ok(())
}

/// Handle `tpt-nrg run`, dispatching to the requested study.
fn run_command(args: &RunArgs) -> Result<(), CliError> {
    let system = load_detected(&args.system, args.from)?;
    if let Some(bus) = args.fault_bus {
        fault::run(&system, bus, args.format)
    } else if args.lcoe_capex.is_some() {
        dispatch::run_lcoe(args, &system, args.format)
    } else if args.commit {
        dispatch::run_commitment(&system, args.format);
        Ok(())
    } else if let Some(load) = args.dispatch {
        dispatch::run_economic(&system, load, args.format)
    } else {
        powerflow::run(args, &system, args.format)
    }
}

/// Handle `tpt-nrg viz`.
fn viz_command(args: &VizArgs) -> Result<(), CliError> {
    let system = load_detected(&args.system, args.from)?;
    let options = viz::options(args);
    let result = powerflow::solve(&system, tpt_nrg_powerflow::PowerFlowMethod::NewtonRaphson).ok();
    let svg = tpt_nrg_viz::render(&system, result.as_ref(), &options);
    emit(args.out.as_deref(), &svg)
}

/// Handle `tpt-nrg new`.
fn new_command(args: &NewArgs) -> Result<(), CliError> {
    template::scaffold(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn run_parses_the_documented_invocation() {
        let cli = Cli::try_parse_from([
            "tpt-nrg",
            "run",
            "--system",
            "case.json",
            "--method",
            "newton-raphson",
            "--format",
            "table",
        ])
        .expect("documented invocation parses");
        let Command::Run(args) = cli.command else {
            panic!("expected `run`");
        };
        assert_eq!(args.system, PathBuf::from("case.json"));
        assert_eq!(args.format, OutputFormat::Table);
    }

    #[test]
    fn convert_parses_both_formats() {
        let cli = Cli::try_parse_from([
            "tpt-nrg", "convert", "--from", "matpower", "--to", "json", "case.m",
        ])
        .expect("documented invocation parses");
        let Command::Convert(args) = cli.command else {
            panic!("expected `convert`");
        };
        assert_eq!(args.from, Some(FormatArg::Matpower));
        assert_eq!(args.to, Some(FormatArg::Json));
        assert_eq!(args.input, PathBuf::from("case.m"));
    }

    #[test]
    fn convert_parses_diff_and_round_trip_flags() {
        let cli = Cli::try_parse_from([
            "tpt-nrg",
            "convert",
            "case.raw",
            "--diff",
            "case.m",
            "--against-from",
            "matpower",
            "--format",
            "json",
        ])
        .expect("diff invocation parses");
        let Command::Convert(args) = cli.command else {
            panic!("expected `convert`");
        };
        assert_eq!(args.diff, Some(PathBuf::from("case.m")));
        assert_eq!(args.against_from, Some(FormatArg::Matpower));
        assert_eq!(args.format, OutputFormat::Json);
        assert!(!args.round_trip);

        let cli = Cli::try_parse_from(["tpt-nrg", "convert", "case.m", "--round-trip"])
            .expect("round-trip invocation parses");
        let Command::Convert(args) = cli.command else {
            panic!("expected `convert`");
        };
        assert!(args.round_trip);
        assert!(args.diff.is_none());
    }

    #[test]
    fn convert_rejects_conflicting_modes() {
        // `--diff` and `--round-trip` are two answers to one question.
        assert!(Cli::try_parse_from([
            "tpt-nrg",
            "convert",
            "case.m",
            "--diff",
            "case.json",
            "--round-trip",
        ])
        .is_err());
        // A diff is a report, not a file to write.
        assert!(Cli::try_parse_from([
            "tpt-nrg",
            "convert",
            "case.m",
            "--diff",
            "case.json",
            "-o",
            "report.txt",
        ])
        .is_err());
        // `--against-from` only means something with `--diff`.
        assert!(
            Cli::try_parse_from(["tpt-nrg", "convert", "case.m", "--against-from", "psse",])
                .is_err()
        );
    }

    #[test]
    fn new_parses_the_documented_invocation() {
        let cli = Cli::try_parse_from([
            "tpt-nrg", "new", "my-study", "--dir", "projects", "--local", ".", "--force",
        ])
        .expect("documented invocation parses");
        let Command::New(args) = cli.command else {
            panic!("expected `new`");
        };
        assert_eq!(args.name, "my-study");
        assert_eq!(args.dir, Some(PathBuf::from("projects")));
        assert_eq!(args.local, Some(PathBuf::from(".")));
        assert!(args.force);
    }

    #[test]
    fn format_detection_follows_the_extension() {
        let cases = [
            ("case.m", tpt_nrg_interop::Format::Matpower),
            ("case.raw", tpt_nrg_interop::Format::Psse),
            ("case.rdf", tpt_nrg_interop::Format::Cim),
            ("case.yml", tpt_nrg_interop::Format::Yaml),
            ("case.csv", tpt_nrg_interop::Format::Csv),
            ("case.json", tpt_nrg_interop::Format::Json),
        ];
        for (name, expected) in cases {
            let detected = resolve_format(Path::new(name), None, tpt_nrg_interop::Format::Json)
                .expect("a known extension is detected");
            assert_eq!(detected, expected, "{name}");
        }
    }

    #[test]
    fn unknown_method_is_rejected() {
        assert!(
            Cli::try_parse_from(["tpt-nrg", "run", "--system", "c.json", "--method", "magic"])
                .is_err()
        );
    }

    #[test]
    fn lcoe_flags_require_each_other() {
        assert!(Cli::try_parse_from([
            "tpt-nrg",
            "run",
            "--system",
            "c.json",
            "--lcoe-capex",
            "1000"
        ])
        .is_err());
    }

    #[test]
    fn format_arg_maps_to_interop_format() {
        assert_eq!(
            tpt_nrg_interop::Format::from(FormatArg::Matpower),
            tpt_nrg_interop::Format::Matpower
        );
        assert_eq!(
            tpt_nrg_interop::Format::from(FormatArg::Cim),
            tpt_nrg_interop::Format::Cim
        );
    }

    #[test]
    fn method_arg_maps_to_solver() {
        use tpt_nrg_powerflow::PowerFlowMethod;
        assert_eq!(
            PowerFlowMethod::from(MethodArg::Dc),
            PowerFlowMethod::DcPowerFlow
        );
        assert_eq!(
            PowerFlowMethod::from(MethodArg::NewtonRaphson),
            PowerFlowMethod::NewtonRaphson
        );
    }
}

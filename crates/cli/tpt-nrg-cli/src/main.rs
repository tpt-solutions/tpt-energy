//! `tpt-nrg` — command-line interface for TPT Energy.
//!
//! Three subcommands cover the common planning workflows:
//!
//! - `tpt-nrg run --system case.json --method newton-raphson --format table`
//!   solves a case and prints bus voltages, branch flows, and losses.
//! - `tpt-nrg convert --from matpower --to json case.m` rewrites a case in
//!   another exchange format.
//! - `tpt-nrg viz --system case.json --out diagram.svg` writes a single-line
//!   diagram and voltage/loading heatmap.
//!
//! Exit codes: `0` success, `1` the analysis failed, `2` bad input or usage.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod dispatch;
mod fault;
mod powerflow;
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
    /// Convert a case between exchange formats.
    Convert(ConvertArgs),
    /// Render a single-line diagram and heatmap.
    Viz(VizArgs),
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
        Command::Convert(args) => convert_command(&args),
        Command::Viz(args) => viz_command(&args),
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

    /// Format of the input file.
    #[arg(long, value_enum, default_value = "json")]
    from: FormatArg,

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
#[derive(Debug, Parser)]
struct ConvertArgs {
    /// Case file to read.
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Where to write the result; omit for standard output.
    #[arg(short, long, visible_alias = "out", value_name = "FILE")]
    output: Option<PathBuf>,

    /// Format of the input file.
    #[arg(long, value_enum, default_value = "json")]
    from: FormatArg,

    /// Format to write.
    #[arg(short, long, value_enum, default_value = "json")]
    to: FormatArg,
}

/// Arguments to `tpt-nrg viz`.
#[derive(Debug, Parser)]
struct VizArgs {
    /// System to draw.
    #[arg(short, long, value_name = "FILE")]
    system: PathBuf,

    /// Format of the input file.
    #[arg(long, value_enum, default_value = "json")]
    from: FormatArg,

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

/// Load a system from disk in the given format.
fn load(path: &Path, format: FormatArg) -> Result<tpt_nrg_core::EnergySystem, CliError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CliError::Usage(format!("cannot read {}: {e}", path.display())))?;
    let format: tpt_nrg_interop::Format = format.into();
    tpt_nrg_interop::from_text(&text, format)
        .map_err(|e| CliError::Usage(format!("cannot parse {}: {e}", path.display())))
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
    let system = load(&args.system, args.from)?;
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

/// Handle `tpt-nrg convert`.
fn convert_command(args: &ConvertArgs) -> Result<(), CliError> {
    let text = std::fs::read_to_string(&args.input)
        .map_err(|e| CliError::Usage(format!("cannot read {}: {e}", args.input.display())))?;
    let from: tpt_nrg_interop::Format = args.from.into();
    let to: tpt_nrg_interop::Format = args.to.into();
    let system = tpt_nrg_interop::from_text(&text, from)
        .map_err(|e| CliError::Usage(format!("cannot parse {}: {e}", args.input.display())))?;
    let out = tpt_nrg_interop::to_text(&system, to)
        .map_err(|e| CliError::Usage(format!("cannot write {to}: {e}")))?;
    emit(args.output.as_deref(), &out)
}

/// Handle `tpt-nrg viz`.
fn viz_command(args: &VizArgs) -> Result<(), CliError> {
    let system = load(&args.system, args.from)?;
    let options = viz::options(args);
    let result = powerflow::solve(&system, tpt_nrg_powerflow::PowerFlowMethod::NewtonRaphson).ok();
    let svg = tpt_nrg_viz::render(&system, result.as_ref(), &options);
    emit(args.out.as_deref(), &svg)
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
        assert_eq!(args.from, FormatArg::Matpower);
        assert_eq!(args.to, FormatArg::Json);
        assert_eq!(args.input, PathBuf::from("case.m"));
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

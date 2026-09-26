//! YAML and CSV import / export for [`EnergySystem`].
//!
//! YAML is the same schema as the native JSON (it deserializes the same
//! `serde` representation), so a hand-written case stays short:
//!
//! ```yaml
//! id: demo
//! name: Two-bus demo
//! base_mva: 100.0
//! frequency_hz: 60.0
//! buses:
//!   - id: 1
//!     name: Slack
//!     type: Slack
//!     voltage_magnitude_pu: 1.06
//! ```
//!
//! CSV comes in two shapes:
//!
//! - **Bundled** — a directory of one table per record type
//!   (`buses.csv`, `branches.csv`, `generators.csv`, `loads.csv`,
//!   `storage.csv`, `system.csv`).
//! - **Flat** — a single table with a `record` column (`bus`, `branch`,
//!   `generator`, `load`, `storage`, `system`).
//!
//! Both are useful: bundled for a system handed to you, flat for a system
//! that has to fit in one file or one spreadsheet.

use std::path::Path;

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType, Load, Storage};

use crate::error::{field_index, InteropError, InteropResult};

/// Parse an [`EnergySystem`] from YAML.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if the document is not valid YAML and
/// [`InteropError::InvalidSystem`] if the result fails structural validation.
pub fn from_yaml(text: &str) -> InteropResult<EnergySystem> {
    let system: EnergySystem =
        serde_norway::from_str(text).map_err(|e| InteropError::parse("yaml", e.to_string()))?;
    system.validate().map_err(InteropError::InvalidSystem)?;
    Ok(system)
}

/// Serialize an [`EnergySystem`] to YAML.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if serialization fails.
pub fn to_yaml(system: &EnergySystem) -> InteropResult<String> {
    serde_norway::to_string(system).map_err(|e| InteropError::parse("yaml", e.to_string()))
}

/// Read a bundled CSV system from a directory of tables.
///
/// Missing optional tables (`branches.csv`, `generators.csv`, `loads.csv`,
/// `storage.csv`, `system.csv`) are skipped; a missing `buses.csv` is an
/// error.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `buses.csv` is absent,
/// [`InteropError::Parse`] if a table is malformed, and
/// [`InteropError::InvalidSystem`] if the result fails structural validation.
pub fn from_csv_dir(dir: impl AsRef<Path>) -> InteropResult<EnergySystem> {
    let dir = dir.as_ref();
    let buses_path = dir.join("buses.csv");
    if !buses_path.is_file() {
        return Err(InteropError::missing("csv", "buses.csv"));
    }
    let header = optional_header(dir, "system.csv");
    let mut sys = EnergySystem::new(
        &header.id,
        &header.name,
        header.base_mva,
        header.frequency_hz,
    );
    add_csv_buses(&mut sys, &read_table(&buses_path)?)?;
    add_csv_branches(&mut sys, &read_optional(dir, "branches.csv")?)?;
    add_csv_generators(&mut sys, &read_optional(dir, "generators.csv")?)?;
    add_csv_loads(&mut sys, &read_optional(dir, "loads.csv")?)?;
    add_csv_storage(&mut sys, &read_optional(dir, "storage.csv")?)?;
    sys.validate().map_err(InteropError::InvalidSystem)?;
    Ok(sys)
}

/// Add the buses of a CSV table (header row first) to a system.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if the table has no header row and
/// propagates any row conversion failure.
fn add_csv_buses(sys: &mut EnergySystem, rows: &[csv::StringRecord]) -> InteropResult<()> {
    let Some((header, data)) = rows.split_first() else {
        return Ok(());
    };
    for row in data {
        let bus = bus_from_row(&Row::new(header, row))?;
        sys.add_bus(bus).map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Add the branches of a CSV table (header row first) to a system.
///
/// # Errors
///
/// Propagates any row conversion failure.
fn add_csv_branches(sys: &mut EnergySystem, rows: &[csv::StringRecord]) -> InteropResult<()> {
    let Some((header, data)) = rows.split_first() else {
        return Ok(());
    };
    for row in data {
        let branch = branch_from_row(&Row::new(header, row))?;
        sys.add_branch(branch)
            .map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Add the generators of a CSV table (header row first) to a system.
///
/// # Errors
///
/// Propagates any row conversion failure.
fn add_csv_generators(sys: &mut EnergySystem, rows: &[csv::StringRecord]) -> InteropResult<()> {
    let Some((header, data)) = rows.split_first() else {
        return Ok(());
    };
    for row in data {
        let gen = generator_from_row(&Row::new(header, row))?;
        sys.add_generator(gen)
            .map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Add the loads of a CSV table (header row first) to a system.
///
/// # Errors
///
/// Propagates any row conversion failure.
fn add_csv_loads(sys: &mut EnergySystem, rows: &[csv::StringRecord]) -> InteropResult<()> {
    let Some((header, data)) = rows.split_first() else {
        return Ok(());
    };
    for row in data {
        let load = load_from_row(&Row::new(header, row))?;
        sys.add_load(load).map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Add the storage units of a CSV table (header row first) to a system.
///
/// # Errors
///
/// Propagates any row conversion failure.
fn add_csv_storage(sys: &mut EnergySystem, rows: &[csv::StringRecord]) -> InteropResult<()> {
    let Some((header, data)) = rows.split_first() else {
        return Ok(());
    };
    for row in data {
        let unit = storage_from_row(&Row::new(header, row))?;
        sys.add_storage(unit).map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Read a flat CSV system: one table with a `record` column.
///
/// The `record` column selects the record type of each row; every other
/// column is matched by header name using the same names as the JSON schema.
/// A row with `record = system` supplies the top-level id, name, base MVA,
/// and frequency; if absent, default values are used.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if the table has no header row or no
/// `record` column, [`InteropError::BadValue`] if a record name is unknown,
/// and [`InteropError::InvalidSystem`] if the result fails structural
/// validation.
pub fn from_flat_csv(text: &str) -> InteropResult<EnergySystem> {
    let rows = parse_table_text(text)?;
    let (header, data) = rows
        .split_first()
        .ok_or_else(|| InteropError::missing("csv", "header row"))?;
    if !header
        .iter()
        .any(|h| h.trim().eq_ignore_ascii_case("record"))
    {
        return Err(InteropError::missing("csv", "a `record` column"));
    }

    let mut top = SystemHeader::default();
    let mut sys = EnergySystem::new(&top.id, &top.name, top.base_mva, top.frequency_hz);
    for row in data {
        let row = Row::new(header, row);
        let kind = row.raw("record").unwrap_or_default().to_ascii_lowercase();
        match kind.as_str() {
            "system" => {
                top = system_header(&row);
                sys = EnergySystem::new(&top.id, &top.name, top.base_mva, top.frequency_hz);
            }
            "bus" => {
                let bus = bus_from_row(&row)?;
                sys.add_bus(bus).map_err(InteropError::InvalidSystem)?;
            }
            "branch" => {
                let branch = branch_from_row(&row)?;
                sys.add_branch(branch)
                    .map_err(InteropError::InvalidSystem)?;
            }
            "generator" => {
                let gen = generator_from_row(&row)?;
                sys.add_generator(gen)
                    .map_err(InteropError::InvalidSystem)?;
            }
            "load" => {
                let load = load_from_row(&row)?;
                sys.add_load(load).map_err(InteropError::InvalidSystem)?;
            }
            "storage" => {
                let unit = storage_from_row(&row)?;
                sys.add_storage(unit).map_err(InteropError::InvalidSystem)?;
            }
            other => {
                return Err(InteropError::bad_value(
                    "csv",
                    "record",
                    format!("unknown record type `{other}`"),
                ))
            }
        }
    }
    sys.validate().map_err(InteropError::InvalidSystem)?;
    Ok(sys)
}

/// Top-level system fields, shared by the bundled and flat CSV layouts.
#[derive(Debug)]
struct SystemHeader {
    /// System identifier.
    id: String,
    /// Human-readable name.
    name: String,
    /// Per-unit base in MVA.
    base_mva: f64,
    /// Nominal frequency in Hz.
    frequency_hz: f64,
}

impl Default for SystemHeader {
    fn default() -> Self {
        Self {
            id: "csv-system".to_string(),
            name: "CSV system".to_string(),
            base_mva: 100.0,
            frequency_hz: 60.0,
        }
    }
}

/// One CSV data row, resolved against its header row.
///
/// Columns are matched case- and space-insensitively, and an empty cell is
/// indistinguishable from an absent column, so every accessor returns an
/// `Option` and the callers pick the default.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    /// The header record naming the columns.
    header: &'a csv::StringRecord,
    /// The data record.
    data: &'a csv::StringRecord,
}

impl<'a> Row<'a> {
    /// Pair a data record with its header.
    #[must_use]
    pub fn new(header: &'a csv::StringRecord, data: &'a csv::StringRecord) -> Self {
        Self { header, data }
    }

    /// Index of a column by name, or `None` if the column is absent.
    fn column(&self, name: &str) -> Option<usize> {
        self.header
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(name))
    }

    /// Raw cell text, `None` when the column or cell is absent or empty.
    fn raw(&self, name: &str) -> Option<&'a str> {
        let value = self.data.get(self.column(name)?)?.trim();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    /// Cell text with a fallback.
    fn text_or(&self, name: &str, fallback: &str) -> String {
        self.raw(name).unwrap_or(fallback).to_string()
    }

    /// Cell parsed as `f64`, `None` when absent or unparseable.
    fn num(&self, name: &str) -> Option<f64> {
        self.raw(name)?.parse::<f64>().ok()
    }

    /// Cell parsed as `f64`, defaulting to `0.0`.
    fn num_or_zero(&self, name: &str) -> f64 {
        self.num(name).unwrap_or(0.0)
    }

    /// Cell parsed as a positive integer, defaulting to `0`.
    fn index(&self, name: &str) -> usize {
        field_index(self.num(name).unwrap_or(0.0))
    }

    /// Identifier column, defaulting to `1` when absent.
    fn id_or_one(&self) -> usize {
        let explicit = self.index("id");
        if explicit == 0 {
            1
        } else {
            explicit
        }
    }

    /// Boolean column, where an absent or empty cell means `true`.
    fn flag(&self, name: &str) -> bool {
        match self.raw(name) {
            None => true,
            Some(v) => !matches!(
                v.to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off" | "out"
            ),
        }
    }
}

/// Read a bus type cell (`Slack`, `PV`, `PQ`, `Isolated`, or the MATPOWER
/// codes 1-4), defaulting to `PQ`.
fn bus_type_cell(row: &Row<'_>) -> InteropResult<BusType> {
    let value = row.text_or("type", "Pq").to_ascii_lowercase();
    match value.as_str() {
        "slack" | "ref" | "reference" | "3" => Ok(BusType::Slack),
        "pv" | "gen" | "generator" | "2" => Ok(BusType::Pv),
        "pq" | "load" | "1" => Ok(BusType::Pq),
        "isolated" | "4" => Ok(BusType::Isolated),
        other => Err(InteropError::bad_value(
            "csv",
            "type",
            format!("unknown bus type `{other}`"),
        )),
    }
}

/// Read a generator-type cell, defaulting to `Thermal`.
fn generator_type_cell(row: &Row<'_>) -> GeneratorType {
    match row.text_or("type", "Thermal").to_ascii_lowercase().as_str() {
        "hydro" | "hydropower" => GeneratorType::Hydro,
        "wind" => GeneratorType::Wind,
        "solar" | "pv" => GeneratorType::Solar,
        "nuclear" => GeneratorType::Nuclear,
        "geothermal" => GeneratorType::Geothermal,
        "other" => GeneratorType::Other,
        _ => GeneratorType::Thermal,
    }
}

/// Read the system-level fields out of a row.
fn system_header(row: &Row<'_>) -> SystemHeader {
    SystemHeader {
        id: row.text_or("id", "csv-system"),
        name: row.text_or("name", "CSV system"),
        base_mva: row.num("base_mva").unwrap_or(100.0),
        frequency_hz: row.num("frequency_hz").unwrap_or(60.0),
    }
}

/// Read the system header from an optional table, or the default.
fn optional_header(dir: &Path, name: &str) -> SystemHeader {
    let rows = read_optional(dir, name).unwrap_or_default();
    match rows.split_first() {
        Some((header, data)) => data.first().map_or_else(SystemHeader::default, |row| {
            system_header(&Row::new(header, row))
        }),
        None => SystemHeader::default(),
    }
}

/// Parse a CSV document into records, tolerating a UTF-8 BOM.
///
/// Header handling is done by this crate rather than by the `csv` reader, so
/// `has_headers` is off and the header row is returned as record zero. That
/// keeps a single code path for the bundled and flat layouts.
fn parse_table_text(text: &str) -> InteropResult<Vec<csv::StringRecord>> {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(body.as_bytes());
    let mut out = Vec::new();
    for record in reader.records() {
        out.push(record.map_err(|e| InteropError::parse("csv", e.to_string()))?);
    }
    Ok(out)
}

/// Read a CSV file into records.
fn read_table(path: &Path) -> InteropResult<Vec<csv::StringRecord>> {
    let text = std::fs::read_to_string(path)?;
    parse_table_text(&text)
}

/// Read an optional table, returning an empty slice when the file is absent.
fn read_optional(dir: &Path, name: &str) -> InteropResult<Vec<csv::StringRecord>> {
    let path = dir.join(name);
    if path.is_file() {
        read_table(&path)
    } else {
        Ok(Vec::new())
    }
}

/// Build a [`Bus`] from a CSV row.
///
/// Columns: `id`, `name`, `type`, `voltage_magnitude_pu`,
/// `voltage_angle_rad`, `base_kv`, `load_mw`, `load_mvar`,
/// `shunt_conductance_pu`, `shunt_susceptance_pu`.
///
/// A bus is always in service in the TPT Energy data model; the bus equivalent
/// of `in_service` is [`BusType::Isolated`].
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `id` is absent and
/// [`InteropError::BadValue`] if `type` is not a recognised bus type.
pub fn bus_from_row(row: &Row<'_>) -> InteropResult<Bus> {
    let id = row.index("id");
    if id == 0 {
        return Err(InteropError::missing("csv", "bus `id` column"));
    }
    let name = row.text_or("name", &format!("Bus {id}"));
    Ok(Bus::new(id, name, bus_type_cell(row)?)
        .with_voltage_pu(
            row.num("voltage_magnitude_pu").unwrap_or(1.0),
            row.num("voltage_angle_rad").unwrap_or(0.0),
        )
        .with_base_kv(row.num("base_kv").unwrap_or(100.0))
        .with_load(row.num_or_zero("load_mw"), row.num_or_zero("load_mvar"))
        .with_shunt(
            row.num_or_zero("shunt_conductance_pu"),
            row.num_or_zero("shunt_susceptance_pu"),
        ))
}

/// Build a [`Branch`] from a CSV row.
///
/// Columns: `id`, `name`, `from_bus`, `to_bus`, `resistance_pu`,
/// `reactance_pu`, `susceptance_pu`, `tap_ratio`, `phase_shift_rad`,
/// `rating_mva`, `in_service`.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `from_bus` or `to_bus` is absent.
pub fn branch_from_row(row: &Row<'_>) -> InteropResult<Branch> {
    let from_bus = row.index("from_bus");
    let to_bus = row.index("to_bus");
    if from_bus == 0 || to_bus == 0 {
        return Err(InteropError::missing(
            "csv",
            "branch `from_bus` / `to_bus` columns",
        ));
    }
    let name = row.text_or("name", &format!("{from_bus}-{to_bus}"));
    Ok(Branch::new(
        row.id_or_one(),
        name,
        from_bus,
        to_bus,
        row.num_or_zero("resistance_pu"),
        row.num_or_zero("reactance_pu"),
    )
    .with_susceptance(row.num_or_zero("susceptance_pu"))
    .with_tap(
        row.num("tap_ratio").unwrap_or(1.0),
        row.num_or_zero("phase_shift_rad"),
    )
    .with_rating(row.num("rating_mva").unwrap_or(100.0))
    .with_in_service(row.flag("in_service")))
}

/// Build a [`Generator`] from a CSV row.
///
/// Columns: `id`, `name`, `bus_id`, `type`, `p_max_mw`, `p_min_mw`,
/// `q_max_mvar`, `q_min_mvar`, `p_schedule_mw`, `voltage_setpoint_pu`,
/// `in_service`.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `bus_id` is absent.
pub fn generator_from_row(row: &Row<'_>) -> InteropResult<Generator> {
    let bus_id = row.index("bus_id");
    if bus_id == 0 {
        return Err(InteropError::missing("csv", "generator `bus_id` column"));
    }
    let id = row.id_or_one();
    let name = row.text_or("name", &format!("G{id}"));
    let mut gen = Generator::new(
        id,
        name,
        generator_type_cell(row),
        row.num("p_max_mw").unwrap_or(0.0),
        row.num("p_min_mw").unwrap_or(0.0),
    )
    .at_bus(bus_id)
    .with_p_schedule(row.num_or_zero("p_schedule_mw"))
    .with_voltage_setpoint(row.num("voltage_setpoint_pu").unwrap_or(1.0))
    .with_reactive_limits(
        row.num("q_min_mvar").unwrap_or(-9999.0),
        row.num("q_max_mvar").unwrap_or(9999.0),
    );
    gen.in_service = row.flag("in_service");
    Ok(gen)
}

/// Build a [`Load`] from a CSV row.
///
/// Columns: `id`, `name`, `bus_id`, `p_mw`, `q_mvar`, `in_service`.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `bus_id` is absent.
pub fn load_from_row(row: &Row<'_>) -> InteropResult<Load> {
    let bus_id = row.index("bus_id");
    if bus_id == 0 {
        return Err(InteropError::missing("csv", "load `bus_id` column"));
    }
    let id = row.id_or_one();
    let name = row.text_or("name", &format!("L{id}"));
    let mut load = Load::new(
        id,
        name,
        bus_id,
        row.num_or_zero("p_mw"),
        row.num_or_zero("q_mvar"),
    );
    load.in_service = row.flag("in_service");
    Ok(load)
}

/// Build a [`Storage`] from a CSV row.
///
/// Columns: `id`, `name`, `bus_id`, `energy_capacity_mwh`, `power_rating_mw`,
/// `initial_soc`, `round_trip_efficiency`, `in_service`.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `bus_id` is absent.
pub fn storage_from_row(row: &Row<'_>) -> InteropResult<Storage> {
    let bus_id = row.index("bus_id");
    if bus_id == 0 {
        return Err(InteropError::missing("csv", "storage `bus_id` column"));
    }
    let id = row.id_or_one();
    let name = row.text_or("name", &format!("S{id}"));
    let mut storage = Storage::new(
        id,
        name,
        bus_id,
        row.num_or_zero("energy_capacity_mwh"),
        row.num_or_zero("power_rating_mw"),
    )
    .with_initial_soc(row.num("initial_soc").unwrap_or(0.5))
    .with_round_trip_efficiency(row.num("round_trip_efficiency").unwrap_or(0.9));
    storage.in_service = row.flag("in_service");
    Ok(storage)
}

/// Header of the flat CSV table: the union of every record type's columns.
const FLAT_CSV_HEADER: [&str; 29] = [
    "record",
    "id",
    "name",
    "type",
    "bus_id",
    "from_bus",
    "to_bus",
    "base_kv",
    "voltage_magnitude_pu",
    "voltage_angle_rad",
    "load_mw",
    "load_mvar",
    "p_mw",
    "q_mvar",
    "p_max_mw",
    "p_min_mw",
    "p_schedule_mw",
    "q_max_mvar",
    "q_min_mvar",
    "voltage_setpoint_pu",
    "resistance_pu",
    "reactance_pu",
    "susceptance_pu",
    "tap_ratio",
    "phase_shift_rad",
    "rating_mva",
    "energy_capacity_mwh",
    "power_rating_mw",
    "in_service",
];

/// Serialize an [`EnergySystem`] as one flat CSV table.
///
/// The output is the inverse of [`from_flat_csv`]: a single table with a
/// `record` column and the union of every record type's columns. Empty cells
/// mean "use the default", so the importer recovers the same system.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if a record cannot be written by the CSV
/// writer, or if the result is not valid UTF-8.
pub fn to_flat_csv(system: &EnergySystem) -> InteropResult<String> {
    // Rows legitimately carry different subsets of the union of columns, so
    // the writer must accept ragged records; the importer resolves each cell
    // against the header row and treats a short row as "no such column".
    let mut writer = csv::WriterBuilder::new()
        .flexible(true)
        .from_writer(Vec::new());
    writer
        .write_record(FLAT_CSV_HEADER)
        .map_err(|e| csv_error(&e))?;
    write_flat_buses(&mut writer, system)?;
    write_flat_branches(&mut writer, system)?;
    write_flat_generators(&mut writer, system)?;
    write_flat_loads(&mut writer, system)?;
    write_flat_storage(&mut writer, system)?;
    let bytes = writer
        .into_inner()
        .map_err(|e| InteropError::parse("csv", e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| InteropError::parse("csv", e.to_string()))
}

/// Write the bus rows of a flat CSV table.
fn write_flat_buses<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    system: &EnergySystem,
) -> InteropResult<()> {
    for b in &system.buses {
        writer
            .write_record([
                "bus",
                &b.id.to_string(),
                &b.name,
                bus_type_label(b.bus_type),
                "",
                "",
                "",
                &cell(b.base_kv),
                &cell(b.voltage_magnitude_pu),
                &cell(b.voltage_angle_rad),
                &cell(b.load_mw),
                &cell(b.load_mvar),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ])
            .map_err(|e| csv_error(&e))?;
    }
    Ok(())
}

/// Write the branch rows of a flat CSV table.
fn write_flat_branches<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    system: &EnergySystem,
) -> InteropResult<()> {
    for br in &system.branches {
        writer
            .write_record([
                "branch",
                &br.id.to_string(),
                &br.name,
                "",
                "",
                &br.from_bus.to_string(),
                &br.to_bus.to_string(),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &cell(br.resistance_pu),
                &cell(br.reactance_pu),
                &cell(br.susceptance_pu),
                &cell(br.tap_ratio),
                &cell(br.phase_shift_rad),
                &cell(br.rating_mva),
                "",
                "",
                &flag(br.in_service),
            ])
            .map_err(|e| csv_error(&e))?;
    }
    Ok(())
}

/// Write the generator rows of a flat CSV table.
fn write_flat_generators<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    system: &EnergySystem,
) -> InteropResult<()> {
    for g in &system.generators {
        writer
            .write_record([
                "generator",
                &g.id.to_string(),
                &g.name,
                generator_type_label(g.generator_type),
                &g.bus_id.to_string(),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &cell(g.p_max_mw),
                &cell(g.p_min_mw),
                &cell(g.p_schedule_mw),
                &cell(g.q_max_mvar),
                &cell(g.q_min_mvar),
                &cell(g.voltage_setpoint_pu),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &flag(g.in_service),
            ])
            .map_err(|e| csv_error(&e))?;
    }
    Ok(())
}

/// Write the load rows of a flat CSV table.
fn write_flat_loads<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    system: &EnergySystem,
) -> InteropResult<()> {
    for l in &system.loads {
        writer
            .write_record([
                "load",
                &l.id.to_string(),
                &l.name,
                "",
                &l.bus_id.to_string(),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &cell(l.p_mw),
                &cell(l.q_mvar),
            ])
            .map_err(|e| csv_error(&e))?;
    }
    Ok(())
}

/// Write the storage rows of a flat CSV table.
fn write_flat_storage<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    system: &EnergySystem,
) -> InteropResult<()> {
    for s in &system.storage {
        writer
            .write_record([
                "storage",
                &s.id.to_string(),
                &s.name,
                "",
                &s.bus_id.to_string(),
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &cell(s.energy_capacity_mwh),
                &cell(s.power_rating_mw),
                &flag(s.in_service),
            ])
            .map_err(|e| csv_error(&e))?;
    }
    Ok(())
}

/// Format a float for a CSV cell.
fn cell(v: f64) -> String {
    v.to_string()
}

/// Format an in-service flag for a CSV cell.
fn flag(v: bool) -> String {
    u8::from(v).to_string()
}

/// Wrap a CSV writer error.
fn csv_error(e: &csv::Error) -> InteropError {
    InteropError::parse("csv", e.to_string())
}

/// Canonical label for a [`BusType`].
fn bus_type_label(t: BusType) -> &'static str {
    match t {
        BusType::Slack => "Slack",
        BusType::Pv => "Pv",
        BusType::Pq => "Pq",
        BusType::Isolated => "Isolated",
    }
}

/// Canonical label for a [`GeneratorType`].
fn generator_type_label(t: GeneratorType) -> &'static str {
    match t {
        GeneratorType::Thermal => "Thermal",
        GeneratorType::Hydro => "Hydro",
        GeneratorType::Wind => "Wind",
        GeneratorType::Solar => "Solar",
        GeneratorType::Nuclear => "Nuclear",
        GeneratorType::Geothermal => "Geothermal",
        GeneratorType::Other => "Other",
    }
}

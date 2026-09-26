//! Microgrid droop control.
//!
//! One control step is: measure, classify each asset, and allocate the power
//! balance in a fixed merit order — renewables first (they are free and
//! weather-driven), then the battery, then dispatchable generation, and only
//! then load shedding. Frequency and voltage droops bias the battery and the
//! dispatchable units so the loop also corrects a persistent frequency or
//! voltage offset rather than only the instantaneous imbalance.
//!
//! The law is deliberately simple and fully documented rather than optimal:
//! it is small enough to run at control-loop rates in a browser, and every
//! term is inspectable from `JavaScript`.

use serde::{Deserialize, Serialize};

use crate::WasmError;

/// Convert a count to `f64` to share a setpoint evenly across assets.
///
/// Exact for magnitudes up to 2^53, far beyond the number of assets a
/// microgrid can have, so the precision-loss allowance lives here.
#[must_use]
#[allow(clippy::cast_precision_loss)]
fn count_to_f64(value: usize) -> f64 {
    value as f64
}

/// Droop and dispatch tuning for [`microgrid_step`].
///
/// Every field has a default, so a caller may send a partial JSON object and
/// only override the droops it cares about.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlConfig {
    /// Nominal system frequency in Hz.
    pub nominal_frequency_hz: f64,
    /// Nominal bus voltage in per-unit.
    pub nominal_voltage_pu: f64,
    /// Active-power droop: MW of response per Hz of frequency error.
    pub frequency_droop_mw_per_hz: f64,
    /// Reactive-power droop: `MVAr` of response, per unit (pu) of voltage error.
    pub voltage_droop_mvar_per_pu: f64,
    /// Minimum state of charge the battery will not discharge below.
    pub min_state_of_charge: f64,
    /// Maximum state of charge the battery will not charge above.
    pub max_state_of_charge: f64,
    /// Control interval in hours, used to convert power to energy.
    pub interval_hours: f64,
    /// Maximum load fraction to shed when the island cannot balance.
    pub max_shed_fraction: f64,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            nominal_frequency_hz: 60.0,
            nominal_voltage_pu: 1.0,
            // Typical inverter droop: 5% frequency drop for full rated power.
            frequency_droop_mw_per_hz: 0.02,
            // 5% voltage drop for full reactive capability.
            voltage_droop_mvar_per_pu: 20.0,
            min_state_of_charge: 0.1,
            max_state_of_charge: 0.95,
            interval_hours: 1.0,
            max_shed_fraction: 0.5,
        }
    }
}

/// Field measurements for one control step.
///
/// Every field is optional so a browser caller can send only what it actually
/// measures; anything absent falls back to the nominal value from the
/// [`ControlConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurements {
    /// Measured frequency in Hz; defaults to nominal when absent.
    #[serde(default)]
    pub frequency_hz: Option<f64>,
    /// Measured bus voltage in per-unit; defaults to nominal when absent.
    #[serde(default)]
    pub voltage_pu: Option<f64>,
    /// Measured total demand in MW; defaults to the state assets imply.
    #[serde(default)]
    pub demand_mw: Option<f64>,
    /// Whether the point of common coupling is live.
    #[serde(default = "default_true")]
    pub grid_connected: bool,
}

impl Default for Measurements {
    /// Grid-connected, with no measurement taken yet: the controller then
    /// falls back to the nominal frequency and voltage.
    fn default() -> Self {
        Self {
            frequency_hz: None,
            voltage_pu: None,
            demand_mw: None,
            grid_connected: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// The outcome of one control step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlDecision {
    /// Per-asset target setpoints.
    pub actions: Vec<crate::ControlAction>,
    /// Net power the utility interchange must supply (positive = import).
    pub utility_mw: f64,
    /// Power still unmatched after all actions; zero when the step balanced.
    pub residual_mw: f64,
    /// Load fraction that had to be shed to balance, in `[0, 1]`.
    pub load_shed_fraction: f64,
    /// Whether the island balanced without shedding.
    pub balanced: bool,
    /// Whether a grid-forming reference was established (islanded only).
    pub grid_forming: bool,
}

/// Decide the next setpoint for every asset in `state`.
///
/// This is the typed entry point behind [`microgrid_step_json`](crate::microgrid_step_json);
/// a native caller can use it directly to avoid a JSON round trip.
///
/// # Errors
///
/// Returns [`WasmError`] of kind `control` when the measurements or the tuning
/// are non-finite or out of range.
pub fn microgrid_step(
    state: &crate::MicrogridState,
    measurements: &Measurements,
    config: &ControlConfig,
) -> Result<ControlDecision, WasmError> {
    check_inputs(measurements, config)?;
    let frequency = measurements
        .frequency_hz
        .unwrap_or(config.nominal_frequency_hz);
    let voltage = measurements.voltage_pu.unwrap_or(config.nominal_voltage_pu);
    let grid_connected = state.grid_connected && measurements.grid_connected;

    // Droop corrections: a frequency *below* nominal asks for more active
    // power, and a voltage *below* nominal asks for more reactive support.
    let droop_mw = (config.nominal_frequency_hz - frequency) * config.frequency_droop_mw_per_hz;
    let droop_mvar = (config.nominal_voltage_pu - voltage) * config.voltage_droop_mvar_per_pu;

    let mut balance = measured_demand(state, measurements);
    let mut actions: Vec<crate::ControlAction> = Vec::with_capacity(state.assets.len());

    // 1. Renewables run at whatever they are producing: they are free, and
    //    curtailing them would only increase the imbalance.
    for asset in state.assets.iter().filter(|a| a.is_renewable()) {
        balance -= asset.output_mw;
        actions.push(crate::ControlAction {
            asset_id: asset.id.clone(),
            kind: asset.kind.clone(),
            target_p_mw: asset.output_mw,
            target_q_mvar: 0.0,
            new_state_of_charge: asset.state_of_charge,
            action: "hold".to_string(),
        });
    }

    // 2. The battery takes the fast share of the imbalance, biased by the
    //    frequency droop, limited by its energy headroom.
    if let Some(battery) = state.assets.iter().find(|a| a.is_battery()) {
        let (target, soc) = battery_step(battery, balance + droop_mw, config);
        balance -= target;
        actions.push(crate::ControlAction {
            asset_id: battery.id.clone(),
            kind: battery.kind.clone(),
            target_p_mw: target,
            target_q_mvar: 0.0,
            new_state_of_charge: Some(soc),
            action: if target > 0.0 { "discharge" } else { "charge" }.to_string(),
        });
    }

    // 3. Dispatchable generation covers the rest when grid-connected, or all of
    //    it when islanded, biased by the voltage droop.
    let dispatchable: Vec<&crate::MicrogridAsset> = state
        .assets
        .iter()
        .filter(|a| a.is_dispatchable())
        .collect();
    if !dispatchable.is_empty() {
        let n = count_to_f64(dispatchable.len());
        let target = if grid_connected {
            balance + droop_mw
        } else {
            balance
        };
        let share = (target / n).clamp(0.0, 1.0);
        for asset in &dispatchable {
            let p = share * asset.capacity_mw;
            balance -= p;
            actions.push(crate::ControlAction {
                asset_id: asset.id.clone(),
                kind: asset.kind.clone(),
                target_p_mw: p,
                target_q_mvar: droop_mvar / n,
                new_state_of_charge: None,
                action: "setpoint".to_string(),
            });
        }
    }

    // 4. Loads are reported either way. When the island cannot cover them the
    //    controller sheds up to the configured cap; otherwise the utility
    //    supplies them and the setpoint is simply the full demand.
    let loads: Vec<&crate::MicrogridAsset> = state.assets.iter().filter(|a| a.is_load()).collect();
    let shed_fraction = load_stage(&loads, balance, grid_connected, config, &mut actions);
    balance -= shed_fraction * loads.iter().map(|a| a.capacity_mw).sum::<f64>();

    // Grid-following assets pass the remainder to the utility; an islanded step
    // has no utility, so whatever is left is a real deficit.
    let utility_mw = if grid_connected { balance } else { 0.0 };
    let residual_mw = if grid_connected {
        0.0
    } else {
        balance.max(0.0)
    };

    Ok(ControlDecision {
        balanced: residual_mw < 1e-6,
        actions,
        utility_mw,
        residual_mw,
        load_shed_fraction: shed_fraction,
        grid_forming: !grid_connected,
    })
}

/// Emit load setpoints, shedding if an island cannot cover them.
///
/// Returns the fraction of load that had to be shed, in `[0, 1]`.
fn load_stage(
    loads: &[&crate::MicrogridAsset],
    balance: f64,
    grid_connected: bool,
    config: &ControlConfig,
    actions: &mut Vec<crate::ControlAction>,
) -> f64 {
    let sheddable: f64 = loads.iter().map(|a| a.capacity_mw).sum();
    let mut fraction = 0.0_f64;
    if !grid_connected && balance > 1e-9 && sheddable > 0.0 {
        fraction = (balance / sheddable).clamp(0.0, config.max_shed_fraction);
    }
    for asset in loads {
        actions.push(crate::ControlAction {
            asset_id: asset.id.clone(),
            kind: asset.kind.clone(),
            target_p_mw: -asset.capacity_mw * (1.0 - fraction),
            target_q_mvar: 0.0,
            new_state_of_charge: None,
            action: if fraction > 0.0 { "shed" } else { "hold" }.to_string(),
        });
    }
    fraction
}

/// Reject measurements and tuning the control law cannot use.
fn check_inputs(m: &Measurements, config: &ControlConfig) -> Result<(), WasmError> {
    for (name, value) in [
        ("frequency_hz", m.frequency_hz),
        ("voltage_pu", m.voltage_pu),
    ] {
        if let Some(v) = value {
            if !v.is_finite() || v <= 0.0 {
                return Err(WasmError::new(
                    crate::WasmErrorKind::Control,
                    format!("{name} must be a positive finite number, got {v}"),
                ));
            }
        }
    }
    if let Some(d) = m.demand_mw {
        if !d.is_finite() || d < 0.0 {
            return Err(WasmError::new(
                crate::WasmErrorKind::Control,
                format!("demand_mw must be a non-negative finite number, got {d}"),
            ));
        }
    }
    if !config.interval_hours.is_finite() || config.interval_hours <= 0.0 {
        return Err(WasmError::new(
            crate::WasmErrorKind::Control,
            "interval_hours must be a positive finite number",
        ));
    }
    if !(0.0..=1.0).contains(&config.min_state_of_charge)
        || !(0.0..=1.0).contains(&config.max_state_of_charge)
        || config.min_state_of_charge > config.max_state_of_charge
    {
        return Err(WasmError::new(
            crate::WasmErrorKind::Control,
            "state-of-charge bounds must satisfy 0 <= min <= max <= 1",
        ));
    }
    Ok(())
}

/// Total demand for the step, from the measurement or the state.
fn measured_demand(state: &crate::MicrogridState, m: &Measurements) -> f64 {
    m.demand_mw.unwrap_or_else(|| {
        state
            .assets
            .iter()
            .filter(|a| a.is_load())
            .map(|a| a.capacity_mw)
            .sum()
    })
}

/// Battery target power and resulting state of charge.
///
/// A positive target discharges, limited by the energy above the minimum SoC;
/// a negative target charges, limited by the room below the maximum SoC. The
/// power rating also bounds the energy moved over one control interval.
fn battery_step(
    battery: &crate::MicrogridAsset,
    demand_mw: f64,
    config: &ControlConfig,
) -> (f64, f64) {
    let soc = battery.state_of_charge.unwrap_or(0.5).clamp(0.0, 1.0);
    let capacity_mwh = battery.energy_capacity_mwh();
    let power_limit = battery.capacity_mw;
    let headroom = (soc - config.min_state_of_charge).max(0.0) * capacity_mwh;
    let room = (config.max_state_of_charge - soc).max(0.0) * capacity_mwh;

    if demand_mw > 0.0 {
        let mw = demand_mw
            .min(power_limit)
            .min(headroom / config.interval_hours);
        let new_soc = (soc - mw * config.interval_hours / capacity_mwh).clamp(0.0, 1.0);
        (mw, new_soc)
    } else {
        let mw = (-demand_mw)
            .min(power_limit)
            .min(room / config.interval_hours);
        let new_soc = (soc + mw * config.interval_hours / capacity_mwh).clamp(0.0, 1.0);
        (-mw, new_soc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MicrogridAsset, MicrogridState, WasmErrorKind};

    fn asset(id: &str, kind: &str, output: f64, capacity: f64, soc: Option<f64>) -> MicrogridAsset {
        MicrogridAsset {
            id: id.to_string(),
            kind: kind.to_string(),
            output_mw: output,
            state_of_charge: soc,
            capacity_mw: capacity,
            energy_capacity_mwh: None,
        }
    }

    fn state(assets: Vec<MicrogridAsset>, grid_connected: bool) -> MicrogridState {
        MicrogridState {
            assets,
            grid_connected,
        }
    }

    fn step(s: &MicrogridState, m: &Measurements, c: &ControlConfig) -> ControlDecision {
        microgrid_step(s, m, c).expect("control step")
    }

    fn action<'a>(d: &'a ControlDecision, id: &str) -> &'a crate::ControlAction {
        d.actions.iter().find(|a| a.asset_id == id).expect("action")
    }

    #[test]
    fn a_balanced_grid_connected_step_needs_no_shedding() {
        let s = state(
            vec![
                asset("s1", "solar", 5.0, 10.0, None),
                asset("b1", "battery", 0.0, 5.0, Some(0.8)),
                asset("g1", "diesel", 0.0, 10.0, None),
                asset("l1", "load", 0.0, 10.0, None),
            ],
            true,
        );
        let d = step(
            &s,
            &Measurements {
                demand_mw: Some(8.0),
                ..Measurements::default()
            },
            &ControlConfig::default(),
        );
        assert!(d.balanced, "grid-connected steps always balance");
        assert!(d.load_shed_fraction.abs() < 1e-12);
        assert!(!d.grid_forming);
        assert!(d.actions.iter().all(|a| a.action != "shed"));
    }

    #[test]
    fn an_islanded_deficit_sheds_load() {
        let s = state(
            vec![
                asset("b1", "battery", 0.0, 5.0, Some(0.5)),
                asset("g1", "diesel", 0.0, 2.0, None),
                asset("l1", "load", 0.0, 20.0, None),
            ],
            false,
        );
        let d = step(
            &s,
            &Measurements {
                demand_mw: Some(20.0),
                ..Measurements::default()
            },
            &ControlConfig::default(),
        );
        assert!(d.grid_forming, "an islanded step sets the reference");
        assert!(d.load_shed_fraction > 0.0, "some load is shed");
        assert!(d.load_shed_fraction <= 0.5, "shedding respects the cap");
    }

    #[test]
    fn renewables_are_held_at_their_output() {
        let s = state(
            vec![
                asset("s1", "solar", 4.0, 10.0, None),
                asset("b1", "battery", 0.0, 10.0, Some(0.9)),
                asset("l1", "load", 0.0, 4.0, None),
            ],
            true,
        );
        let d = step(
            &s,
            &Measurements {
                demand_mw: Some(4.0),
                ..Measurements::default()
            },
            &ControlConfig::default(),
        );
        let solar = action(&d, "s1");
        assert!(
            (solar.target_p_mw - 4.0).abs() < 1e-12,
            "solar is not curtailed"
        );
        assert_eq!(solar.action, "hold");
    }

    #[test]
    fn the_battery_respects_its_energy_headroom() {
        let s = state(
            vec![
                asset("b1", "battery", 0.0, 100.0, Some(0.11)),
                asset("l1", "load", 0.0, 100.0, None),
            ],
            false,
        );
        let d = step(
            &s,
            &Measurements {
                demand_mw: Some(100.0),
                ..Measurements::default()
            },
            &ControlConfig {
                min_state_of_charge: 0.1,
                interval_hours: 1.0,
                ..ControlConfig::default()
            },
        );
        let battery = action(&d, "b1");
        // 1% of headroom on a 100 MWh battery is 1 MWh, i.e. 1 MW for 1 h.
        assert!(
            (battery.target_p_mw - 1.0).abs() < 1e-6,
            "discharge limited to headroom, got {}",
            battery.target_p_mw
        );
        assert!(battery.new_state_of_charge.unwrap() >= 0.1);
    }

    #[test]
    fn a_low_frequency_asks_the_battery_for_more() {
        let s = state(
            vec![
                asset("b1", "battery", 0.0, 10.0, Some(0.9)),
                asset("g1", "diesel", 0.0, 10.0, None),
            ],
            true,
        );
        let sag = step(
            &s,
            &Measurements {
                frequency_hz: Some(59.0),
                demand_mw: Some(0.0),
                ..Measurements::default()
            },
            &ControlConfig::default(),
        );
        assert!(
            action(&sag, "b1").target_p_mw > 0.0,
            "a 1 Hz frequency sag must produce a discharge request"
        );
    }

    #[test]
    fn bad_measurements_are_rejected_with_the_control_kind() {
        let s = state(vec![asset("l1", "load", 0.0, 10.0, None)], true);
        for bad in [
            &Measurements {
                frequency_hz: Some(f64::NAN),
                ..Measurements::default()
            },
            &Measurements {
                voltage_pu: Some(-1.0),
                ..Measurements::default()
            },
            &Measurements {
                demand_mw: Some(f64::INFINITY),
                ..Measurements::default()
            },
        ] {
            let err = microgrid_step(&s, bad, &ControlConfig::default()).unwrap_err();
            assert_eq!(err.kind, WasmErrorKind::Control);
        }
    }

    #[test]
    fn inverted_soc_bounds_are_rejected() {
        let s = state(vec![asset("l1", "load", 0.0, 10.0, None)], true);
        let err = microgrid_step(
            &s,
            &Measurements::default(),
            &ControlConfig {
                min_state_of_charge: 0.9,
                max_state_of_charge: 0.2,
                ..ControlConfig::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Control);
    }
}

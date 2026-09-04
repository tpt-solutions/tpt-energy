//! Generator definition.

use serde::{Deserialize, Serialize};

use crate::curves::{CostCurve, HeatRateCurve, PowerCurve};

/// Type of generating unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum GeneratorType {
    /// Fossil / gas / coal-fired thermal unit.
    Thermal,
    /// Conventional hydroelectric unit.
    Hydro,
    /// Onshore / offshore wind turbine.
    Wind,
    /// Photovoltaic solar plant.
    Solar,
    /// Nuclear steam turbine.
    Nuclear,
    /// Geothermal plant.
    Geothermal,
    /// Other / unspecified.
    Other,
}

impl Default for GeneratorType {
    fn default() -> Self {
        Self::Thermal
    }
}

/// A generating unit connected to a bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generator {
    /// Unique numeric generator id.
    pub id: usize,

    /// Human-readable generator name.
    pub name: String,

    /// Bus id where the unit is connected.
    pub bus_id: usize,

    /// Type of generation technology.
    #[serde(rename = "type")]
    pub generator_type: GeneratorType,

    /// Maximum active power output in MW.
    pub p_max_mw: f64,

    /// Minimum active power output in MW.
    #[serde(default)]
    pub p_min_mw: f64,

    /// Maximum reactive power output in MVAr.
    #[serde(default = "default_q_max")]
    pub q_max_mvar: f64,

    /// Minimum reactive power output in MVAr.
    #[serde(default = "default_q_min")]
    pub q_min_mvar: f64,

    /// Scheduled active power output in MW.
    #[serde(default)]
    pub p_schedule_mw: f64,

    /// Scheduled voltage setpoint in per-unit.
    #[serde(default = "default_v_setpoint")]
    pub voltage_setpoint_pu: f64,

    /// Cost curve (piecewise linear or polynomial).
    #[serde(default)]
    pub cost_curve: Option<CostCurve>,

    /// Heat rate curve (input MMBtu per output MWh).
    #[serde(default)]
    pub heat_rate: Option<HeatRateCurve>,

    /// Power curve (for wind / solar).
    #[serde(default)]
    pub power_curve: Option<PowerCurve>,

    /// Whether the unit is in service.
    #[serde(default = "default_in_service")]
    pub in_service: bool,
}

fn default_q_max() -> f64 {
    9999.0
}
fn default_q_min() -> f64 {
    -9999.0
}
fn default_v_setpoint() -> f64 {
    1.0
}
fn default_in_service() -> bool {
    true
}

impl Generator {
    /// Construct a new generator. Use [`with_cost_curve`](Self::with_cost_curve),
    /// [`with_voltage_setpoint`](Self::with_voltage_setpoint), etc. to
    /// customize.
    pub fn new(
        id: usize,
        name: impl Into<String>,
        generator_type: GeneratorType,
        p_max_mw: f64,
        p_min_mw: f64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            bus_id: 0,
            generator_type,
            p_max_mw,
            p_min_mw,
            q_max_mvar: default_q_max(),
            q_min_mvar: default_q_min(),
            p_schedule_mw: 0.0,
            voltage_setpoint_pu: default_v_setpoint(),
            cost_curve: None,
            heat_rate: None,
            power_curve: None,
            in_service: true,
        }
    }

    /// Connect this generator to a bus.
    pub fn at_bus(mut self, bus_id: usize) -> Self {
        self.bus_id = bus_id;
        self
    }

    /// Set the reactive power capability range.
    pub fn with_reactive_limits(mut self, q_min_mvar: f64, q_max_mvar: f64) -> Self {
        self.q_min_mvar = q_min_mvar;
        self.q_max_mvar = q_max_mvar;
        self
    }

    /// Set the scheduled active power output.
    pub fn with_p_schedule(mut self, p_schedule_mw: f64) -> Self {
        self.p_schedule_mw = p_schedule_mw;
        self
    }

    /// Set the voltage setpoint in per-unit.
    pub fn with_voltage_setpoint(mut self, voltage_setpoint_pu: f64) -> Self {
        self.voltage_setpoint_pu = voltage_setpoint_pu;
        self
    }

    /// Attach a cost curve.
    pub fn with_cost_curve(mut self, cost_curve: CostCurve) -> Self {
        self.cost_curve = Some(cost_curve);
        self
    }

    /// Attach a heat-rate curve.
    pub fn with_heat_rate(mut self, heat_rate: HeatRateCurve) -> Self {
        self.heat_rate = Some(heat_rate);
        self
    }

    /// Attach a power curve (wind/solar).
    pub fn with_power_curve(mut self, power_curve: PowerCurve) -> Self {
        self.power_curve = Some(power_curve);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_construction() {
        let g = Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 10.0).at_bus(1);
        assert_eq!(g.id, 1);
        assert_eq!(g.bus_id, 1);
        assert_eq!(g.generator_type, GeneratorType::Thermal);
        assert!((g.p_max_mw - 100.0).abs() < 1e-12);
    }

    #[test]
    fn generator_serde_roundtrip() {
        let g = Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 10.0)
            .at_bus(1)
            .with_voltage_setpoint(1.05);
        let s = serde_json::to_string(&g).unwrap();
        let d: Generator = serde_json::from_str(&s).unwrap();
        assert_eq!(d.id, 1);
        assert!((d.voltage_setpoint_pu - 1.05).abs() < 1e-12);
    }
}

//! # tpt-nrg-carbon
//!
//! Carbon intensity of electricity generation: kg CO₂ per MWh from the
//! generator mix.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use tpt_nrg_core::{EnergySystem, GeneratorType};

/// Emission factor in kg CO₂ per MWh by fuel type.
///
/// These are typical lifecycle values (combustion only for fossil fuels,
/// lifecycle for renewables). The exact source (e.g. EPA eGRID, IPCC) is a
/// user choice; defaults are reasonable midpoints.
pub fn emission_factor_kg_per_mwh(fuel: GeneratorType) -> f64 {
    match fuel {
        GeneratorType::Thermal => 900.0, // gas / coal average
        GeneratorType::Hydro => 4.0,      // reservoir, lifecycle
        GeneratorType::Wind => 11.0,
        GeneratorType::Solar => 45.0,
        GeneratorType::Nuclear => 12.0,
        GeneratorType::Geothermal => 38.0,
        GeneratorType::Other => 500.0,
    }
}

/// Carbon intensity of the current dispatch (kg CO₂ per MWh) computed as
/// the weighted average of generator emission factors by their current
/// scheduled output.
pub fn carbon_intensity(system: &EnergySystem) -> f64 {
    let mut total_mwh = 0.0;
    let mut total_co2 = 0.0;
    for g in &system.generators {
        if !g.in_service {
            continue;
        }
        let p = g.p_schedule_mw;
        let ef = emission_factor_kg_per_mwh(g.generator_type);
        total_mwh += p;
        total_co2 += p * ef;
    }
    if total_mwh == 0.0 {
        0.0
    } else {
        total_co2 / total_mwh
    }
}

/// Carbon emissions (tonnes CO₂) over a period at a given average output
/// and duration.
pub fn total_emissions_tonnes(
    system: &EnergySystem,
    duration_h: f64,
) -> f64 {
    let total_kg: f64 = system
        .generators
        .iter()
        .filter(|g| g.in_service)
        .map(|g| g.p_schedule_mw * duration_h * emission_factor_kg_per_mwh(g.generator_type))
        .sum();
    total_kg / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Bus, BusType, Generator};

    fn mixed_system() -> EnergySystem {
        let mut s = EnergySystem::new("c", "C", 100.0, 60.0);
        s.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        s.add_bus(Bus::new(2, "B2", BusType::Pv)).unwrap();
        s.add_bus(Bus::new(3, "B3", BusType::Pv)).unwrap();
        s.add_generator(
            Generator::new(1, "G_gas", GeneratorType::Thermal, 100.0, 0.0)
                .at_bus(1)
                .with_p_schedule(50.0),
        )
        .unwrap();
        s.add_generator(
            Generator::new(2, "G_wind", GeneratorType::Wind, 100.0, 0.0)
                .at_bus(2)
                .with_p_schedule(50.0),
        )
        .unwrap();
        s.add_generator(
            Generator::new(3, "G_solar", GeneratorType::Solar, 50.0, 0.0)
                .at_bus(3)
                .with_p_schedule(50.0),
        )
        .unwrap();
        s
    }

    #[test]
    fn intensity_weighted_average() {
        let s = mixed_system();
        let ci = carbon_intensity(&s);
        // 50 MW gas (900) + 50 MW wind (11) + 50 MW solar (45) = 150 MW total
        // CO2 = 50*900 + 50*11 + 50*45 = 47800 kg/h
        // CI = 47800/150 = 318.7 kg/MWh
        assert!((ci - 318.67).abs() < 1.0, "ci = {ci}");
    }

    #[test]
    fn total_emissions_for_one_hour() {
        let s = mixed_system();
        let e = total_emissions_tonnes(&s, 1.0);
        // 47800 kg = 47.8 t
        assert!((e - 47.8).abs() < 0.1, "e = {e}");
    }
}

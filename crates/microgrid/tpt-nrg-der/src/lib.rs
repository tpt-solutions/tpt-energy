//! # tpt-nrg-der
//!
//! Distributed energy resource (DER) models and microgrid controllers.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Distributed energy resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DerAsset {
    /// Rooftop or utility-scale solar PV.
    Solar {
        /// Nameplate AC capacity in MW.
        capacity_mw: f64,
        /// Current output (0-1 of capacity).
        output_fraction: f64,
    },
    /// Wind turbine.
    Wind {
        /// Nameplate capacity in MW.
        capacity_mw: f64,
        /// Current output (0-1 of capacity).
        output_fraction: f64,
    },
    /// Battery storage (see `tpt-nrg-battery` for full model).
    Battery {
        /// Power rating in MW (charge/discharge).
        power_rating_mw: f64,
        /// Energy capacity in MWh.
        energy_capacity_mwh: f64,
        /// Current SoC (0-1).
        state_of_charge: f64,
    },
    /// Controllable load.
    Load {
        /// Rated load in MW.
        rated_mw: f64,
        /// Current demand (0-1 of rated).
        demand_fraction: f64,
    },
    /// Diesel genset.
    Diesel {
        /// Rated power in MW.
        rated_mw: f64,
        /// Current output in MW.
        output_mw: f64,
        /// Minimum stable output in MW.
        min_output_mw: f64,
    },
}

impl DerAsset {
    /// Net power contribution of the asset (MW, positive = supply, negative
    /// = demand).
    pub fn net_power_mw(&self) -> f64 {
        match self {
            DerAsset::Solar {
                capacity_mw,
                output_fraction,
            } => capacity_mw * output_fraction,
            DerAsset::Wind {
                capacity_mw,
                output_fraction,
            } => capacity_mw * output_fraction,
            DerAsset::Battery {
                power_rating_mw,
                energy_capacity_mwh,
                state_of_charge,
            } => {
                // For a simplified view, report power as the rated charge rate
                // when SoC > 50% and rated discharge when SoC < 50%, capped by
                // the energy actually available over a nominal 1-hour dispatch
                // interval so the asset can't discharge more than it holds.
                let available_mw = state_of_charge * energy_capacity_mwh;
                if *state_of_charge > 0.5 {
                    power_rating_mw.min(available_mw)
                } else if *state_of_charge > 0.2 {
                    (*power_rating_mw * 0.5).min(available_mw)
                } else {
                    0.0
                }
            }
            DerAsset::Load {
                rated_mw,
                demand_fraction,
            } => -rated_mw * demand_fraction,
            DerAsset::Diesel { output_mw, .. } => *output_mw,
        }
    }
}

/// Control strategy for the microgrid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStrategy {
    /// Grid-following inverter (requires external grid for V/f reference).
    GridFollowing,
    /// Grid-forming inverter (provides V/f reference for islanded operation).
    GridForming,
    /// Droop-based P/f and Q/V control.
    DroopControl,
}

/// Microgrid controller state.
#[derive(Debug, Clone)]
pub struct MicrogridController {
    /// Whether the microgrid is currently connected to the main grid.
    pub grid_connected: bool,
    /// DER assets in the microgrid.
    pub assets: Vec<DerAsset>,
    /// Active control strategy.
    pub control_strategy: ControlStrategy,
    /// Total load in MW.
    pub total_load_mw: f64,
}

impl MicrogridController {
    /// Construct a new controller.
    pub fn new(control_strategy: ControlStrategy, total_load_mw: f64) -> Self {
        Self {
            grid_connected: true,
            assets: Vec::new(),
            control_strategy,
            total_load_mw,
        }
    }

    /// Add a DER asset.
    pub fn add_asset(&mut self, asset: DerAsset) {
        self.assets.push(asset);
    }

    /// Total generation (MW) from the assets.
    pub fn total_generation_mw(&self) -> f64 {
        self.assets.iter().map(|a| a.net_power_mw().max(0.0)).sum()
    }

    /// Total load including DER loads.
    pub fn total_demand_mw(&self) -> f64 {
        let der_loads: f64 = self
            .assets
            .iter()
            .map(|a| (-a.net_power_mw()).max(0.0))
            .sum();
        self.total_load_mw + der_loads
    }

    /// Net balance (generation - demand). Positive = surplus, negative =
    /// deficit.
    pub fn net_balance_mw(&self) -> f64 {
        self.total_generation_mw() - self.total_demand_mw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_net_power() {
        let s = DerAsset::Solar {
            capacity_mw: 10.0,
            output_fraction: 0.5,
        };
        assert!((s.net_power_mw() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn controller_balance() {
        let mut c = MicrogridController::new(ControlStrategy::GridFollowing, 5.0);
        c.add_asset(DerAsset::Solar {
            capacity_mw: 10.0,
            output_fraction: 0.6,
        });
        c.add_asset(DerAsset::Load {
            rated_mw: 1.0,
            demand_fraction: 1.0,
        });
        // Generation 6 MW, demand 5+1=6 MW → balance 0
        assert!(c.net_balance_mw().abs() < 1e-9);
    }
}

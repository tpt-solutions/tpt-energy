//! # tpt-nrg-vpp
//!
//! Virtual power plant: aggregation of DER assets for market participation.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Aggregation model for a VPP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationModel {
    /// Sum of all asset capacities and outputs.
    Sum,
    /// Diversity-adjusted: outputs are correlated, so total is less than
    /// the sum.
    DiversityAdjusted {
        /// Diversity factor in (0, 1].
        factor: f64,
    },
}

/// Virtual power plant: aggregator over a fleet of DER assets.
#[derive(Debug, Clone)]
pub struct VirtualPowerPlant {
    /// Per-asset nameplate capacity (MW).
    pub asset_capacities_mw: Vec<f64>,
    /// Per-asset current output (MW).
    pub asset_outputs_mw: Vec<f64>,
    /// Per-asset upward flexibility (MW).
    pub asset_up_flex_mw: Vec<f64>,
    /// Per-asset downward flexibility (MW).
    pub asset_down_flex_mw: Vec<f64>,
    /// Aggregation model.
    pub aggregation_model: AggregationModel,
}

impl VirtualPowerPlant {
    /// Construct a new VPP from a list of per-asset (capacity, current
    /// output, up-flex, down-flex).
    pub fn new(assets: Vec<(f64, f64, f64, f64)>, aggregation_model: AggregationModel) -> Self {
        let mut caps = Vec::new();
        let mut outs = Vec::new();
        let mut up = Vec::new();
        let mut down = Vec::new();
        for (c, o, u, d) in assets {
            caps.push(c);
            outs.push(o);
            up.push(u);
            down.push(d);
        }
        Self {
            asset_capacities_mw: caps,
            asset_outputs_mw: outs,
            asset_up_flex_mw: up,
            asset_down_flex_mw: down,
            aggregation_model,
        }
    }

    /// Total nameplate capacity (MW).
    pub fn total_capacity_mw(&self) -> f64 {
        self.asset_capacities_mw.iter().sum()
    }

    /// Total current output (MW).
    pub fn total_output_mw(&self) -> f64 {
        self.apply_aggregation(self.asset_outputs_mw.iter().sum())
    }

    /// Total upward flexible capacity (MW).
    pub fn flexible_capacity_up_mw(&self) -> f64 {
        self.apply_aggregation(self.asset_up_flex_mw.iter().sum())
    }

    /// Total downward flexible capacity (MW).
    pub fn flexible_capacity_down_mw(&self) -> f64 {
        self.apply_aggregation(self.asset_down_flex_mw.iter().sum())
    }

    /// Pro-rata dispatch: increase each asset's output by `delta_mw`,
    /// respecting each asset's upward flexibility. Returns the dispatch
    /// plan and the unfulfilled portion.
    pub fn dispatch_assets(&self, delta_mw: f64) -> DispatchPlan {
        let total_up = self.asset_up_flex_mw.iter().sum::<f64>();
        if total_up <= 0.0 {
            return DispatchPlan {
                per_asset_delta_mw: vec![0.0; self.asset_capacities_mw.len()],
                delivered_mw: 0.0,
                unfulfilled_mw: delta_mw,
            };
        }
        let scale = (delta_mw / total_up).clamp(0.0, 1.0);
        let per_asset: Vec<f64> = self.asset_up_flex_mw.iter().map(|u| u * scale).collect();
        let delivered = per_asset.iter().sum();
        DispatchPlan {
            per_asset_delta_mw: per_asset,
            delivered_mw: delivered,
            unfulfilled_mw: delta_mw - delivered,
        }
    }

    fn apply_aggregation(&self, sum: f64) -> f64 {
        match self.aggregation_model {
            AggregationModel::Sum => sum,
            AggregationModel::DiversityAdjusted { factor } => sum * factor,
        }
    }
}

/// Result of a dispatch request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchPlan {
    /// Per-asset delta in MW (positive = increase output, negative =
    /// decrease).
    pub per_asset_delta_mw: Vec<f64>,
    /// Total MW delivered.
    pub delivered_mw: f64,
    /// MW that could not be delivered (positive when constrained).
    pub unfulfilled_mw: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_sums() {
        let v = VirtualPowerPlant::new(
            vec![(10.0, 5.0, 2.0, 2.0), (20.0, 10.0, 5.0, 3.0)],
            AggregationModel::Sum,
        );
        assert!((v.total_capacity_mw() - 30.0).abs() < 1e-9);
        assert!((v.total_output_mw() - 15.0).abs() < 1e-9);
        assert!((v.flexible_capacity_up_mw() - 7.0).abs() < 1e-9);
    }

    #[test]
    fn diversity_adjustment() {
        let v = VirtualPowerPlant::new(
            vec![(10.0, 5.0, 2.0, 2.0), (20.0, 10.0, 5.0, 3.0)],
            AggregationModel::DiversityAdjusted { factor: 0.9 },
        );
        assert!((v.total_output_mw() - 13.5).abs() < 1e-9);
    }

    #[test]
    fn dispatch_pro_rata() {
        let v = VirtualPowerPlant::new(
            vec![(10.0, 5.0, 2.0, 2.0), (20.0, 10.0, 4.0, 3.0)],
            AggregationModel::Sum,
        );
        let plan = v.dispatch_assets(3.0);
        // total up-flex = 6 MW; scale = 0.5; per-asset = 1.0, 2.0
        assert!((plan.delivered_mw - 3.0).abs() < 1e-9);
        assert!((plan.unfulfilled_mw - 0.0).abs() < 1e-9);
        assert!((plan.per_asset_delta_mw[0] - 1.0).abs() < 1e-9);
        assert!((plan.per_asset_delta_mw[1] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn dispatch_caps_at_total_flex() {
        let v = VirtualPowerPlant::new(
            vec![(10.0, 5.0, 2.0, 2.0), (20.0, 10.0, 4.0, 3.0)],
            AggregationModel::Sum,
        );
        let plan = v.dispatch_assets(100.0);
        // total up-flex = 6 MW; can't deliver 100.
        assert!((plan.delivered_mw - 6.0).abs() < 1e-9);
        assert!((plan.unfulfilled_mw - 94.0).abs() < 1e-9);
    }
}

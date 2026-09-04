//! Load definition.

use serde::{Deserialize, Serialize};

/// A load attached to a bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Load {
    /// Unique numeric load id.
    pub id: usize,

    /// Human-readable load name.
    pub name: String,

    /// Bus id where the load is connected.
    pub bus_id: usize,

    /// Active power consumption in MW (positive = consumption).
    pub p_mw: f64,

    /// Reactive power consumption in MVAr (positive = consumption).
    pub q_mvar: f64,

    /// Whether the load is in service.
    #[serde(default = "default_in_service")]
    pub in_service: bool,
}

fn default_in_service() -> bool {
    true
}

impl Load {
    /// Construct a new load.
    pub fn new(id: usize, name: impl Into<String>, bus_id: usize, p_mw: f64, q_mvar: f64) -> Self {
        Self {
            id,
            name: name.into(),
            bus_id,
            p_mw,
            q_mvar,
            in_service: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_construction() {
        let l = Load::new(1, "L1", 2, 30.0, 10.0);
        assert_eq!(l.bus_id, 2);
        assert!((l.p_mw - 30.0).abs() < 1e-12);
    }
}

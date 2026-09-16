//! # tpt-nrg-protection
//!
//! Relay coordination and protection zone modelling.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// A protective relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relay {
    /// Relay id.
    pub id: usize,
    /// Branch id the relay is on.
    pub branch_id: usize,
    /// "From" or "to" end of the branch.
    pub end: RelayEnd,
    /// Pickup current in pu.
    pub pickup_pu: f64,
    /// Inverse-time characteristic time multiplier (TMS).
    pub time_multiplier: f64,
    /// Curve type.
    pub curve: RelayCurve,
    /// Time delay added (for coordination).
    pub time_delay_s: f64,
}

/// Which end of a branch a relay protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayEnd {
    /// Sending end.
    From,
    /// Receiving end.
    To,
}

/// Inverse-time overcurrent relay characteristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayCurve {
    /// Standard inverse (IEC 60255-151).
    StandardInverse,
    /// Very inverse.
    VeryInverse,
    /// Extremely inverse.
    ExtremelyInverse,
    /// Definite time.
    DefiniteTime,
}

impl RelayCurve {
    /// Trip time (s) for the given current multiple `i = I/Ipickup`.
    ///
    /// IEC 60255-151 characteristic `t = k / (i^α − 1)` with
    /// (k, α) = (0.14, 0.02) standard inverse, (13.5, 1) very inverse and
    /// (80, 2) extremely inverse. Returns `INFINITY` at or below pickup.
    pub fn trip_time(&self, i: f64) -> f64 {
        if i <= 1.0 {
            return f64::INFINITY;
        }
        match self {
            RelayCurve::StandardInverse => 0.14 / (i.powf(0.02) - 1.0),
            RelayCurve::VeryInverse => 13.5 / (i - 1.0),
            RelayCurve::ExtremelyInverse => 80.0 / (i * i - 1.0),
            RelayCurve::DefiniteTime => 0.1,
        }
    }
}

impl Relay {
    /// Compute the operating time (s) for a fault current `i_fault_pu`.
    pub fn operating_time(&self, i_fault_pu: f64) -> f64 {
        let mult = i_fault_pu / self.pickup_pu;
        if mult < 1.0 {
            return f64::INFINITY;
        }
        self.time_multiplier * self.curve.trip_time(mult) + self.time_delay_s
    }
}

/// Coordination check: ensure that `primary` trips before `backup` for all
/// fault currents. The time margin is typically 0.2-0.4 s.
pub fn check_coordination(primary: &Relay, backup: &Relay, fault_currents_pu: &[f64]) -> bool {
    for &i in fault_currents_pu {
        let t_primary = primary.operating_time(i);
        let t_backup = backup.operating_time(i);
        if t_backup - t_primary < 0.2 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relay(pickup: f64, tms: f64, curve: RelayCurve) -> Relay {
        Relay {
            id: 1,
            branch_id: 1,
            end: RelayEnd::From,
            pickup_pu: pickup,
            time_multiplier: tms,
            curve,
            time_delay_s: 0.0,
        }
    }

    #[test]
    fn definite_time_trips() {
        let r = relay(1.0, 1.0, RelayCurve::DefiniteTime);
        assert!((r.operating_time(5.0) - 0.1).abs() < 1e-9);
    }

    #[test]
    fn inverse_faster_at_higher_current() {
        let r = relay(1.0, 0.5, RelayCurve::StandardInverse);
        assert!(r.operating_time(10.0) < r.operating_time(2.0));
    }

    #[test]
    fn no_trip_below_pickup() {
        let r = relay(1.0, 0.5, RelayCurve::StandardInverse);
        assert!(r.operating_time(0.5).is_infinite());
    }

    #[test]
    fn coordination_pass() {
        let primary = relay(1.0, 0.1, RelayCurve::StandardInverse);
        let mut backup = relay(1.0, 0.5, RelayCurve::StandardInverse);
        backup.time_delay_s = 0.3;
        let currents = vec![2.0, 5.0, 10.0, 20.0];
        assert!(check_coordination(&primary, &backup, &currents));
    }

    #[test]
    fn iec_60255_reference_trip_times() {
        // Published IEC 60255-151 trip times at TMS = 1.
        let si_2 = RelayCurve::StandardInverse.trip_time(2.0);
        assert!((si_2 - 10.03).abs() < 0.05, "SI M=2 = {si_2}");
        assert!((RelayCurve::VeryInverse.trip_time(2.0) - 13.5).abs() < 1e-9);
        assert!(
            (RelayCurve::ExtremelyInverse.trip_time(2.0) - 80.0 / 3.0).abs() < 1e-9
        );
        assert!((RelayCurve::VeryInverse.trip_time(5.0) - 3.375).abs() < 1e-9);
        assert!(
            (RelayCurve::ExtremelyInverse.trip_time(5.0) - 10.0 / 3.0).abs() < 1e-9
        );
        assert!(RelayCurve::StandardInverse.trip_time(1.0).is_infinite());
    }
}

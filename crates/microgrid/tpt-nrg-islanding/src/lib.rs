//! # tpt-nrg-islanding
//!
//! Loss-of-mains detection, controlled transition to islanded operation,
//! and resynchronization with the main grid.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Detection thresholds for loss-of-mains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandingDetector {
    /// Under-voltage threshold (pu, e.g. 0.88).
    pub under_voltage_pu: f64,
    /// Over-voltage threshold (pu, e.g. 1.10).
    pub over_voltage_pu: f64,
    /// Under-frequency threshold (Hz, e.g. 59.5).
    pub under_frequency_hz: f64,
    /// Over-frequency threshold (Hz, e.g. 60.5).
    pub over_frequency_hz: f64,
    /// Rate-of-change-of-frequency threshold (Hz/s, e.g. 0.5).
    pub rocof_threshold_hz_per_s: f64,
    /// Detection time window (seconds).
    pub time_window_s: f64,
}

impl Default for IslandingDetector {
    fn default() -> Self {
        // Defaults based on IEEE 1547 category II.
        Self {
            under_voltage_pu: 0.88,
            over_voltage_pu: 1.10,
            under_frequency_hz: 59.5,
            over_frequency_hz: 60.5,
            rocof_threshold_hz_per_s: 0.5,
            time_window_s: 0.16,
        }
    }
}

impl IslandingDetector {
    /// Construct a new detector with default IEEE 1547 thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decide whether islanding has occurred given the measured voltage
    /// (pu), frequency (Hz), and rate of change of frequency (Hz/s).
    pub fn detect_islanding(
        &self,
        voltage_pu: f64,
        frequency_hz: f64,
        rocof_hz_per_s: f64,
    ) -> bool {
        if voltage_pu < self.under_voltage_pu || voltage_pu > self.over_voltage_pu {
            return true;
        }
        if frequency_hz < self.under_frequency_hz || frequency_hz > self.over_frequency_hz {
            return true;
        }
        if rocof_hz_per_s.abs() > self.rocof_threshold_hz_per_s {
            return true;
        }
        false
    }
}

/// Result of a controlled transition to islanded operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionResult {
    /// Whether the transition was successful.
    pub success: bool,
    /// Load shed in MW (positive value).
    pub load_shed_mw: f64,
    /// Storage dispatch (positive = discharge, negative = charge) in MW.
    pub storage_dispatch_mw: f64,
    /// New voltage setpoint (pu).
    pub new_voltage_pu: f64,
    /// New frequency setpoint (Hz).
    pub new_frequency_hz: f64,
}

/// Transition a microgrid from grid-connected to islanded operation.
///
/// The strategy is: detect islanding → shed non-critical load → dispatch
/// storage → form a new V/f reference with grid-forming inverters.
pub fn transition_to_island(
    total_load_mw: f64,
    available_generation_mw: f64,
    storage_available_mw: f64,
    nominal_voltage_pu: f64,
    nominal_frequency_hz: f64,
) -> TransitionResult {
    let net = available_generation_mw + storage_available_mw - total_load_mw;
    if available_generation_mw >= total_load_mw {
        // Generation alone covers the load: keep storage in reserve.
        return TransitionResult {
            success: true,
            load_shed_mw: 0.0,
            storage_dispatch_mw: 0.0,
            new_voltage_pu: nominal_voltage_pu,
            new_frequency_hz: nominal_frequency_hz,
        };
    }
    if net >= 0.0 {
        // Generation is short; storage covers exactly the shortfall.
        let shortfall = total_load_mw - available_generation_mw;
        return TransitionResult {
            success: true,
            load_shed_mw: 0.0,
            storage_dispatch_mw: shortfall.min(storage_available_mw),
            new_voltage_pu: nominal_voltage_pu,
            new_frequency_hz: nominal_frequency_hz,
        };
    }
    // Insufficient generation: shed load to match.
    let required_shed = -net;
    let total_after_shed = total_load_mw - required_shed;
    if total_after_shed < 0.0 {
        return TransitionResult {
            success: false,
            load_shed_mw: total_load_mw,
            storage_dispatch_mw: storage_available_mw,
            new_voltage_pu: nominal_voltage_pu,
            new_frequency_hz: nominal_frequency_hz,
        };
    }
    TransitionResult {
        success: true,
        load_shed_mw: required_shed,
        storage_dispatch_mw: storage_available_mw,
        new_voltage_pu: nominal_voltage_pu,
        new_frequency_hz: nominal_frequency_hz,
    }
}

/// Result of a resynchronization attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Whether resynchronization succeeded.
    pub success: bool,
    /// Final voltage magnitude error (pu).
    pub voltage_error_pu: f64,
    /// Final voltage angle error (rad).
    pub angle_error_rad: f64,
    /// Final frequency error (Hz).
    pub frequency_error_hz: f64,
}

/// Resynchronize the microgrid with the main grid by matching voltage,
/// frequency, and phase angle.
pub fn resynchronize(
    v_microgrid: f64,
    v_grid: f64,
    angle_microgrid: f64,
    angle_grid: f64,
    freq_microgrid: f64,
    freq_grid: f64,
    v_tolerance_pu: f64,
    angle_tolerance_rad: f64,
    freq_tolerance_hz: f64,
) -> SyncResult {
    let v_err = (v_microgrid - v_grid).abs();
    // Angles are cyclic: wrap the error onto [0, π] so that e.g. a
    // freewheeling island at 2π vs the grid at 0 matches.
    let two_pi = 2.0 * std::f64::consts::PI;
    let raw_a_err = (angle_microgrid - angle_grid).abs() % two_pi;
    let a_err = raw_a_err.min(two_pi - raw_a_err);
    let f_err = (freq_microgrid - freq_grid).abs();
    SyncResult {
        success: v_err < v_tolerance_pu
            && a_err < angle_tolerance_rad
            && f_err < freq_tolerance_hz,
        voltage_error_pu: v_err,
        angle_error_rad: a_err,
        frequency_error_hz: f_err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_normal() {
        let d = IslandingDetector::new();
        assert!(!d.detect_islanding(1.0, 60.0, 0.1));
    }

    #[test]
    fn detect_under_voltage() {
        let d = IslandingDetector::new();
        assert!(d.detect_islanding(0.7, 60.0, 0.1));
    }

    #[test]
    fn detect_over_frequency() {
        let d = IslandingDetector::new();
        assert!(d.detect_islanding(1.0, 61.0, 0.1));
    }

    #[test]
    fn detect_rocof() {
        let d = IslandingDetector::new();
        assert!(d.detect_islanding(1.0, 60.0, 2.0));
    }

    #[test]
    fn transition_surplus() {
        let r = transition_to_island(5.0, 4.0, 3.0, 1.0, 60.0);
        assert!(r.success);
        assert_eq!(r.load_shed_mw, 0.0);
    }

    #[test]
    fn transition_deficit_sheds_load() {
        let r = transition_to_island(10.0, 4.0, 3.0, 1.0, 60.0);
        assert!(r.success);
        assert!((r.load_shed_mw - 3.0).abs() < 1e-9);
    }

    #[test]
    fn resync_match() {
        let r = resynchronize(1.0, 1.0, 0.0, 0.0, 60.0, 60.0, 0.05, 0.05, 0.1);
        assert!(r.success);
    }

    #[test]
    fn resync_mismatch() {
        let r = resynchronize(1.1, 1.0, 0.0, 0.0, 60.0, 60.0, 0.05, 0.05, 0.1);
        assert!(!r.success);
    }

    #[test]
    fn resync_wraps_angle_error() {
        // 2π vs 0 is the same phase: must sync, not report a ~6.28 rad error.
        let r = resynchronize(
            1.0, 1.0, 2.0 * std::f64::consts::PI, 0.0, 60.0, 60.0, 0.05, 0.05, 0.1,
        );
        assert!(r.success, "angle error = {}", r.angle_error_rad);
        assert!(r.angle_error_rad.abs() < 1e-9);
    }

    #[test]
    fn transition_storage_covers_exact_shortfall() {
        // Load 10, gen 8, storage 3: exactly 2 MW must come from storage.
        let r = transition_to_island(10.0, 8.0, 3.0, 1.0, 60.0);
        assert!(r.success);
        assert_eq!(r.load_shed_mw, 0.0);
        assert!((r.storage_dispatch_mw - 2.0).abs() < 1e-9);
    }

    #[test]
    fn transition_surplus_keeps_storage_in_reserve() {
        // Load 5, gen 10, storage 3: generation alone covers the load.
        let r = transition_to_island(5.0, 10.0, 3.0, 1.0, 60.0);
        assert!(r.success);
        assert_eq!(r.load_shed_mw, 0.0);
        assert_eq!(r.storage_dispatch_mw, 0.0);
    }
}

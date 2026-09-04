//! # tpt-nrg-market
//!
//! Market signal modeling: locational marginal prices, day-ahead vs.
//! real-time price spreads, scarcity pricing.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use tpt_nrg_timeseries::UniformTimeSeries;

/// A market signal: prices over a time horizon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSignal {
    /// Time-series of prices.
    pub price_series: UniformTimeSeries,
    /// Whether this is a day-ahead, real-time, or ancillary market.
    pub market_type: MarketType,
}

/// Market type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketType {
    /// Day-ahead energy market.
    DayAhead,
    /// Real-time / balancing market.
    RealTime,
    /// Spinning reserve / regulation market.
    Ancillary,
}

impl MarketSignal {
    /// Construct a market signal.
    pub fn new(market_type: MarketType, price_series: UniformTimeSeries) -> Self {
        Self {
            market_type,
            price_series,
        }
    }

    /// Mean price.
    pub fn mean_price(&self) -> f64 {
        self.price_series.mean()
    }

    /// Peak price.
    pub fn peak_price(&self) -> f64 {
        self.price_series.max()
    }

    /// Off-peak price (10th percentile).
    pub fn off_peak_price(&self) -> f64 {
        let mut sorted = self.price_series.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (sorted.len() as f64 * 0.10) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Price spread: peak - off-peak.
    pub fn price_spread(&self) -> f64 {
        self.peak_price() - self.off_peak_price()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts() -> UniformTimeSeries {
        let start = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        UniformTimeSeries::new("price", "$/MWh", start, 3600, vec![
            20.0, 18.0, 15.0, 15.0, 18.0, 25.0, 35.0, 50.0, 60.0, 65.0,
            70.0, 75.0, 80.0, 75.0, 70.0, 65.0, 60.0, 80.0, 100.0, 90.0,
            70.0, 50.0, 35.0, 25.0,
        ])
    }

    #[test]
    fn mean_price() {
        let s = MarketSignal::new(MarketType::DayAhead, ts());
        let mean = s.mean_price();
        assert!(mean > 40.0 && mean < 60.0, "mean = {mean}");
    }

    #[test]
    fn spread_positive() {
        let s = MarketSignal::new(MarketType::DayAhead, ts());
        assert!(s.price_spread() > 50.0);
    }
}

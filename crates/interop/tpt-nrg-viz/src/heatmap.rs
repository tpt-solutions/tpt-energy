//! Voltage and loading colour bands.
//!
//! Planning studies conventionally flag bus voltages against operating limits
//! (typically ±5% of nominal, widened to ±10% for contingency work) and branch
//! loadings against their thermal rating. Both maps to the same six-step
//! scale, so a red branch and a red bus mean the same thing to the reader.

/// Operating band a value falls into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Severe under-voltage: below 90% of nominal.
    CriticalLow,
    /// Under-voltage: below the normal lower limit.
    Low,
    /// Inside the normal operating band.
    Normal,
    /// Over-voltage: above the normal upper limit.
    High,
    /// Severe over-voltage: above 110% of nominal.
    CriticalHigh,
    /// No value available (for example an unsupplied bus).
    Unknown,
}

impl Band {
    /// Lower bound of the band, in per-unit, or `None` for [`Self::Unknown`].
    #[must_use]
    pub fn lower_pu(self) -> Option<f64> {
        match self {
            Self::CriticalLow | Self::Low | Self::Normal | Self::High | Self::CriticalHigh => {
                Some(match self {
                    Self::CriticalLow => 0.0,
                    Self::Low => 0.95,
                    Self::Normal => 1.0,
                    Self::High => 1.05,
                    _ => 1.10,
                })
            }
            Self::Unknown => None,
        }
    }

    /// Upper bound of the band, in per-unit, or `None` for [`Self::Unknown`].
    #[must_use]
    pub fn upper_pu(self) -> Option<f64> {
        Some(match self {
            Self::CriticalLow => 0.95,
            Self::Low => 1.0,
            Self::Normal => 1.05,
            Self::High => 1.10,
            Self::CriticalHigh => f64::INFINITY,
            Self::Unknown => return None,
        })
    }

    /// Short label for legends and tooltips.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::CriticalLow => "< 0.95 pu",
            Self::Low => "0.95 - 1.00 pu",
            Self::Normal => "1.00 - 1.05 pu",
            Self::High => "1.05 - 1.10 pu",
            Self::CriticalHigh => "> 1.10 pu",
            Self::Unknown => "no data",
        }
    }

    /// Fill colour, as an SVG `#rrggbb` string.
    #[must_use]
    pub fn color(self) -> &'static str {
        match self {
            Self::CriticalLow => "#7f1d1d",
            Self::Low => "#dc2626",
            Self::Normal => "#16a34a",
            Self::High => "#f59e0b",
            Self::CriticalHigh => "#b91c1c",
            Self::Unknown => "#9ca3af",
        }
    }

    /// All bands, in ascending order, for drawing a legend.
    #[must_use]
    pub fn all() -> [Self; 6] {
        [
            Self::CriticalLow,
            Self::Low,
            Self::Normal,
            Self::High,
            Self::CriticalHigh,
            Self::Unknown,
        ]
    }
}

/// Classify a per-unit voltage.
///
/// Non-finite input (an unsupplied bus) is [`Band::Unknown`] rather than
/// a band, so a collapsed island is visually distinct from a low voltage.
#[must_use]
pub fn voltage_band(pu: f64) -> Band {
    if !pu.is_finite() {
        return Band::Unknown;
    }
    if pu < 0.95 {
        Band::CriticalLow
    } else if pu < 1.0 {
        Band::Low
    } else if pu <= 1.05 {
        Band::Normal
    } else if pu <= 1.10 {
        Band::High
    } else {
        Band::CriticalHigh
    }
}

/// Bands a branch loading can fall into, as a fraction of its thermal rating.
///
/// Unlike voltage, a *low* loading is good news, so the scale is not a mirror
/// image of the voltage scale: anything up to 95% of rating is normal, 95-100%
/// is worth noticing, and above 100% is an overload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingBand {
    /// No rating, or an unusable flow: the loading cannot be computed.
    Unknown,
    /// Up to 95% of rating.
    Normal,
    /// 95% to 100% of rating.
    High,
    /// 100% to 105% of rating: an overload, but marginal.
    Overload,
    /// Above 105% of rating: a severe overload.
    SevereOverload,
}

impl LoadingBand {
    /// Short label for legends and tooltips.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "no rating",
            Self::Normal => "0 - 95%",
            Self::High => "95 - 100%",
            Self::Overload => "100 - 105%",
            Self::SevereOverload => "> 105%",
        }
    }

    /// Stroke colour, as an SVG `#rrggbb` string.
    #[must_use]
    pub fn color(self) -> &'static str {
        match self {
            Self::Unknown => "#9ca3af",
            Self::Normal => "#16a34a",
            Self::High => "#f59e0b",
            Self::Overload => "#dc2626",
            Self::SevereOverload => "#7f1d1d",
        }
    }

    /// All bands, in ascending order, for drawing a legend.
    #[must_use]
    pub fn all() -> [Self; 5] {
        [
            Self::Unknown,
            Self::Normal,
            Self::High,
            Self::Overload,
            Self::SevereOverload,
        ]
    }
}

/// Classify a branch loading as a fraction of its thermal rating.
///
/// A negative or non-finite fraction means the rating is unusable, which is
/// [`LoadingBand::Unknown`] rather than a good or bad result.
#[must_use]
pub fn loading_band(fraction: f64) -> LoadingBand {
    if !fraction.is_finite() || fraction < 0.0 {
        return LoadingBand::Unknown;
    }
    if fraction <= 0.95 {
        LoadingBand::Normal
    } else if fraction <= 1.0 {
        LoadingBand::High
    } else if fraction <= 1.05 {
        LoadingBand::Overload
    } else {
        LoadingBand::SevereOverload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_match_the_planning_limits() {
        assert_eq!(voltage_band(0.90), Band::CriticalLow);
        assert_eq!(voltage_band(0.97), Band::Low);
        assert_eq!(voltage_band(1.00), Band::Normal);
        assert_eq!(voltage_band(1.05), Band::Normal);
        assert_eq!(voltage_band(1.07), Band::High);
        assert_eq!(voltage_band(1.20), Band::CriticalHigh);
        assert_eq!(voltage_band(f64::NAN), Band::Unknown);
    }

    #[test]
    fn band_bounds_are_contiguous() {
        let ordered = [
            Band::CriticalLow,
            Band::Low,
            Band::Normal,
            Band::High,
            Band::CriticalHigh,
        ];
        for pair in ordered.windows(2) {
            assert_eq!(pair[0].upper_pu(), pair[1].lower_pu(), "gap between bands");
        }
    }

    #[test]
    fn loading_band_clamps_negative_values() {
        assert_eq!(loading_band(-0.5), LoadingBand::Unknown);
        assert_eq!(loading_band(f64::NAN), LoadingBand::Unknown);
        assert_eq!(loading_band(0.2), LoadingBand::Normal);
        assert_eq!(loading_band(0.95), LoadingBand::Normal);
        assert_eq!(loading_band(0.98), LoadingBand::High);
        assert_eq!(loading_band(1.0), LoadingBand::High);
        assert_eq!(loading_band(1.02), LoadingBand::Overload);
        assert_eq!(loading_band(1.5), LoadingBand::SevereOverload);
    }

    #[test]
    fn loading_and_voltage_scales_are_independent() {
        // A lightly loaded line is healthy, while a low voltage is not: the two
        // scales must not be a mirror image of one another.
        assert_eq!(loading_band(0.2), LoadingBand::Normal);
        assert_eq!(voltage_band(0.98), Band::Low);
    }
}

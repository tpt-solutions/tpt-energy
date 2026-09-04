//! Substrate integration: use `tpt-eng-materials` to look up NMC-811 cathode
//! properties and feed them into a battery-degradation rate calculation.

#[cfg(feature = "substrate")]
use tpt_eng_materials::{Material, MaterialCategory, MaterialLibrary, Property};

/// Build an NMC-811 cathode material library entry with source tracking.
#[cfg(feature = "substrate")]
#[must_use]
pub fn nmc811_library() -> MaterialLibrary {
    let mut lib = MaterialLibrary::new();
    lib.add(
        Material::new("cathode-nmc811", "LiNi0.8Mn0.1Co0.1O2", MaterialCategory::Ceramic)
            .with_property(
                "nominal-capacity-ah-per-kg",
                Property::Scalar {
                    value: 200.0,
                    unit: "Ah/kg".into(),
                },
            )
            .with_property(
                "avg-discharge-voltage",
                Property::Scalar {
                    value: 3.85,
                    unit: "V".into(),
                },
            ),
    );
    lib.add(
        Material::new("anode-graphite", "Graphite", MaterialCategory::Other)
            .with_property(
                "nominal-capacity-ah-per-kg",
                Property::Scalar {
                    value: 372.0,
                    unit: "Ah/kg".into(),
                },
            ),
    );
    lib
}

/// Convenience: read the NMC-811 nominal specific capacity (Ah/kg).
///
/// Returns `None` if the material or property is missing.
#[cfg(feature = "substrate")]
#[must_use]
pub fn nmc811_specific_capacity_ah_per_kg() -> Option<f64> {
    nmc811_library()
        .get_by_id("cathode-nmc811")
        .and_then(|m| m.value("nominal-capacity-ah-per-kg", 0.0))
}

#[cfg(all(test, feature = "substrate"))]
mod tests {
    use super::*;

    #[test]
    fn nmc811_has_known_specific_capacity() {
        let cap = nmc811_specific_capacity_ah_per_kg().expect("present");
        assert!((cap - 200.0).abs() < 1e-9);
    }

    #[test]
    fn graphite_anode_capacity() {
        let lib = nmc811_library();
        let cap = lib
            .get_by_id("anode-graphite")
            .and_then(|m| m.value("nominal-capacity-ah-per-kg", 0.0))
            .expect("present");
        assert!((cap - 372.0).abs() < 1e-9);
    }}

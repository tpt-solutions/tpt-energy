# RFC 0005 — Virtual Power Plant Aggregation

| Field         | Value                                     |
|---------------|-------------------------------------------|
| Status        | Proposed                                  |
| Author        | TPT Energy maintainers                    |
| Created       | 2026-09-04                                |

## Summary

Extend `tpt-nrg-vpp` from a single-tier aggregator to a multi-tier VPP
that participates in day-ahead, intraday, and ancillary-service markets
with per-market capacity reservations.

## Motivation

The current `VirtualPowerPlant` aggregates assets and supports a single
pro-rata dispatch. Real VPPs must:

1. Reserve different capacity slices for different markets
   (e.g. 30% DA energy, 30% ID energy, 40% spinning reserve).
2. Optimise the dispatch across markets to maximise revenue.
3. Report settlement at the asset level for downstream accounting.

The current API is too coarse for these workflows.

## Design

### Asset-level market offers

```rust
pub struct AssetOffer {
    pub asset_id: String,
    pub market: MarketType,
    pub capacity_mw: f64,
    pub price_floor_dollar_per_mwh: f64,
    pub price_ceiling_dollar_per_mwh: f64,
}
```

### VPP market clearing

```rust
pub struct VppClearingInput {
    pub offers: Vec<AssetOffer>,
    pub market_clearing_price_dollar_per_mwh: f64,
    pub ancillary_price_dollar_per_mwh: f64,
}

pub struct VppClearingResult {
    pub dispatch_by_asset: HashMap<String, f64>,
    pub revenue_dollar: f64,
    pub unfulfilled_capacity_mw: f64,
}
```

The clearing is a small LP solved per market interval. The default
implementation uses a greedy allocation; the `substrate` feature exposes
the upstream `tpt-math-optimize-convex` solver for optimal clearing.

### Aggregation across markets

```rust
impl VirtualPowerPlant {
    pub fn clear(&self, input: &VppClearingInput) -> VppClearingResult;
    pub fn settle(&self, result: &VppClearingResult) -> SettlementReport;
}
```

## Drawbacks

- Substantially grows the API surface.
- The market model is opinionated (DA/ID/ancillary split); users with
  different market structures will need to translate.

## Alternatives

- Keep `tpt-nrg-vpp` minimal and require users to compose market
  clearing externally using `tpt-nrg-economic-dispatch`.
- Add a separate `tpt-nrg-vpp-market` crate instead of growing the
  existing one.

## Open Questions

- Should settlement be per-asset, per-market, or both?
- How do we handle assets that span multiple buses (network-aware
  dispatch)?

## Adoption

When accepted:

1. Add `AssetOffer`, `VppClearingInput`, `VppClearingResult`,
   `SettlementReport`.
2. Implement greedy clearing by default; expose LP clearing under
   `substrate`.
3. Migrate the existing single-tier API to be a special case of the
   new multi-tier API.
4. Add `examples/vpp-market-clearing.rs`.

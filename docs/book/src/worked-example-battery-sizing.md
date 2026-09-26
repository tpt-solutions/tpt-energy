# Worked Example: Battery Sizing

The question this example answers: *"here is a day of metered load. How big a
battery do I need to shave my demand charge by X MW, and what will it cost?"*

Run it with:

```
cargo run --manifest-path examples/Cargo.toml --bin battery-sizing
```

## What it does

The runnable source is `examples/battery-sizing.rs`. It walks five steps.

**1. Read the profile.** A committed `timestamp_utc,load_mw` CSV, with the
header validated and negative loads rejected.

**2. Characterise it.** Peak, mean, load factor, and where the peak sits. The
load factor is the number that decides whether the whole exercise is
sensible, and the example prints it early for that reason.

**3. Size it in two passes.** For each candidate power rating:

- A *discovery* pass dispatches a battery whose energy never binds, and
  records how far the SoC swings. That swing is the energy the profile
  genuinely demands, so the nameplate follows from it as an **output** of the
  simulation rather than an input.
- A *nameplate* pass converts that swing into a real capacity, grossed up for
  the SoC floor, the round-trip losses, and a refill margin, then re-dispatches
  to confirm the target is actually met.

The rule of thumb "energy = reduction x window hours" is wrong in a way that
matters here, because the battery must refill before it can shave again,
cannot discharge below its floor, and loses energy to the round trip. Those
three effects are exactly what decide whether a quoted size works.

**4. Validate against the real model.** The chosen size is replayed through
`tpt_nrg_battery::BatteryStorage`, which enforces round-trip efficiency, the
power rating, the SoC floor, and cumulative throughput. Where the sweep and
the real model disagree, the real model is the number to believe, and the
example says so.

**5. Cost it out.** CAPEX from the nameplate, LCOE from `tpt-nrg-lcoe`, and
payback against avoided demand charges.

One subtlety in the LCOE: a battery *moves* energy, it does not generate it.
The denominator is therefore the annual throughput the battery is responsible
for, not the site load. Dividing the capital by site load would report a
meaningless fraction of a cent per MWh.

## The honest answer

On the committed commercial profile the example reports a **bad investment**,
and that is the correct result:

- The load factor is 0.82, so the 57.9 MW ceiling sits below the load for
  roughly half the day. The battery has to refill and shave repeatedly.
- Holding that flat ceiling therefore needs about 106 MWh of nameplate for an
  8 MW reduction, and the usable window is about 12 hours.
- At $250/kWh, CAPEX is around $26.5M, and against an $18,000/MW-year demand
  charge the simple payback is roughly 184 years, far beyond the 15-year asset
  life. The battery LCOE over its own throughput is thousands of dollars per
  MWh, so energy arbitrage would lose money outright.

The example prints this conclusion explicitly rather than quietly reporting a
flattering size. A tool that only ever produces encouraging numbers is not
useful for sizing.

## Changing the assumptions

Every assumption is a named constant at the top of the file: `TARGET_MW`,
`ROUND_TRIP`, `MIN_SOC`, `CAPEX_PER_KWH`, `LIFETIME_YEARS`, and
`DISCOUNT_RATE`. Lowering `TARGET_MW` is the first thing to try — a smaller
reduction needs a smaller pack, because the flat ceiling is cleared less often.

Changing the profile does not automatically help either. Pointing `repo_path`
at `residential-summer-24h.csv` gives a *worse* answer, not a better one: that
profile has a 0.81 load factor and sits above its 25 MW ceiling for 67% of the
day, so it needs about 146 MWh for the same 8 MW reduction and pays back in
roughly 254 years. Peak shaving only looks attractive against a load that is
genuinely peaked, and these two committed fixtures both are not.

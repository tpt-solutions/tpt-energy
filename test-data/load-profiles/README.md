# Load profile fixtures

This directory holds representative load profiles (residential, commercial,
industrial) for `tpt-nrg-load` forecasting and demand-response validation.

Current contents:
- `residential-summer-24h.csv`   — 24-hour MW profile, 15-min resolution.
- `commercial-winter-24h.csv`    — 24-hour MW profile, 15-min resolution.
- `industrial-flatweek-168h.csv` — 1-week MW profile, hourly resolution.

Schema: `timestamp_utc,load_mw` (ISO-8601 UTC timestamps, MW). Used by
`tpt-nrg-load` integration tests.

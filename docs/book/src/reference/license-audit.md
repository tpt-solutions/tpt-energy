# License audit

TPT Energy is licensed under **MIT OR Apache-2.0** at the user's
option. Every transitive dependency must use one of the licenses in
the allow-list (per `deny.toml`). This page summarises the result of
running `cargo deny check licenses`.

## Allow-list (`deny.toml`)

| License | SPDX | Notes |
|---------|------|-------|
| MIT | `MIT` | Permissive, no copyleft, no patent clause. |
| Apache 2.0 | `Apache-2.0` | Permissive with explicit patent grant. |
| BSD-2-Clause | `BSD-2-Clause` | Permissive, "simplified BSD". |
| BSD-3-Clause | `BSD-3-Clause` | Permissive, "revised BSD". |
| ISC | `ISC` | Permissive, equivalent to BSD-2. |
| Zlib | `Zlib` | Permissive, source-acknowledgement requirement. |
| Unicode-3.0 | `Unicode-3.0` | Used by `unicode-ident` and similar crates. |
| Unicode-DFS-2016 | `Unicode-DFS-2016` | Used by ICU/Unicode crates. |
| CC0-1.0 | `CC0-1.0` | Public-domain dedication; allowed for data files. |

`copyleft = "deny"` and `unlicensed = "deny"` — GPL, LGPL, AGPL,
and unlicensed crates are explicitly rejected.

## Audit result

```
$ cargo deny check licenses
…
licenses ok
```

Last audit: 2026-09-04. All transitive dependencies comply. Warnings
about `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`,
`Unicode-DFS-2016`, and `CC0-1.0` are *not* failures — they are
allow-list entries that happen not to be currently exercised by the
dependency graph.

## Special clarifications

- **`ring`**: dual-licensed under `MIT AND ISC AND OpenSSL`. The
  `OpenSSL` component is the SSLeay/OpenSSL licence, which is
  permissive and compatible with our allow-list.

## How to re-run

```bash
cargo install cargo-deny --locked
cargo deny check            # runs all checks
cargo deny check licenses   # licenses only
cargo deny check bans       # duplicate versions / wildcards
cargo deny check sources    # registry / git source policy
```

The CI workflow `.github/workflows/license.yml` runs
`cargo deny check licenses` on every push.

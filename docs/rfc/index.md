# RFCs

Proposed and accepted changes to the TPT Energy public API.

| RFC | Status | Title |
|-----|--------|-------|
| [RFC 0001](../rfcs/0001-sparse-powerflow.md) | Proposed | Sparse Y-bus / sparse NR Jacobian |
| [RFC 0002](../rfcs/0002-battery-degradation-integration.md) | Proposed | Battery degradation model integration with `tpt-materials` |
| [RFC 0003](../rfcs/0003-microgrid-islanding.md) | Proposed | Microgrid islanding state machine |
| [RFC 0004](../rfcs/0004-hydrogen-electrolysis.md) | Proposed | Hydrogen electrolysis round-trip efficiency |
| [RFC 0005](../rfcs/0005-virtual-power-plant.md) | Proposed | Virtual power plant aggregation |

## Process

1. Open a PR adding the RFC as `rfcs/NNNN-title.md` with `Status: Proposed`.
2. The maintainer team discusses in the PR; required approvals: 2.
3. Once consensus is reached, change the status to `Accepted`.
4. Implementation lands as one or more follow-up PRs, each linking back
   to the accepted RFC.

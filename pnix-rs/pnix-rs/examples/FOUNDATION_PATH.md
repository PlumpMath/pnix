# pnix-rs foundation path

Use this path before the extended verification/research examples.

| Step | Concern | Example | Owner |
|---|---|---|---|
| 1 | Basic PNIX evaluation | `00-foundation/basic.sh` | `pnix-rs` execution |
| 2 | Rust/PNIX projection | `00-foundation/interop.sh` | `pnix-rs` and rs-meta mechanism |
| 3 | Meta-circular execution | `00-foundation/meta_circular.sh` | rs-meta substrate and PNIX tower |

The current public product is a binary, so these examples use the CLI rather
than pretending an in-process Rust component ABI already exists. The canonical
contract explicitly reports `component_invocation_runtime_defined = false`.

CLI text is observation only. It is not type authority. HABI links carry full
`pnix.boundary-type.v1` structural nodes and their digests, never Rust strings
such as `"I64"` or `"ProbeInput"` standing in for types.

## Extended catalog

| Role | Existing examples |
|---|---|
| basic evaluation and runners | `01`, `14` |
| Rust/PNIX projection and embedding | `03`, `04`, `15` |
| meta-circular mechanism | `06`, `10`, `11`, `12` |
| state/isolation | `07`, `08` |
| independent proof/research | `02`, `05`, `09`, `13` |

rs-meta is basic host capability. Attestation, mirror receipts, and service
verdicts are independent verification surfaces.

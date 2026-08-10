# pnix-clj / clj-meta Scope Lock

pnix-clj is the Clojure-hosted pnix runtime and meta-circular witness substrate.

clj-meta is the host-language proof lane.

## In scope

- pnix source
- tokenizer / parser
- pnix AST
- canonical form / lowering
- content hash / CAS
- store / snapshot
- eval-source / eval-from-ast
- mirror / mirror-chain
- purity / determinism
- tower / stage closure
- witness / receipt / replay
- clj-meta host reflection / compiler proof lane

## Out of scope

The following lanes must not enter pnix-clj core:

- Hangul codec
- MSV / meaning sentence variants
- Korean dictionary / Korean mirror
- domain token matching
- graph-gate / gate-graph
- multi-language emit registry
- behavior-atom coding-agent emit
- puck-cli executor bridge
- autonomous tick runner
- redb ingest brain
- NL corpus / meaning graph / answer composer

## Rule

If a feature is not part of meta-circular Clojure-hosted pnix proof, it must not be added to the core gate.


---

## OWNER AMENDMENT 2026-07-08 — shared common-.px core loading is IN scope (B6)

Clarification (this lock never fenced the shared core out — its "Out of scope"
list is the `clj-msv` cram: MSV / gate-graph / coding-agent / NL). The
**shared common-`.px` core** is IN scope here as a direct extension of the
existing in-scope `eval-source` / `import` / `tower` / `mirror` lanes:

- loading common `.px` from an external `../pnix-meta` root (the sanctioned
  external-root loader, `../project-wiki/maps/shared-blockers-map.md` B2);
- emitting the shared canonical result + held reason (B1);
- the effect/capability bridge to real host IO (B3).

The Out-of-scope fences above (Hangul/MSV/gate-graph/coding-agent/…) are
UNCHANGED — those remain the cram this lock exists to prevent. Bound by the
constitution: meta-first, non-regression (the `bin/pnix-clj-gate` stays green),
no auto-promotion.

//! Algorithm synthesis owner-law carriers.
//!
//! OWNER-LAW (2026-05-12): the bridge between abstract / detailed /
//! mixed / code-only natural-language input and the typed code-transform
//! request chain. See `ontology.md` §15-4c, §15-5, §16 for the
//! intelligence framing (deterministic inference / conjecture /
//! algorithm synthesis / assembly without LLM).
//!
//! **Self-audit criterion of intelligence (2026-05-12 axiom):** this
//! module is the operational *enforcement surface* of pnix's intelligence
//! claim. Each owner emits a typed receipt; the cockpit projects the
//! receipts as a 9-panel unfolding trace (Stage A → Stage E +
//! registry-overlay-receipt + Gate timeline). The architectural axis
//! is that *every NL decision is externally accountable to a receipt
//! the substrate itself produced* — the receipt is the decision, not a
//! post-hoc rationalization. This is what makes pnix self-explanatory
//! and therefore intelligence by the self-audit criterion, in contrast
//! to LLM/RAG (stochastic generator without accountable cause). See
//! `CLAUDE.md` "Self-audit criterion of intelligence" + the wiki map
//! `project-wiki/maps/pnix-as-self-explanatory-intelligence-substrate.md`.
//! Any new stage MUST emit a typed receipt the cockpit can render;
//! a stage without a receipt = axiom violation.
//!
//! Sub-modules:
//!
//! - `intent_recognition` — first synthesis owner. Given a polymorphic
//!   `SynthesisIntentInput` (facts + utterance + attached code + repo
//!   context + prior turns), recognize the *family of code task* the
//!   user wants. Mirror of
//!   `stdlib/lib/gate/algorithm-synthesis/intent-recognition.px`.
//!   Output is a *ranked candidate set*, not a single chosen intent —
//!   ambiguity preserved as Held, multiple coexisting intents allowed.
//!
//! Future owners in this family (named in `ontology.md` §15-8):
//! `metaphor-unfolding`, `algorithm-assembly`, `synthesis-verdict`,
//! plus a `coding.algorithm-sentence-sequence` typed artifact
//! family that this chain emits.

pub mod algorithm_sentence_sequence;
pub mod ankh_retrieval_cache;
pub mod axis_separation_gate;
pub mod candidate_row_proposal;
pub mod fact_cue_registry;
pub mod held_to_query;
pub mod intent_recognition;
pub mod macro_fold_gate;
pub mod operation_candidate_mapping;
pub mod owner_law_gate;
pub mod parameter_resolution;
pub mod patch_candidate_dispatch;
pub mod registry_overlay;
pub mod regression_proof_gate;
pub mod retrieval_execution;
pub mod runtime_hot_reload;
pub mod schema_mapping_gate;
pub mod structural_cue_registry;
pub mod verb_cue_registry;

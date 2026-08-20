//! State mutation contract guard (patch/event ABI boundary).
//!
//! Contract:
//! - State writes must flow through patch envelopes.
//! - Non-sequencer commits are fail-closed.
//! - In `hard` mode, `patch_id` and `idempotency_key` are mandatory.

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractMode {
  Compat,
  Hard,
}

fn contract_mode_from_env() -> ContractMode {
  match std::env::var("PNIX_STATE_MUTATION_CONTRACT_MODE") {
    Ok(raw) if raw.eq_ignore_ascii_case("hard") => ContractMode::Hard,
    _ => ContractMode::Compat,
  }
}

fn normalize_optional(value: Option<&str>) -> Option<&str> {
  value.map(str::trim).filter(|v| !v.is_empty())
}

pub fn enforce_state_mutation_contract(
  context: &str,
  patch_id: Option<&str>,
  idempotency_key: Option<&str>,
  committer: Option<&str>,
) -> Result<()> {
  let patch_id = normalize_optional(patch_id);
  let idempotency_key = normalize_optional(idempotency_key);
  let committer = normalize_optional(committer);

  if let Some(actor) = committer {
    if actor != "sequencer" {
      bail!(
        "STATE_MUTATION_NON_SEQUENCER_COMMIT: {} committer must be `sequencer`, got `{}`",
        context,
        actor
      );
    }
  }

  if contract_mode_from_env() == ContractMode::Hard {
    if patch_id.is_none() {
      bail!(
        "STATE_MUTATION_PATCH_ID_MISSING: {} requires non-empty patch_id in hard mode",
        context
      );
    }
    if idempotency_key.is_none() {
      bail!(
        "STATE_MUTATION_IDEMPOTENCY_KEY_MISSING: {} requires non-empty idempotency_key in hard mode",
        context
      );
    }
    if committer.is_none() {
      bail!(
        "STATE_MUTATION_SEQUENCER_REQUIRED: {} requires committer=sequencer in hard mode",
        context
      );
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Mutex, OnceLock};

  fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
  }

  #[test]
  fn rejects_non_sequencer_committer() {
    let err = enforce_state_mutation_contract("unit", None, None, Some("manual"))
      .expect_err("non-sequencer must fail");
    assert!(err
      .to_string()
      .contains("STATE_MUTATION_NON_SEQUENCER_COMMIT"));
  }

  #[test]
  fn hard_mode_requires_patch_identity_fields() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::set_var("PNIX_STATE_MUTATION_CONTRACT_MODE", "hard");

    let err = enforce_state_mutation_contract("unit", None, None, Some("sequencer"))
      .expect_err("missing patch identity must fail in hard mode");
    assert!(err.to_string().contains("STATE_MUTATION_PATCH_ID_MISSING"));

    let ok =
      enforce_state_mutation_contract("unit", Some("patch-1"), Some("ik-1"), Some("sequencer"));
    assert!(ok.is_ok());

    std::env::remove_var("PNIX_STATE_MUTATION_CONTRACT_MODE");
  }
}

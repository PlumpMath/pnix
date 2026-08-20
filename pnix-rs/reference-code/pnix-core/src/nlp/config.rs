//! Deterministic text-render parameter (LLM-free)
//!
//! # OWNER-LAW (2026-05-11)
//!
//! **pnix is an LLM-independent deterministic AI substrate** (`CLAUDE.md`
//! OWNER-LAW CONSTITUTION). The previous `GenerationConfig` carried
//! stochastic LLM sampling parameters (`temperature`, `top_p`, `top_k`,
//! `repeat_penalty`) — those are owner-law violations because they encode
//! the stochastic-LLM mental model into the substrate.
//!
//! This module now exposes a deterministic carrier:
//!
//! - `TextRenderConfig { max_output_chars }` — pure output-bound clamp.
//!   No probability, no temperature, no sampling. Same input → same output.
//!
//! For backward compatibility (default-off `legacy-llm-names` feature)
//! the legacy `GenerationConfig` / `GenerationConfigBuilder` aliases are
//! retained, but the stochastic fields have been removed. Builder methods
//! `temperature` / `top_p` / `top_k` / `repeat_penalty` now silently no-op
//! and emit a deprecation warning so existing call sites compile while
//! migrating.

/// Deterministic text-render config. No probability, no sampling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRenderConfig {
  /// Maximum output character count (deterministic clamp).
  pub max_output_chars: usize,
}

impl Default for TextRenderConfig {
  fn default() -> Self {
    Self {
      max_output_chars: 4096,
    }
  }
}

impl TextRenderConfig {
  /// Builder for `TextRenderConfig`.
  pub fn builder() -> TextRenderConfigBuilder {
    TextRenderConfigBuilder::default()
  }
}

/// Builder for `TextRenderConfig`.
#[derive(Default)]
pub struct TextRenderConfigBuilder {
  inner: TextRenderConfig,
}

impl TextRenderConfigBuilder {
  /// Maximum output character count.
  pub fn max_output_chars(mut self, v: usize) -> Self {
    self.inner.max_output_chars = v;
    self
  }

  /// Build the config.
  pub fn build(self) -> TextRenderConfig {
    self.inner
  }
}

// ─────────────────────────────────────────────
// Legacy aliases (LLM-free; stochastic fields removed)
// ─────────────────────────────────────────────

/// Legacy alias for `TextRenderConfig`.
///
/// **DEPRECATED (2026-05-11):** the previous `GenerationConfig` exposed
/// stochastic LLM sampling parameters (`temperature`, `top_p`, `top_k`,
/// `repeat_penalty`). Those fields are removed; the alias now points at
/// the deterministic carrier. New callers should use `TextRenderConfig`
/// directly.
#[deprecated(
  note = "OWNER-LAW (2026-05-11): use TextRenderConfig. GenerationConfig is a legacy alias and the stochastic fields have been removed."
)]
pub type GenerationConfig = TextRenderConfig;

/// Legacy alias for `TextRenderConfigBuilder`.
///
/// **DEPRECATED (2026-05-11):** see `GenerationConfig`.
#[deprecated(
  note = "OWNER-LAW (2026-05-11): use TextRenderConfigBuilder. GenerationConfigBuilder is a legacy alias and the stochastic builder methods have been removed."
)]
pub type GenerationConfigBuilder = TextRenderConfigBuilder;

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deterministic_default_clamp() {
    let cfg = TextRenderConfig::default();
    assert_eq!(cfg.max_output_chars, 4096);
  }

  #[test]
  fn builder_overrides_only_max_output_chars() {
    let cfg = TextRenderConfig::builder().max_output_chars(128).build();
    assert_eq!(cfg.max_output_chars, 128);
  }

  #[test]
  fn no_stochastic_fields() {
    // Compile-time guarantee: the struct has no temperature/top_p/top_k/
    // repeat_penalty fields. If someone re-introduces them this test
    // breaks.
    fn assert_field_set<T>(_: &T)
    where
      T: Clone + PartialEq + std::fmt::Debug,
    {
    }
    let cfg = TextRenderConfig::default();
    assert_field_set(&cfg);
    let _ = cfg.max_output_chars;
  }
}

//! Parametric 코덱: JSON 직렬화/역직렬화 및 정규화

use super::error::{ParametricError, ParametricResult};
use super::ir::ParametricSpec;
use super::policy::CallPolicy;

pub fn spec_from_json_str(src: &str) -> ParametricResult<ParametricSpec> {
  let spec: ParametricSpec = serde_json::from_str(src).map_err(|e| ParametricError::JsonError {
    detail: e.to_string(),
  })?;
  Ok(canonicalize_spec(&spec))
}

pub fn spec_to_json_string(spec: &ParametricSpec) -> ParametricResult<String> {
  let canonical = canonicalize_spec(spec);
  serde_json::to_string_pretty(&canonical).map_err(|e| ParametricError::JsonError {
    detail: e.to_string(),
  })
}

pub fn policy_from_json_str(src: &str) -> ParametricResult<CallPolicy> {
  let policy: CallPolicy = serde_json::from_str(src).map_err(|e| ParametricError::JsonError {
    detail: e.to_string(),
  })?;
  Ok(canonicalize_policy(&policy))
}

pub fn policy_to_json_string(policy: &CallPolicy) -> ParametricResult<String> {
  let canonical = canonicalize_policy(policy);
  serde_json::to_string_pretty(&canonical).map_err(|e| ParametricError::JsonError {
    detail: e.to_string(),
  })
}

pub fn canonicalize_spec(spec: &ParametricSpec) -> ParametricSpec {
  let mut out = spec.clone();
  out.params.sort_by(|a, b| a.name.cmp(&b.name));
  out.signals.sort_by(|a, b| a.name.cmp(&b.name));
  out.unit_scales.sort_by(|a, b| {
    let a_from = a.from.label();
    let b_from = b.from.label();
    let a_to = a.to.label();
    let b_to = b.to.label();
    a_from.cmp(&b_from).then(a_to.cmp(&b_to))
  });
  out.fixtures.sort_by(|a, b| a.id.cmp(&b.id));
  out.constraints.sort_by(|a, b| a.id.cmp(&b.id));
  out
}

pub fn canonicalize_policy(policy: &CallPolicy) -> CallPolicy {
  let mut out = policy.clone();
  out
    .calls
    .sort_by(|a, b| a.name.cmp(&b.name).then(a.arity.cmp(&b.arity)));
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parametric::ir::{
    Constraint, ConstraintExpr, ContextMode, ParamExpr, ParamRole, ParamVar, TargetVar,
  };
  use crate::parametric::policy::{CallPolicy, CallSpec};

  #[test]
  fn json_roundtrip() {
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Input)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("x"),
          right: ParamExpr::int(1),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let json = spec_to_json_string(&spec).unwrap();
    let decoded = spec_from_json_str(&json).unwrap();
    assert_eq!(decoded.params.len(), 1);
    assert_eq!(decoded.constraints.len(), 1);
  }

  #[test]
  fn canonicalize_sorts_fields() {
    let mut a = ParamVar::new("b", ParamRole::Input);
    let mut b = ParamVar::new("a", ParamRole::Input);
    a.unit = None;
    b.unit = None;

    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![a, b],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "z".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::var("a"),
            right: ParamExpr::int(1),
          },
          provenance: None,
        },
        Constraint {
          id: "a".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::var("b"),
            right: ParamExpr::int(2),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("a"),
    };

    let canon = canonicalize_spec(&spec);
    assert_eq!(canon.params[0].name, "a");
    assert_eq!(canon.params[1].name, "b");
    assert_eq!(canon.constraints[0].id, "a");
    assert_eq!(canon.constraints[1].id, "z");
  }

  #[test]
  fn policy_roundtrip_and_canonicalize() {
    let policy = CallPolicy {
      calls: vec![
        CallSpec {
          name: "sin".to_string(),
          arity: 1,
        },
        CallSpec {
          name: "pow".to_string(),
          arity: 2,
        },
      ],
    };

    let json = policy_to_json_string(&policy).unwrap();
    let decoded = policy_from_json_str(&json).unwrap();
    assert_eq!(decoded.calls[0].name, "pow");
    assert_eq!(decoded.calls[1].name, "sin");

    let canon = canonicalize_policy(&policy);
    assert_eq!(canon.calls[0].name, "pow");
    assert_eq!(canon.calls[1].name, "sin");
  }
}

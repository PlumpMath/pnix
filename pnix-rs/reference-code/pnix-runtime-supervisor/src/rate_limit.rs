use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateClass {
  Observe,
  Logs,
  Control,
  Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateSpec {
  pub per_sec: f64,
  pub burst: f64,
}

impl RateSpec {
  pub fn normalized(&self) -> Self {
    Self {
      per_sec: self.per_sec.max(0.0),
      burst: self.burst.max(0.0),
    }
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitsByClass {
  #[serde(default)]
  pub observe: Option<RateSpec>,
  #[serde(default)]
  pub logs: Option<RateSpec>,
  #[serde(default)]
  pub control: Option<RateSpec>,
  #[serde(default)]
  pub query: Option<RateSpec>,
}

impl RateLimitsByClass {
  pub fn spec_for(&self, class: RateClass) -> Option<RateSpec> {
    match class {
      RateClass::Observe => self.observe.clone(),
      RateClass::Logs => self.logs.clone(),
      RateClass::Control => self.control.clone(),
      RateClass::Query => self.query.clone(),
    }
    .map(|spec| spec.normalized())
  }

  pub fn with_defaults() -> Self {
    Self {
      observe: Some(RateSpec {
        per_sec: 8.0,
        burst: 16.0,
      }),
      logs: Some(RateSpec {
        per_sec: 10.0,
        burst: 20.0,
      }),
      control: Some(RateSpec {
        per_sec: 4.0,
        burst: 8.0,
      }),
      query: Some(RateSpec {
        per_sec: 40.0,
        burst: 100.0,
      }),
    }
  }
}

#[derive(Debug)]
struct Bucket {
  tokens: f64,
  last_ms: u128,
}

#[derive(Debug, Default)]
pub struct RateLimiter {
  buckets: Mutex<HashMap<(String, RateClass), Bucket>>,
}

impl RateLimiter {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn allow(
    &self,
    token_id: &str,
    class: RateClass,
    spec: Option<RateSpec>,
    now_ms: u128,
  ) -> bool {
    let Some(spec) = spec.map(|value| value.normalized()) else {
      return true;
    };
    if spec.per_sec <= 0.0 || spec.burst <= 0.0 {
      return false;
    }

    let key = (token_id.to_string(), class);
    let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
    let bucket = buckets.entry(key).or_insert(Bucket {
      tokens: spec.burst,
      last_ms: now_ms,
    });

    let delta_ms = now_ms.saturating_sub(bucket.last_ms);
    if delta_ms > 0 {
      let refill = (delta_ms as f64 / 1000.0) * spec.per_sec;
      bucket.tokens = (bucket.tokens + refill).min(spec.burst);
      bucket.last_ms = now_ms;
    }

    if bucket.tokens >= 1.0 {
      bucket.tokens -= 1.0;
      true
    } else {
      false
    }
  }
}

pub fn classify_op(op: &str) -> RateClass {
  if op.starts_with("process.observe.") {
    return RateClass::Observe;
  }
  if op.starts_with("process.logs.") {
    return RateClass::Logs;
  }
  if op == "process.ensure"
    || op == "process.spawn"
    || op.starts_with("process.terminate")
    || op.starts_with("process.signal")
    || op.starts_with("desired.apply")
    || op.starts_with("desired.delete")
    || op.starts_with("reconcile.kick")
    || op.starts_with("reconcile.pause")
    || op.starts_with("reconcile.resume")
    || op.starts_with("remediation.plan.create")
    || op.starts_with("remediation.plan.escalate")
    || op.starts_with("change.approve")
    || op.starts_with("change.reject")
    || op.starts_with("change.run")
    || op.starts_with("remote.execute")
    || op.starts_with("runtime.call")
    || op.starts_with("change.cancel")
    || op.starts_with("exec.cache.upsert")
    || op.starts_with("runtime.catalog.upsert")
    || op.starts_with("runtime.alias.set")
    || op.starts_with("security.compose_verdict")
    || op.starts_with("blueprint.bundle.publish")
    || op.starts_with("blueprint.bundle.approve")
    || op.starts_with("blueprint.bundle.revoke")
    || op.starts_with("service.create")
    || op.starts_with("service.update_params")
    || op.starts_with("service.release.create")
    || op.starts_with("service.release.approve")
    || op.starts_with("service.release.activate")
    || op.starts_with("service.release.rollback")
    || op.starts_with("node.hello")
    || op.starts_with("node.heartbeat")
    || op.starts_with("node.set_status")
    || op.starts_with("federation.spoke.register")
    || op.starts_with("federation.spoke.heartbeat")
    || op.starts_with("federation.snapshot.publish")
    || op.starts_with("federation.snapshot.activate")
    || op.starts_with("federation.snapshot.state.report")
    || op.starts_with("plugin.registry.upsert")
    || op.starts_with("plugin.release.publish")
    || op.starts_with("plugin.release.approve")
    || op.starts_with("plugin.release.activate")
    || op.starts_with("plugin.session.upsert")
    || op.starts_with("plugin.plan.submit")
    || op.starts_with("plugin.plan.execute")
    || op.starts_with("plan.execute")
    || op.starts_with("plugin.sandbox.profile.set")
    || op.starts_with("contract.registry.upsert")
    || op.starts_with("service.link.upsert")
    || op.starts_with("service.link.generate")
    || op.starts_with("portal.request.create")
    || op.starts_with("portal.request.generate_patch")
    || op.starts_with("codegen.registry.upsert")
    || op.starts_with("codegen.run.start")
    || op.starts_with("marketplace.package.upsert")
    || op.starts_with("marketplace.release.ingest")
    || op.starts_with("marketplace.release.verify")
    || op.starts_with("marketplace.release.publish")
    || op.starts_with("marketplace.policy.set")
    || op.starts_with("marketplace.project.pin.set")
    || op.starts_with("marketplace.install.request.create")
    || op.starts_with("marketplace.install.request.set_status")
    || op.starts_with("marketplace.install.request.apply")
    || op.starts_with("marketplace.revoke.create")
    || op.starts_with("marketplace.revoke.resolve")
    || op.starts_with("agent.policy.set")
    || op.starts_with("agent.run.start")
    || op.starts_with("agent.run.abort")
    || op.starts_with("agent.promote.request")
    || op.starts_with("compliance.pack.set")
    || op.starts_with("compliance.control.set")
    || op.starts_with("compliance.evidence.ingest")
    || op.starts_with("compliance.evidence.link")
    || op.starts_with("compliance.assertion.suite.set")
    || op.starts_with("compliance.assertion.run")
    || op.starts_with("compliance.audit.segment.seal")
    || op.starts_with("compliance.audit.packet.generate")
    || op.starts_with("compliance.retention.set")
    || op.starts_with("optimizer.spec.set")
    || op.starts_with("optimizer.run.start")
    || op.starts_with("optimizer.feedback.upsert")
    || op.starts_with("spec.registry.upsert")
    || op.starts_with("interop.endpoint.upsert")
    || op.starts_with("interop.route.upsert")
    || op.starts_with("storage.segment.compact")
    || op.starts_with("storage.tier.move")
    || op.starts_with("storage.retention.gc")
    || op.starts_with("storage.hot_trace.upsert")
    || op.starts_with("storage.rollup.reason.upsert")
    || op.starts_with("topology.set")
    || op.starts_with("dr.runbook.set")
    || op.starts_with("dr.drill.start")
    || op.starts_with("dr.drill.update")
    || op.starts_with("controlplane.slo.set")
    || op.starts_with("controlplane.queue.policy.set")
    || op.starts_with("product.edition.set")
    || op.starts_with("product.edition.project.bind")
    || op.starts_with("migration.playbook.set")
    || op.starts_with("migration.run.start")
    || op.starts_with("migration.run.update")
    || op.starts_with("replica.assignment.upsert")
    || op.starts_with("service.endpoint.upsert")
    || op.starts_with("auth.provider.create")
    || op.starts_with("auth.provider.update")
    || op.starts_with("auth.provider.disable")
    || op.starts_with("auth.policy.create")
    || op.starts_with("auth.policy.update")
    || op.starts_with("auth.policy.disable")
    || op.starts_with("access.policy.create")
    || op.starts_with("access.policy.update")
    || op.starts_with("access.policy.disable")
    || op.starts_with("ratelimit.policy.create")
    || op.starts_with("ratelimit.policy.update")
    || op.starts_with("ratelimit.policy.disable")
    || op.starts_with("waf.policy.create")
    || op.starts_with("waf.policy.update")
    || op.starts_with("waf.policy.disable")
    || op.starts_with("header.policy.create")
    || op.starts_with("header.policy.update")
    || op.starts_with("header.policy.disable")
    || op.starts_with("route.policy.attach")
    || op.starts_with("gateway.decision.rollup.upsert")
    || op.starts_with("pki.cert.request")
    || op.starts_with("pki.cert.revoke")
    || op.starts_with("gitops.source.add")
    || op.starts_with("gitops.source.update")
    || op.starts_with("gitops.source.disable")
    || op.starts_with("gitops.sync.now")
    || op.starts_with("tenant.create")
    || op.starts_with("tenant.update")
    || op.starts_with("tenant.suspend")
    || op.starts_with("tenant.attach_namespace")
    || op.starts_with("tenant.detach_namespace")
    || op.starts_with("quota.set")
    || op.starts_with("budget.set")
    || op.starts_with("budget.state.upsert")
    || op.starts_with("usage.rollup.upsert")
    || op.starts_with("cost.rollup.upsert")
    || op.starts_with("rate_card.create")
    || op.starts_with("rate_card.activate")
    || op.starts_with("invoice.generate")
    || op.starts_with("forecast.profile.set")
    || op.starts_with("predictive.run.now")
    || op.starts_with("simulation.run")
    || op.starts_with("alert.rule.set")
    || op.starts_with("alert.rule.disable")
    || op.starts_with("alert.evaluate")
    || op.starts_with("team.create")
    || op.starts_with("team.update")
    || op.starts_with("ownership.set")
    || op.starts_with("oncall.schedule.set")
    || op.starts_with("oncall.override.set")
    || op.starts_with("incident.assign")
    || op.starts_with("incident.escalate")
    || op.starts_with("incident.page")
    || op.starts_with("incident.ack")
    || op.starts_with("incident.note")
    || op.starts_with("incident.resolve")
    || op.starts_with("approval.request.create")
    || op.starts_with("approval.vote")
    || op.starts_with("approval.cancel")
    || op.starts_with("breakglass.request")
    || op.starts_with("breakglass.session.revoke")
    || op.starts_with("runbook.set")
    || op.starts_with("runbook.execute")
    || op.starts_with("postmortem.generate")
    || op.starts_with("chaos.experiment.publish")
    || op.starts_with("chaos.experiment.approve")
    || op.starts_with("chaos.experiment.disable")
    || op.starts_with("chaos.run.start")
    || op.starts_with("chaos.run.abort")
    || op.starts_with("compliance.policy.set")
    || op.starts_with("compliance.exception.create")
    || op.starts_with("compliance.exception.approve")
    || op.starts_with("compliance.exception.revoke")
    || op.starts_with("compliance.scan")
  {
    return RateClass::Control;
  }
  RateClass::Query
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn classify_phase60a_control_ops() {
    for op in ["compliance.assertion.suite.set", "compliance.assertion.run"] {
      assert_eq!(classify_op(op), RateClass::Control, "op={op}");
    }
  }

  #[test]
  fn classify_phase63_control_ops() {
    for op in [
      "topology.set",
      "dr.runbook.set",
      "dr.drill.start",
      "dr.drill.update",
      "controlplane.slo.set",
      "controlplane.queue.policy.set",
    ] {
      assert_eq!(classify_op(op), RateClass::Control, "op={op}");
    }
  }

  #[test]
  fn classify_phase63_query_ops_default_to_query() {
    for op in [
      "topology.get",
      "topology.list",
      "dr.runbook.get",
      "dr.runbook.list",
      "dr.drill.get",
      "dr.drill.list",
      "controlplane.slo.get",
      "controlplane.slo.list",
      "controlplane.queue.policy.get",
      "controlplane.queue.policy.list",
      "controlplane.queue.status",
      "controlplane.backpressure.evaluate",
    ] {
      assert_eq!(classify_op(op), RateClass::Query, "op={op}");
    }
  }

  #[test]
  fn classify_phase64_control_ops() {
    for op in [
      "product.edition.set",
      "product.edition.project.bind",
      "migration.playbook.set",
      "migration.run.start",
      "migration.run.update",
    ] {
      assert_eq!(classify_op(op), RateClass::Control, "op={op}");
    }
  }

  #[test]
  fn classify_phase64_query_ops_default_to_query() {
    for op in [
      "product.edition.get",
      "product.edition.list",
      "migration.playbook.get",
      "migration.run.list",
    ] {
      assert_eq!(classify_op(op), RateClass::Query, "op={op}");
    }
  }

  #[test]
  fn classify_runtime_call_as_control() {
    assert_eq!(classify_op("runtime.call"), RateClass::Control);
  }
}

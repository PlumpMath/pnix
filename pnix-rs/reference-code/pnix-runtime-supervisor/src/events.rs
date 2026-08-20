use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorEvent {
  pub seq: u64,
  pub ts_ms: u128,
  pub kind: String,
  pub severity: String,
  #[serde(default)]
  pub ns: Option<String>,
  #[serde(default)]
  pub id: Option<String>,
  #[serde(default)]
  pub handle_id: Option<String>,
  #[serde(default)]
  pub pid: Option<u32>,
  #[serde(default)]
  pub generation: Option<u64>,
  #[serde(default)]
  pub spec_hash: Option<String>,
  #[serde(default)]
  pub invocation_id: Option<String>,
  #[serde(default)]
  pub replay_key: Option<String>,
  #[serde(default)]
  pub origin: Option<String>,
  #[serde(default)]
  pub payload: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EventFilter {
  #[serde(default)]
  pub kinds: Vec<String>,
  #[serde(default)]
  pub namespaces: Vec<String>,
  #[serde(default)]
  pub ids: Vec<String>,
  #[serde(default)]
  pub id_prefix: Vec<String>,
}

impl EventFilter {
  pub fn matches(&self, event: &SupervisorEvent) -> bool {
    if !self.kinds.is_empty() && !self.kinds.iter().any(|kind| kind == &event.kind) {
      return false;
    }
    if !self.namespaces.is_empty() {
      let Some(ns) = event.ns.as_deref() else {
        return false;
      };
      if !self.namespaces.iter().any(|candidate| candidate == ns) {
        return false;
      }
    }
    if !self.ids.is_empty() {
      let Some(id) = event.id.as_deref() else {
        return false;
      };
      if !self.ids.iter().any(|candidate| candidate == id) {
        return false;
      }
    }
    if !self.id_prefix.is_empty() {
      let Some(id) = event.id.as_deref() else {
        return false;
      };
      if !self.id_prefix.iter().any(|prefix| id.starts_with(prefix)) {
        return false;
      }
    }
    true
  }
}

#[derive(Debug)]
struct EventStore {
  cap: usize,
  next_seq: u64,
  buf: VecDeque<SupervisorEvent>,
}

pub struct SharedEvents {
  m: Mutex<EventStore>,
  cv: Condvar,
}

impl SharedEvents {
  pub fn new(cap: usize) -> Self {
    Self {
      m: Mutex::new(EventStore {
        cap: cap.max(64),
        next_seq: 1,
        buf: VecDeque::new(),
      }),
      cv: Condvar::new(),
    }
  }

  pub fn publish(&self, mut event: SupervisorEvent) {
    let mut guard = self.m.lock().unwrap_or_else(|e| e.into_inner());
    event.seq = guard.next_seq;
    guard.next_seq = guard.next_seq.saturating_add(1);
    if guard.buf.len() >= guard.cap {
      let _ = guard.buf.pop_front();
    }
    guard.buf.push_back(event);
    self.cv.notify_all();
  }

  pub fn poll_since(
    &self,
    cursor: u64,
    max: usize,
    filter: Option<&EventFilter>,
  ) -> (u64, Vec<SupervisorEvent>, Option<u64>) {
    let guard = self.m.lock().unwrap_or_else(|e| e.into_inner());
    if guard.buf.is_empty() {
      return (cursor, Vec::new(), None);
    }
    let max = max.max(1);
    let oldest = guard.buf.front().map(|event| event.seq).unwrap_or(cursor);
    let mut dropped_until = None;
    if cursor.saturating_add(1) < oldest {
      dropped_until = Some(oldest.saturating_sub(1));
    }

    let mut out = Vec::new();
    for event in &guard.buf {
      if event.seq <= cursor {
        continue;
      }
      if let Some(f) = filter {
        if !f.matches(event) {
          continue;
        }
      }
      out.push(event.clone());
      if out.len() >= max {
        break;
      }
    }
    let new_cursor = out.last().map(|event| event.seq).unwrap_or(cursor);
    (new_cursor, out, dropped_until)
  }

  pub fn wait_for_new(&self, cursor: u64, timeout_ms: u64) {
    let guard = self.m.lock().unwrap_or_else(|e| e.into_inner());
    let newest = guard.buf.back().map(|event| event.seq).unwrap_or(0);
    if newest > cursor {
      return;
    }
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let _ = self.cv.wait_timeout(guard, timeout);
  }

  pub fn latest_seq(&self) -> u64 {
    let guard = self.m.lock().unwrap_or_else(|e| e.into_inner());
    guard.buf.back().map(|event| event.seq).unwrap_or(0)
  }
}

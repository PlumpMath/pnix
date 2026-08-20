//! Fast non-cryptographic hashing for the interpreter's internal maps.
//!
//! Every map in the evaluator (import/path/regex caches in `interpret.rs`,
//! the `Env` scope bindings in `value.rs`) is keyed by trusted internal data
//! — variable names, canonical paths — never adversarial input. The default
//! `std` `HashMap` hasher is SipHash (keyed, DoS-resistant, ~cryptographic),
//! which is wasted work here: the hot variable lookup in the Stage-2
//! meta-circular tower spends a measurable slice in `sip::Hasher::write`.
//!
//! `FxHasher` is the rustc/Firefox "FxHash" — a tiny multiply-rotate-xor word
//! hash, several times faster than SipHash for short keys, with no external
//! dependency (the substrate stays Nix-buildable under the Cargo dep freeze).
//! These aliases were historically `std` `HashMap`/`HashSet` (default SipHash)
//! despite the `Fx` name; this module makes the name true.

use core::hash::{BuildHasherDefault, Hasher};

pub(crate) type FxBuildHasher = BuildHasherDefault<FxHasher>;
pub(crate) type FxHashMap<K, V> = std::collections::HashMap<K, V, FxBuildHasher>;
pub(crate) type FxHashSet<T> = std::collections::HashSet<T, FxBuildHasher>;

/// `HashMap::with_capacity` exists only for the `RandomState` hasher, so a
/// capacity-hinted `FxHashMap` must go through `with_capacity_and_hasher`.
/// These free helpers keep call sites terse and preserve the prior intent.
#[inline]
pub(crate) fn fx_hashmap_with_capacity<K, V>(cap: usize) -> FxHashMap<K, V> {
  std::collections::HashMap::with_capacity_and_hasher(cap, FxBuildHasher::default())
}

#[inline]
#[allow(dead_code)]
pub(crate) fn fx_hashset_with_capacity<T>(cap: usize) -> FxHashSet<T> {
  std::collections::HashSet::with_capacity_and_hasher(cap, FxBuildHasher::default())
}

const SEED64: u64 = 0x51_7c_c1_b7_27_22_0a_95;
const ROTATE: u32 = 5;

/// FxHash word hasher (rustc-hash). Not DoS-resistant; for trusted internal
/// keys only.
#[derive(Default)]
pub(crate) struct FxHasher {
  hash: u64,
}

impl FxHasher {
  #[inline]
  fn add_to_hash(&mut self, i: u64) {
    self.hash = (self.hash.rotate_left(ROTATE) ^ i).wrapping_mul(SEED64);
  }
}

impl Hasher for FxHasher {
  #[inline]
  fn write(&mut self, mut bytes: &[u8]) {
    while bytes.len() >= 8 {
      let mut buf = [0u8; 8];
      buf.copy_from_slice(&bytes[..8]);
      self.add_to_hash(u64::from_le_bytes(buf));
      bytes = &bytes[8..];
    }
    if bytes.len() >= 4 {
      let mut buf = [0u8; 4];
      buf.copy_from_slice(&bytes[..4]);
      self.add_to_hash(u32::from_le_bytes(buf) as u64);
      bytes = &bytes[4..];
    }
    if bytes.len() >= 2 {
      let mut buf = [0u8; 2];
      buf.copy_from_slice(&bytes[..2]);
      self.add_to_hash(u16::from_le_bytes(buf) as u64);
      bytes = &bytes[2..];
    }
    if !bytes.is_empty() {
      self.add_to_hash(bytes[0] as u64);
    }
  }

  #[inline]
  fn write_u8(&mut self, i: u8) {
    self.add_to_hash(i as u64);
  }
  #[inline]
  fn write_u16(&mut self, i: u16) {
    self.add_to_hash(i as u64);
  }
  #[inline]
  fn write_u32(&mut self, i: u32) {
    self.add_to_hash(i as u64);
  }
  #[inline]
  fn write_u64(&mut self, i: u64) {
    self.add_to_hash(i);
  }
  #[inline]
  fn write_usize(&mut self, i: usize) {
    self.add_to_hash(i as u64);
  }
  #[inline]
  fn finish(&self) -> u64 {
    self.hash
  }
}

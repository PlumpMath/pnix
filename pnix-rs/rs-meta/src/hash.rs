//! Canonical facet hashes (Plan Phase E2) — zero-dep FNV-1a 64, consolidated
//! from the private copies in native.rs/check.rs. Facet PIPELINES live in
//! witness.rs; this module only hashes given texts/bytes, so it stays pure
//! and dependency-free (evaluated-subset friendly).

pub fn fnv1a_bytes(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h = h ^ ((*b) as u64);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn fnv1a_text(s: &str) -> u64 {
    fnv1a_bytes(s.as_bytes())
}

/// Canonical 16-hex-digit rendering used in receipts and witness records.
pub fn hash_hex(h: u64) -> String {
    format!("{:016x}", h)
}

pub fn text_hash_hex(s: &str) -> String {
    hash_hex(fnv1a_text(s))
}

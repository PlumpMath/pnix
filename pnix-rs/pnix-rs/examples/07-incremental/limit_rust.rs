//! plain Rust의 한계 — 이름 기반 캐시는 alpha-rename에 무효화된다.

use std::collections::HashMap;

fn main() {
    // 이름을 키로 캐시하면, 같은 의미라도 이름이 다르면 캐시 미스.
    let mut cache: HashMap<&str, i64> = HashMap::new();
    cache.insert("total", 42);          // fn total() { ... }
    // 리팩터로 total -> sum 로 rename하면:
    let renamed = "sum";
    println!("rename 후 캐시 히트? {}", cache.contains_key(renamed)); // false
    println!("plain Rust: 이름 기반 캐시는 alpha-rename에 무효 (의미 기반 재사용 없음)");
}

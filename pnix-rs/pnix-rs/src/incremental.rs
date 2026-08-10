//! Definition-granular content addressing + realisation early cutoff (P9).
//!
//! Unison identity model: each top-level `let` definition gets a
//! DEPENDENCY-SUBSTITUTED content hash — references to sibling definitions are
//! replaced by those definitions' hashes before hashing, so a definition's
//! identity depends on its meaning, not on sibling NAMES (alpha-renaming a
//! dependency does not invalidate anything: names are metadata). Mutual
//! recursion hashes as an SCC group: members joined in sorted-name order into
//! one group text (v0 honesty: names *inside* a cycle are part of the group
//! text, so renaming within a cycle changes the group hash — documented
//! boundary, not hidden).
//!
//! Nix-CA realisation model: `realisation record` maps `ir_sha256` (the drv)
//! to the produced `value_sha256` (the out) with a witness hash; a known ir
//! hash short-circuits evaluation entirely (early cutoff). The store is a
//! cache (default `work/realisations.tsv`), not a proof receipt.

use crate::gate;
use crate::interop;
use crate::ir;
use crate::px;
use crate::sha256::sha256_hex;

pub const INCREMENTAL_SCHEMA: &str = "pnix-rs.incremental.v0";
pub const DEFAULT_STORE: &str = "work/realisations.tsv";

/// Dependency-substituted hashes for the top-level let definitions.
/// Returns (name, hash) pairs in source order.
pub fn definition_hashes(source: &str) -> Result<Vec<(String, String)>, String> {
    let ast = px::px_parse(source)?;
    let (bindings, _body) = match &ast {
        px::PxExpr::LetIn { bindings, body } => (bindings.clone(), body),
        _ => return Err(String::from("incremental: top-level expression is not a let")),
    };
    let n = bindings.len();
    let names: Vec<String> = bindings.iter().map(|(name, _)| name.clone()).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            if names[i] == names[j] {
                return Err(format!(
                    "incremental: duplicate definition name {} (shadowing lets are not content-addressable)",
                    names[i]
                ));
            }
        }
    }

    // Sibling reference graph.
    let mut refs: Vec<Vec<usize>> = Vec::new();
    for (_name, expr) in &bindings {
        let mut free = Vec::new();
        let mut bound = Vec::new();
        crate::specialize::px_free_vars(expr, &mut free, &mut bound);
        let mut row = Vec::new();
        for (j, sib) in names.iter().enumerate() {
            if free.iter().any(|f| f == sib) {
                row.push(j);
            }
        }
        refs.push(row);
    }

    // Transitive reachability (small n; O(n^3) is fine).
    let mut reach: Vec<Vec<bool>> = vec![vec![false; n]; n];
    for (i, row) in refs.iter().enumerate() {
        for &j in row {
            reach[i][j] = true;
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            for j in 0..n {
                if reach[i][j] {
                    for k in 0..n {
                        if reach[j][k] && !reach[i][k] {
                            reach[i][k] = true;
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    // SCC id per node: mutual reachability (or self for non-recursive nodes).
    let mut scc_id: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in 0..n {
            if i != j && reach[i][j] && reach[j][i] && scc_id[j] > scc_id[i] {
                scc_id[j] = scc_id[i];
            }
        }
    }

    let canonical =
        |expr: &px::PxExpr, hashes: &Vec<Option<String>>| -> String {
            let substituted = substitute_sibling_refs(expr, &names, hashes);
            px::px_emit(&px::px_normalize(&substituted))
        };

    let mut hashes: Vec<Option<String>> = vec![None; n];
    // Iterate until every definition is hashed (dependencies-first converges
    // because substitution only needs hashes of OTHER SCCs).
    let mut progress = true;
    while progress {
        progress = false;
        for i in 0..n {
            if hashes[i].is_some() {
                continue;
            }
            let group: Vec<usize> = (0..n).filter(|&j| scc_id[j] == scc_id[i]).collect();
            // Ready when every external dependency of the group is hashed.
            let ready = group.iter().all(|&m| {
                refs[m]
                    .iter()
                    .all(|&d| scc_id[d] == scc_id[i] || hashes[d].is_some())
            });
            if !ready {
                continue;
            }
            let self_recursive = group.len() > 1
                || refs[i].iter().any(|&d| d == i);
            if !self_recursive && group.len() == 1 {
                let text = canonical(&bindings[i].1, &hashes);
                hashes[i] = Some(sha256_hex(format!("def:{}", text).as_bytes()));
            } else {
                let mut members: Vec<usize> = group.clone();
                members.sort_by(|a, b| names[*a].cmp(&names[*b]));
                let mut group_text = String::from("scc:");
                for &m in &members {
                    let text = canonical(&bindings[m].1, &hashes);
                    group_text.push_str(&format!("{}={};", names[m], text));
                }
                let group_hash = sha256_hex(group_text.as_bytes());
                for &m in &members {
                    hashes[m] = Some(sha256_hex(
                        format!("{}:{}", group_hash, names[m]).as_bytes(),
                    ));
                }
            }
            progress = true;
        }
    }

    let mut out = Vec::new();
    for i in 0..n {
        match &hashes[i] {
            Some(h) => out.push((names[i].clone(), h.clone())),
            None => return Err(format!("incremental: unhashed definition {}", names[i])),
        }
    }
    Ok(out)
}

/// Replace references to already-hashed siblings with hash markers (for
/// hashing only; the result is never evaluated or re-parsed).
/// Demand-driven change propagation (salsa/adapton early-cutoff, proposal
/// 0007): the set of definition names whose dependency-substituted content
/// hash DIFFERS between two program versions. By construction of the
/// dependency-substituted hash, this is exactly the edited definition plus its
/// transitive DEPENDENTS — independent siblings keep identical hashes (early
/// cutoff: same hash => no recompute). Both versions must declare the same
/// definition names.
pub fn changed_between(source_a: &str, source_b: &str) -> Result<Vec<String>, String> {
    let ha = definition_hashes(source_a)?;
    let hb = definition_hashes(source_b)?;
    let mut changed = Vec::new();
    for (name, hash_a) in &ha {
        let hash_b = hb
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| h.clone());
        match hash_b {
            Some(hb2) if &hb2 == hash_a => {}
            _ => changed.push(name.clone()),
        }
    }
    changed.sort();
    Ok(changed)
}

fn substitute_sibling_refs(
    e: &px::PxExpr,
    names: &[String],
    hashes: &Vec<Option<String>>,
) -> px::PxExpr {
    match e {
        px::PxExpr::DeferredError(_) => e.clone(),
        px::PxExpr::Var(name) => {
            for (j, sib) in names.iter().enumerate() {
                if sib == name {
                    if let Some(h) = &hashes[j] {
                        return px::PxExpr::Var(format!("#{}", h));
                    }
                }
            }
            e.clone()
        }
        px::PxExpr::Int(_) | px::PxExpr::Float(_) | px::PxExpr::Bool(_) | px::PxExpr::Null => e.clone(),
        px::PxExpr::With { .. } => e.clone(),
        px::PxExpr::Str(parts) => px::PxExpr::Str(
            parts
                .iter()
                .map(|p| match p {
                    px::PxStrPart::Lit(s) => px::PxStrPart::Lit(s.clone()),
                    px::PxStrPart::Sub(sub) => {
                        px::PxStrPart::Sub(substitute_sibling_refs(sub, names, hashes))
                    }
                })
                .collect(),
        ),
        px::PxExpr::List(items) => px::PxExpr::List(
            items
                .iter()
                .map(|i| substitute_sibling_refs(i, names, hashes))
                .collect(),
        ),
        px::PxExpr::Select { base, name } => px::PxExpr::Select {
            base: Box::new(substitute_sibling_refs(base, names, hashes)),
            name: name.clone(),
        },
        px::PxExpr::Lambda { param, body } => px::PxExpr::Lambda {
            param: param.clone(),
            body: std::rc::Rc::new(substitute_sibling_refs(body, names, hashes)),
        },
        px::PxExpr::Apply { func, arg } => px::PxExpr::Apply {
            func: Box::new(substitute_sibling_refs(func, names, hashes)),
            arg: Box::new(substitute_sibling_refs(arg, names, hashes)),
        },
        px::PxExpr::If { cond, then_e, else_e } => px::PxExpr::If {
            cond: Box::new(substitute_sibling_refs(cond, names, hashes)),
            then_e: Box::new(substitute_sibling_refs(then_e, names, hashes)),
            else_e: Box::new(substitute_sibling_refs(else_e, names, hashes)),
        },
        px::PxExpr::Binary { op, lhs, rhs } => px::PxExpr::Binary {
            op: op.clone(),
            lhs: Box::new(substitute_sibling_refs(lhs, names, hashes)),
            rhs: Box::new(substitute_sibling_refs(rhs, names, hashes)),
        },
        px::PxExpr::LetIn { bindings, body } => {
            // Inner let names shadow siblings; v0 keeps substitution simple by
            // not descending past a shadowing binder for shadowed names.
            let shadowed: Vec<String> = bindings.iter().map(|(n, _)| n.clone()).collect();
            let filtered: Vec<String> = names
                .iter()
                .filter(|n| !shadowed.iter().any(|s| s == *n))
                .cloned()
                .collect();
            let filtered_hashes: Vec<Option<String>> = names
                .iter()
                .zip(hashes.iter())
                .filter(|(n, _)| !shadowed.iter().any(|s| s == *n))
                .map(|(_, h)| h.clone())
                .collect();
            px::PxExpr::LetIn {
                bindings: bindings
                    .iter()
                    .map(|(n, v)| {
                        (n.clone(), substitute_sibling_refs(v, &filtered, &filtered_hashes))
                    })
                    .collect(),
                body: Box::new(substitute_sibling_refs(body, &filtered, &filtered_hashes)),
            }
        }
        px::PxExpr::Attrs(fields) => px::PxExpr::Attrs(
            fields
                .iter()
                .map(|(n, v)| (n.clone(), substitute_sibling_refs(v, names, hashes)))
                .collect(),
        ),
    }
}

/// Realisation-backed evaluation with early cutoff.
/// Returns (value_sha256, cutoff).
pub fn incremental_eval(
    source: &str,
    store_path: &str,
    granted: &[String],
) -> Result<(String, bool), String> {
    let record = ir::ir_of(source)?;
    let ir_hash = record.ir_sha256;

    let existing = interop::host_read_file(store_path, granted).unwrap_or_default();
    for line in existing.split('\n') {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() == 3 && cols[0] == ir_hash {
            return Ok((String::from(cols[1]), true));
        }
    }

    let value = px::px_run(source)?;
    let value_sha = sha256_hex(value.as_bytes());
    let witness = gate::eval_witness(source, granted)?;
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&format!("{}\t{}\t{}\n", ir_hash, value_sha, witness.out_hash));
    interop::host_write_file(store_path, &updated, granted)?;
    Ok((value_sha, false))
}

/// Verifying cache (proposal, maps pnix-hy 30): on a cache HIT, RE-DERIVE the
/// value and confirm it matches the stored value hash. A plain cache trusts the
/// store (early cutoff); a verifying cache AUDITS it — catching a corrupted or
/// stale entry. Returns (value_hash, verified_hit): verified_hit=true means the
/// cached value was re-checked and agreed; an error means the store lies.
pub fn incremental_eval_verify(
    source: &str,
    store_path: &str,
    granted: &[String],
) -> Result<(String, bool), String> {
    let record = ir::ir_of(source)?;
    let ir_hash = record.ir_sha256;
    let value = px::px_run(source)?;
    let value_sha = sha256_hex(value.as_bytes());

    let existing = interop::host_read_file(store_path, granted).unwrap_or_default();
    for line in existing.split('\n') {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() == 3 && cols[0] == ir_hash {
            if cols[1] == value_sha {
                return Ok((value_sha, true));
            } else {
                return Err(format!(
                    "verifying cache: store entry for {} is {} but re-derived {}",
                    &ir_hash[0..12],
                    cols[1],
                    value_sha
                ));
            }
        }
    }
    Err(format!("verifying cache: no entry for {}", &ir_hash[0..12]))
}

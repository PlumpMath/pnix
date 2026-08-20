//! Rust ↔ px projection for the pnix-rs lane (P6, value axis v0).
//!
//! This lane's own meta-circular projection (the analogue of pnix-hy's
//! hy_mirror axis, with Rust as the host): a px canonical value is projected
//! into a *Rust program* whose leaves are native Rust values (i64 / bool /
//! &str literals) and whose body recomposes the px canonical rendering in
//! Rust. That program is then executed by the rs-meta substrate on BOTH of its
//! tiers (interpreter and rustc), and the roundtrip claim is 3-way equality:
//!
//!   px canonical  ==  rs-meta interp(projected Rust)  ==  rustc(projected Rust)
//!
//! The generated Rust stays inside the rs-meta evaluated subset so both tiers
//! can run it. Values containing opaque leaves (lambda/builtin) are `held`.
//!
//! Explicitly not claimed: this is value projection, not a compiler; the Rust
//! AST structural axis (px attrset ⇄ Rust AST) is held behind
//! `docs/proposals/0001-rust-ast-projection.md`.

use crate::gate;
use crate::interop;
use crate::px;
use crate::sha256::sha256_hex;

pub const RUST_MIRROR_SCHEMA: &str = "pnix-rs.rust-mirror.v0";

/// Rust `&str` literal for a px string (leaf projection).
fn rust_str_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        if c == '\\' {
            out.push_str("\\\\");
        } else if c == '"' {
            out.push_str("\\\"");
        } else if c == '\n' {
            out.push_str("\\n");
        } else if c == '\t' {
            out.push_str("\\t");
        } else {
            out.push(c);
        }
    }
    out.push('"');
    out
}

/// A Rust expression of type String that recomposes the px canonical rendering
/// of `v`, with native Rust literals at the leaves. `needs_escape` is set when
/// any string leaf requires the runtime escape helper.
fn value_to_rust_string_expr(v: &px::PxVal, needs_escape: &mut bool) -> Result<String, String> {
    match v {
        // force then mirror the resolved value (thunks live only in attrset
        // field slots; forcing here recomposes the real leaf)
        px::PxVal::Thunk(_) => value_to_rust_string_expr(&px::px_force(v)?, needs_escape),
        px::PxVal::Bytes(_) => Err(String::from(
            "raw-byte strings have no canonical text form (held)",
        )),
        px::PxVal::Int(n) => Ok(format!("format!(\"{{}}\", {}i64)", n)),
        px::PxVal::Float(f) => {
            let x = *f;
            if x - x == 0.0 {
                Ok(format!("format!(\"{{:?}}\", {:?})", f))
            } else {
                Err(String::from("held: non-finite float leaf"))
            }
        }
        px::PxVal::Bool(b) => Ok(format!("format!(\"{{}}\", {})", b)),
        px::PxVal::Null => Ok(String::from("\"null\".to_string()")),
        px::PxVal::Str(s) => {
            *needs_escape = true;
            Ok(format!(
                "format!(\"\\\"{{}}\\\"\", px_escape({}))",
                rust_str_literal(s)
            ))
        }
        px::PxVal::List(items) => {
            if items.is_empty() {
                return Ok(String::from("String::from(\"[ ]\")"));
            }
            let mut out = String::from("{ let mut out = String::from(\"[ \"); ");
            for item in items.iter() {
                let piece = value_to_rust_string_expr(item, needs_escape)?;
                out.push_str(&format!("out.push_str(&{}); out.push(' '); ", piece));
            }
            out.push_str("out.push(']'); out }");
            Ok(out)
        }
        px::PxVal::Attrs(fields) => {
            // px_print sorts keys; project in already-sorted order so the Rust
            // side recomposes the same canonical text.
            let mut sorted: Vec<(String, &px::PxVal)> = Vec::new();
            for (k, val) in fields.iter() {
                // __pnix_attr_pos is unsafeGetAttrPos's internal per-field
                // position side-table (px.rs's px_is_attr_pos_key); px_print
                // hides it from canonical text, so the projected Rust
                // program must hide it too or the 3-way check never agrees.
                if k == "__pnix_attr_pos" {
                    continue;
                }
                sorted.push((k.clone(), val));
            }
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = String::from("{ let mut out = String::from(\"{ \"); ");
            for (k, val) in sorted {
                let piece = value_to_rust_string_expr(val, needs_escape)?;
                out.push_str(&format!(
                    "out.push_str({}); ",
                    rust_str_literal(&format!("{} = ", k))
                ));
                out.push_str(&format!("out.push_str(&{}); out.push_str(\"; \"); ", piece));
            }
            out.push_str("out.push('}'); out }");
            Ok(out)
        }
        px::PxVal::Closure { .. } => Err(String::from("held: opaque leaf (lambda)")),
        px::PxVal::Builtin { .. } => Err(String::from("held: opaque leaf (builtin)")),
        // px_print renders a path unquoted (unlike a string); project the
        // same plain (Rust-escaped) literal, with no surrounding quotes in
        // the recomposed text.
        px::PxVal::Path(p) => Ok(format!("{}.to_string()", rust_str_literal(p))),
    }
}

/// The px-escape helper emitted into projected programs (mirror of
/// `px_escape_string`), kept inside the rs-meta evaluated subset.
const RUST_ESCAPE_HELPER: &str = "fn px_escape(s: &str) -> String {\n\
    let mut out = String::new();\n\
    for c in s.chars() {\n\
        if c == '\\\\' { out.push_str(\"\\\\\\\\\"); }\n\
        else if c == '\"' { out.push_str(\"\\\\\\\"\"); }\n\
        else if c == '\\n' { out.push_str(\"\\\\n\"); }\n\
        else if c == '\\t' { out.push_str(\"\\\\t\"); }\n\
        else { out.push(c); }\n\
    }\n\
    out\n\
}\n";

/// Full projected Rust program printing the px canonical rendering of `v`.
pub fn px_value_to_rust_print_program(v: &px::PxVal) -> Result<String, String> {
    let mut needs_escape = false;
    let expr = value_to_rust_string_expr(v, &mut needs_escape)?;
    let mut out = String::new();
    if needs_escape {
        out.push_str(RUST_ESCAPE_HELPER);
    }
    out.push_str(&format!("fn main() {{ println!(\"{{}}\", {}); }}\n", expr));
    Ok(out)
}

pub struct RustMirrorRecord {
    pub schema: &'static str,
    pub status: &'static str,
    pub px_value: String,
    pub program: Option<String>,
    pub witness: gate::Witness,
}

fn projection_witness(
    px_source: &str,
    program: &str,
    status: &'static str,
    granted: &[String],
) -> gate::Witness {
    let mut sorted: Vec<String> = granted.to_vec();
    sorted.sort();
    gate::Witness {
        direction: String::from("rust-projection"),
        source_lang: String::from("px"),
        target_lang: String::from("rust"),
        input_kind: String::from("canonical-value"),
        output_kind: String::from("rust-print-program"),
        loss_status: String::from(status),
        effect_class: String::from("host-call"),
        capability_required: String::from("host-call"),
        in_hash: sha256_hex(px_source.as_bytes()),
        out_hash: sha256_hex(program.as_bytes()),
        env_hash: sha256_hex(format!("granted={}", sorted.join(",")).as_bytes()),
        status: if status == "lossless" {
            String::from("ok")
        } else {
            String::from(status)
        },
        loss: String::from("none"),
    }
}

/// Value-axis roundtrip: evaluate px, project the value into Rust, run the
/// projection on both substrate tiers, require 3-way canonical equality.
pub fn rust_value_roundtrip(
    px_source: &str,
    bootstrap: &str,
    granted: &[String],
) -> Result<RustMirrorRecord, String> {
    let ast = px::px_parse(px_source)?;
    let env = Vec::new();
    let value = px::px_eval(&ast, &env)?;
    let canonical = px::px_print(&value);

    let program = match px_value_to_rust_print_program(&value) {
        Ok(p) => p,
        Err(held) => {
            return Ok(RustMirrorRecord {
                schema: RUST_MIRROR_SCHEMA,
                status: "held",
                px_value: canonical.clone(),
                program: None,
                witness: projection_witness(px_source, &held, "held", granted),
            })
        }
    };

    let interp_out = interop::host_run_bootstrap_inline(bootstrap, "run", &program, granted)?;
    let native_out =
        interop::host_run_bootstrap_inline(bootstrap, "native-run", &program, granted)?;
    let three_way =
        interp_out.trim() == canonical && native_out.trim() == canonical;
    let status = if three_way { "lossless" } else { "rejected" };
    Ok(RustMirrorRecord {
        schema: RUST_MIRROR_SCHEMA,
        status,
        px_value: canonical,
        program: Some(program.clone()),
        witness: projection_witness(px_source, &program, status, granted),
    })
}

pub fn render(record: &RustMirrorRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", record.schema));
    out.push_str(&format!("px_value {}\n", record.px_value));
    match &record.program {
        Some(p) => out.push_str(&format!("program_sha256 {}\n", sha256_hex(p.as_bytes()))),
        None => out.push_str("program_sha256 -\n"),
    }
    out.push_str(&format!("status {}\n", record.status));
    out.push_str(&gate::render_witness(&record.witness));
    out
}

// ---- AST axis (v1): rs-meta canonical AST sig-tree <-> px data ------------------
//
// `rs-meta ast-canonical` emits a stable serialization of the Rust AST (its
// stability is proven by rs-meta's stage3 mirror: byte-identical across three
// evaluation levels). This axis reifies that serialization's bracket tree as a
// first-class px value, and requires two roundtrips:
//
//   1. regeneration: px tree -> text == original sig text (structure held), and
//   2. px embedding: the tree value survives px_print -> px_parse -> px_eval
//      (the reified Rust AST lives as genuine px data through the sacred
//      runtime).
//
// The tokenizer is quote-aware (sig strings are double-quoted with escapes).
// Held edge (documented): a Rust `char` literal whose character is an
// unbalanced bracket (e.g. '(') renders raw in the sig format and would break
// bracket pairing — such sources are rejected as held, not mis-parsed.

enum SigNode {
    Text(String),
    Group { open: char, items: Vec<SigNode> },
}

fn close_of(open: char) -> char {
    if open == '(' {
        ')'
    } else if open == '{' {
        '}'
    } else {
        ']'
    }
}

fn sig_parse_items(
    chars: &[char],
    mut i: usize,
    closer: Option<char>,
) -> Result<(Vec<SigNode>, usize), String> {
    let mut items = Vec::new();
    let mut text = String::new();
    while i < chars.len() {
        let c = chars[i];
        if let Some(cl) = closer {
            if c == cl {
                if !text.is_empty() {
                    items.push(SigNode::Text(text));
                }
                return Ok((items, i + 1));
            }
        }
        if c == '"' {
            // Quote-aware: consume the whole string literal as text.
            text.push(c);
            i += 1;
            while i < chars.len() {
                let sc = chars[i];
                text.push(sc);
                i += 1;
                if sc == '\\' {
                    if i < chars.len() {
                        text.push(chars[i]);
                        i += 1;
                    }
                } else if sc == '"' {
                    break;
                }
            }
        } else if c == '(' || c == '{' || c == '[' {
            if !text.is_empty() {
                items.push(SigNode::Text(text));
                text = String::new();
            }
            let (children, next) = sig_parse_items(chars, i + 1, Some(close_of(c)))?;
            items.push(SigNode::Group { open: c, items: children });
            i = next;
        } else if c == ')' || c == '}' || c == ']' {
            return Err(format!("sig tree: unbalanced closer {}", c));
        } else {
            text.push(c);
            i += 1;
        }
    }
    if closer.is_some() {
        return Err(String::from("sig tree: unterminated group"));
    }
    if !text.is_empty() {
        items.push(SigNode::Text(text));
    }
    Ok((items, i))
}

fn sig_nodes_to_px(nodes: &[SigNode]) -> px::PxVal {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            SigNode::Text(t) => out.push(px::px_attrs(vec![
                (String::from("kind"), px::PxVal::Str(String::from("text"))),
                (String::from("text"), px::PxVal::Str(t.clone())),
            ])),
            SigNode::Group { open, items } => out.push(px::px_attrs(vec![
                (String::from("kind"), px::PxVal::Str(String::from("group"))),
                (String::from("open"), px::PxVal::Str(open.to_string())),
                (String::from("items"), sig_nodes_to_px(items)),
            ])),
        }
    }
    px::px_list(out)
}

/// Reify a canonical AST sig text as a px value (bracket tree).
pub fn sig_tree_to_px(text: &str) -> Result<px::PxVal, String> {
    let chars: Vec<char> = text.chars().collect();
    let (items, _end) = sig_parse_items(&chars, 0, None)?;
    Ok(sig_nodes_to_px(&items))
}

fn attr_get<'a>(fields: &'a [(String, px::PxVal)], name: &str) -> Result<&'a px::PxVal, String> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
        .ok_or_else(|| format!("sig tree: missing field {}", name))
}

/// Recompose the exact sig text from the px tree value.
pub fn px_to_sig_text(v: &px::PxVal) -> Result<String, String> {
    let items = match v {
        px::PxVal::List(items) => items,
        _ => return Err(String::from("sig tree: expected a list")),
    };
    let mut out = String::new();
    for item in items.iter() {
        let fields = match item {
            px::PxVal::Attrs(fields) => fields,
            _ => return Err(String::from("sig tree: expected attrset nodes")),
        };
        let kind = match attr_get(fields, "kind")? {
            px::PxVal::Str(s) => s.clone(),
            _ => return Err(String::from("sig tree: kind must be a string")),
        };
        if kind == "text" {
            match attr_get(fields, "text")? {
                px::PxVal::Str(s) => out.push_str(s),
                _ => return Err(String::from("sig tree: text must be a string")),
            }
        } else if kind == "group" {
            let open = match attr_get(fields, "open")? {
                px::PxVal::Str(s) if s.chars().count() == 1 => match s.chars().next() {
                    Some(c) => c,
                    None => return Err(String::from("sig tree: empty open")),
                },
                _ => return Err(String::from("sig tree: open must be one char")),
            };
            out.push(open);
            out.push_str(&px_to_sig_text(attr_get(fields, "items")?)?);
            out.push(close_of(open));
        } else {
            return Err(format!("sig tree: unknown kind {}", kind));
        }
    }
    Ok(out)
}

pub struct RustAstRecord {
    pub schema: &'static str,
    pub status: &'static str,
    pub sig_sha256: String,
    pub regen_match: bool,
    pub px_embed_match: bool,
    pub witness: gate::Witness,
}

/// AST-axis roundtrip: Rust source -> rs-meta canonical AST sig -> px tree
/// value -> (a) regenerated sig text, (b) px-embedded data roundtrip.
pub fn rust_ast_roundtrip(
    rust_source: &str,
    bootstrap: &str,
    granted: &[String],
) -> Result<RustAstRecord, String> {
    let sig_text =
        interop::host_run_bootstrap_inline(bootstrap, "ast-canonical", rust_source, granted)?;
    let tree = sig_tree_to_px(&sig_text)?;
    let regen = px_to_sig_text(&tree)?;
    let regen_match = regen == sig_text;

    // The reified tree must live as genuine px data: its canonical print is
    // valid px source whose evaluation prints identically.
    let printed = px::px_print(&tree);
    let px_embed_match = match px::px_run(&printed) {
        Ok(reprinted) => reprinted == printed,
        Err(_) => false,
    };

    let status = if regen_match && px_embed_match {
        "lossless"
    } else {
        "rejected"
    };
    let mut sorted: Vec<String> = granted.to_vec();
    sorted.sort();
    let witness = gate::Witness {
        direction: String::from("rust-ast-projection"),
        source_lang: String::from("rust"),
        target_lang: String::from("px"),
        input_kind: String::from("canonical-ast-sig"),
        output_kind: String::from("px-tree-value"),
        loss_status: String::from(status),
        effect_class: String::from("host-call"),
        capability_required: String::from("host-call"),
        in_hash: sha256_hex(sig_text.as_bytes()),
        out_hash: sha256_hex(printed.as_bytes()),
        env_hash: sha256_hex(format!("granted={}", sorted.join(",")).as_bytes()),
        status: if status == "lossless" {
            String::from("ok")
        } else {
            String::from(status)
        },
        loss: String::from("none"),
    };
    Ok(RustAstRecord {
        schema: RUST_MIRROR_SCHEMA,
        status,
        sig_sha256: sha256_hex(sig_text.as_bytes()),
        regen_match,
        px_embed_match,
        witness,
    })
}

// ---- AST axis v2: typed-kind encoding of core sig expressions ------------------
//
// The v1a bracket tree holds STRUCTURE; v2 types the core EXPRESSION nodes of
// the sig format (int/var/bin/if — the kinds that align 1:1 with the P11
// tower encoding) as tagged px attrsets rint/rvar/rbin/rif, with two claims:
//
//   1. regeneration: typed node -> sig text is byte-identical to the input;
//   2. the tower JOIN: the px-written translator (runtime/tower/rust_bridge.px)
//      maps typed Rust nodes into the tower encoding, the px self-interpreter
//      evaluates them, and the result must equal what rustc prints for the
//      same Rust expression (via the rs-meta native tier).
//
// Everything outside the core (calls, blocks with statements, items) is a
// held error here — v1a's bracket tree remains the full-coverage axis.

struct SigCursor<'a> {
    chars: &'a [char],
    pos: usize,
}

impl<'a> SigCursor<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn eat(&mut self, c: char) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "sig typed: expected {} at {} (found {:?})",
                c, self.pos, self.peek()
            ))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                out.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        if out.is_empty() {
            Err(format!("sig typed: expected ident at {}", self.pos))
        } else {
            Ok(out)
        }
    }
    fn integer(&mut self) -> Result<i64, String> {
        let mut out = String::new();
        if self.peek() == Some('-') {
            out.push('-');
            self.pos += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                out.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        out.parse::<i64>()
            .map_err(|_| format!("sig typed: bad integer at {}", self.pos))
    }
}

fn sig_typed_expr(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let head = cur.ident()?;
    if head == "int" {
        cur.eat('(')?;
        let n = cur.integer()?;
        cur.eat(')')?;
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("rint"))),
            (String::from("value"), px::PxVal::Int(n)),
        ]))
    } else if head == "var" {
        cur.eat('(')?;
        let name = cur.ident()?;
        cur.eat(')')?;
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("rvar"))),
            (String::from("name"), px::PxVal::Str(name)),
        ]))
    } else if head == "bin" {
        cur.eat('(')?;
        let op = cur.ident()?;
        cur.eat(',')?;
        let lhs = sig_typed_expr(cur)?;
        cur.eat(',')?;
        let rhs = sig_typed_expr(cur)?;
        cur.eat(')')?;
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("rbin"))),
            (String::from("op"), px::PxVal::Str(op)),
            (String::from("lhs"), lhs),
            (String::from("rhs"), rhs),
        ]))
    } else if head == "if" {
        cur.eat('(')?;
        let cond = sig_typed_expr(cur)?;
        cur.eat(',')?;
        let then_e = sig_typed_block(cur)?;
        cur.eat(',')?;
        let else_e = sig_typed_block(cur)?;
        cur.eat(')')?;
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("rif"))),
            (String::from("cond"), cond),
            (String::from("then_e"), then_e),
            (String::from("else_e"), else_e),
        ]))
    } else {
        Err(format!("held: sig node kind {} is outside the typed core", head))
    }
}

/// Pure-expression block `{|expr}` (statement blocks are held).
fn sig_typed_block(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    cur.eat('{')?;
    cur.eat('|')?;
    let inner = sig_typed_expr(cur)?;
    cur.eat('}')?;
    Ok(inner)
}

/// Parse a core sig expression into typed px nodes.
pub fn sig_typed_parse(text: &str) -> Result<px::PxVal, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut cur = SigCursor { chars: &chars, pos: 0 };
    let node = sig_typed_expr(&mut cur)?;
    if cur.pos != chars.len() {
        return Err(format!("sig typed: trailing input at {}", cur.pos));
    }
    Ok(node)
}

fn typed_field<'a>(v: &'a px::PxVal, name: &str) -> Result<&'a px::PxVal, String> {
    match v {
        px::PxVal::Attrs(fields) => fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, val)| val)
            .ok_or_else(|| format!("sig typed: missing field {}", name)),
        _ => Err(String::from("sig typed: node is not an attrset")),
    }
}

fn typed_field_str(v: &px::PxVal, name: &str) -> Result<String, String> {
    match typed_field(v, name)? {
        px::PxVal::Str(s) => Ok(s.clone()),
        _ => Err(format!("sig typed: field {} is not a string", name)),
    }
}

/// Regenerate the exact sig text from a typed node (inverse of sig_typed_parse).
pub fn sig_typed_render(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    if kind == "rint" {
        match typed_field(v, "value")? {
            px::PxVal::Int(n) => Ok(format!("int({})", n)),
            _ => Err(String::from("sig typed: rint value")),
        }
    } else if kind == "rvar" {
        Ok(format!("var({})", typed_field_str(v, "name")?))
    } else if kind == "rbin" {
        Ok(format!(
            "bin({},{},{})",
            typed_field_str(v, "op")?,
            sig_typed_render(typed_field(v, "lhs")?)?,
            sig_typed_render(typed_field(v, "rhs")?)?
        ))
    } else if kind == "rif" {
        Ok(format!(
            "if({},{{|{}}},{{|{}}})",
            sig_typed_render(typed_field(v, "cond")?)?,
            sig_typed_render(typed_field(v, "then_e")?)?,
            sig_typed_render(typed_field(v, "else_e")?)?
        ))
    } else {
        Err(format!("sig typed: unknown kind {}", kind))
    }
}

pub struct RustJoinRecord {
    pub typed_roundtrip: bool,
    pub rustc_out: String,
    pub self_interp_out: String,
    pub witness: gate::Witness,
}

/// P6 v2 join: a Rust EXPRESSION goes through rs-meta ast-canonical, the core
/// sig-typed parser, the px-written bridge (rust_bridge.px), and the px
/// self-interpreter — and the result must equal what rustc prints for the
/// same expression (rs-meta native tier). Three substrates agree on one
/// Rust expression: rustc == rs-meta interp (by rs-meta's own TV) == px tower.
pub fn rust_expr_join(
    rust_expr: &str,
    bootstrap: &str,
    granted: &[String],
) -> Result<RustJoinRecord, String> {
    let probe = format!("fn probe() -> i64 {{ {} }}", rust_expr);
    let sig = interop::host_run_bootstrap_inline(bootstrap, "ast-canonical", &probe, granted)?;
    let sig = sig.trim();
    let prefix = "fn probe()->i64 {|";
    let suffix = "};";
    if !sig.starts_with(prefix) || !sig.ends_with(suffix) {
        return Err(format!("sig typed: unexpected wrapper shape {}", sig));
    }
    let expr_sig = &sig[prefix.len()..sig.len() - suffix.len()];

    let typed = sig_typed_parse(expr_sig)?;
    let regen = sig_typed_render(&typed)?;
    let typed_roundtrip = regen == expr_sig;

    let bridge_src = interop::host_read_file("runtime/tower/rust_bridge.px", granted)?;
    let translate_call = format!("({}) {}", bridge_src.trim(), px::px_print(&typed));
    let translated = px::px_run(&translate_call)?;
    let self_interp_out = crate::tower::self_interp_eval_encoded(&translated, granted)?;

    let main_program = format!("fn main() {{ println!(\"{{}}\", {}); }}", rust_expr);
    let rustc_out = interop::host_run_bootstrap_inline(bootstrap, "native-run", &main_program, granted)?
        .trim()
        .to_string();

    let ok = typed_roundtrip && rustc_out == self_interp_out;
    let mut sorted: Vec<String> = granted.to_vec();
    sorted.sort();
    let witness = gate::Witness {
        direction: String::from("rust-typed-projection"),
        source_lang: String::from("rust"),
        target_lang: String::from("px"),
        input_kind: String::from("canonical-ast-sig-expr"),
        output_kind: String::from("tower-encoded-node"),
        loss_status: String::from(if ok { "lossless" } else { "rejected" }),
        effect_class: String::from("host-call"),
        capability_required: String::from("host-call"),
        in_hash: sha256_hex(expr_sig.as_bytes()),
        out_hash: sha256_hex(self_interp_out.as_bytes()),
        env_hash: sha256_hex(format!("granted={}", sorted.join(",")).as_bytes()),
        status: String::from(if ok { "ok" } else { "rejected" }),
        loss: String::from("none"),
    };
    Ok(RustJoinRecord {
        typed_roundtrip,
        rustc_out,
        self_interp_out,
        witness,
    })
}

// ---- AST axis v3: whole-program typed core + px -> Rust reconstruction ---------
//
// v3 extends the typed core from expressions to PROGRAMS (fn items, call,
// blocks with `ex`/`let` statements, println) and adds the REVERSE joint:
// a Rust source RENDERER over the typed px tree. Acceptance is AST identity
// through rs-meta itself — ast-canonical(rendered) == ast-canonical(original)
// — plus rustc output parity. Items outside the core (structs/enums/impl/
// match/mut/loops) hold; the v1a bracket tree remains the full-coverage axis.

fn typed_kv(kind: &str, fields: Vec<(String, px::PxVal)>) -> px::PxVal {
    let mut all = vec![(String::from("kind"), px::PxVal::Str(String::from(kind)))];
    for f in fields {
        all.push(f);
    }
    px::px_attrs(all)
}

impl<'a> SigCursor<'a> {
    fn eat_str(&mut self, s: &str) -> Result<(), String> {
        for c in s.chars() {
            self.eat(c)?;
        }
        Ok(())
    }
    fn peek_is(&self, c: char) -> bool {
        self.peek() == Some(c)
    }
    /// Quoted string, verbatim inner text (sig_esc-escaped — the same
    /// escaping Rust literals use for our core).
    fn quoted(&mut self) -> Result<String, String> {
        self.eat('"')?;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c == '\\' {
                out.push(c);
                self.pos += 1;
                if let Some(n) = self.peek() {
                    out.push(n);
                    self.pos += 1;
                }
            } else if c == '"' {
                self.pos += 1;
                return Ok(out);
            } else {
                out.push(c);
                self.pos += 1;
            }
        }
        Err(String::from("sig typed: unterminated string"))
    }
}

/// Core expr extended with call/println (v3).
fn sig_typed_expr_v3(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let save = cur.pos;
    let head = cur.ident()?;
    if head == "call" {
        cur.eat('(')?;
        let name = cur.ident()?;
        let mut args = Vec::new();
        cur.eat(',')?;
        cur.eat('[')?;
        while !cur.peek_is(']') {
            args.push(sig_typed_expr_v3(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rcall",
            vec![
                (String::from("name"), px::PxVal::Str(name)),
                (String::from("args"), px::px_list(args)),
            ],
        ))
    } else if head == "println" {
        cur.eat('(')?;
        let fmt = cur.quoted()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut args = Vec::new();
        while !cur.peek_is(']') {
            args.push(sig_typed_expr_v3(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rprintln",
            vec![
                (String::from("fmt"), px::PxVal::Str(fmt)),
                (String::from("args"), px::px_list(args)),
            ],
        ))
    } else if head == "if" {
        cur.eat('(')?;
        let cond = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let then_b = sig_typed_block_v3(cur)?;
        cur.eat(',')?;
        // else is a block, or `_` for an if-without-else (v6).
        let else_b = if cur.peek_is('_') {
            cur.eat('_')?;
            typed_kv("rnoelse", vec![])
        } else {
            sig_typed_block_v3(cur)?
        };
        cur.eat(')')?;
        Ok(typed_kv(
            "rif",
            vec![
                (String::from("cond"), cond),
                (String::from("then_e"), then_b),
                (String::from("else_e"), else_b),
            ],
        ))
    } else if head == "bin" {
        // v3-recursive bin (the v2 arm would not see call/println operands).
        cur.eat('(')?;
        let op = cur.ident()?;
        cur.eat(',')?;
        let lhs = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let rhs = sig_typed_expr_v3(cur)?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rbin",
            vec![
                (String::from("op"), px::PxVal::Str(op)),
                (String::from("lhs"), lhs),
                (String::from("rhs"), rhs),
            ],
        ))
    } else if head == "field" {
        // v4: `field(base,name)`.
        cur.eat('(')?;
        let base = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let name = cur.ident()?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rfield",
            vec![
                (String::from("base"), base),
                (String::from("name"), px::PxVal::Str(name)),
            ],
        ))
    } else if head == "slit" {
        // v4: `slit(Type,[name:expr,...])`.
        cur.eat('(')?;
        let ty = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut fields = Vec::new();
        while !cur.peek_is(']') {
            let fname = cur.ident()?;
            cur.eat(':')?;
            let value = sig_typed_expr_v3(cur)?;
            fields.push(px::px_attrs(vec![
                (String::from("name"), px::PxVal::Str(fname)),
                (String::from("value"), value),
            ]));
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rslit",
            vec![
                (String::from("ty"), px::PxVal::Str(ty)),
                (String::from("fields"), px::px_list(fields)),
            ],
        ))
    } else if head == "pcall" {
        // v4: `pcall(Type,method,[args])`.
        cur.eat('(')?;
        let ty = cur.ident()?;
        cur.eat(',')?;
        let method = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut args = Vec::new();
        while !cur.peek_is(']') {
            args.push(sig_typed_expr_v3(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rpcall",
            vec![
                (String::from("ty"), px::PxVal::Str(ty)),
                (String::from("method"), px::PxVal::Str(method)),
                (String::from("args"), px::px_list(args)),
            ],
        ))
    } else if head == "mcall" {
        // v4: `mcall(recv,method,[typeargs],[args])`; typeargs must be empty
        // (turbofish held).
        cur.eat('(')?;
        let recv = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let method = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        if !cur.peek_is(']') {
            return Err(String::from("held: method turbofish type arguments"));
        }
        cur.eat(']')?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut args = Vec::new();
        while !cur.peek_is(']') {
            args.push(sig_typed_expr_v3(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rmcall",
            vec![
                (String::from("recv"), recv),
                (String::from("method"), px::PxVal::Str(method)),
                (String::from("args"), px::px_list(args)),
            ],
        ))
    } else if head == "match" {
        // v5: `match(scrut,[pat=>body,...])`.
        cur.eat('(')?;
        let scrut = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut arms = Vec::new();
        while !cur.peek_is(']') {
            let pat = sig_typed_pattern(cur)?;
            cur.eat_str("=>")?;
            let body = sig_typed_expr_v3(cur)?;
            arms.push(px::px_attrs(vec![
                (String::from("pat"), pat),
                (String::from("body"), body),
            ]));
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rmatch",
            vec![
                (String::from("scrut"), scrut),
                (String::from("arms"), px::px_list(arms)),
            ],
        ))
    } else if head == "assign" {
        // v6: `assign(place,value)`.
        cur.eat('(')?;
        let place = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let value = sig_typed_expr_v3(cur)?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rassign",
            vec![
                (String::from("place"), place),
                (String::from("value"), value),
            ],
        ))
    } else if head == "while" {
        // v6: `while(cond,block)`.
        cur.eat('(')?;
        let cond = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let body = sig_typed_block_v3(cur)?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rwhile",
            vec![
                (String::from("cond"), cond),
                (String::from("body"), body),
            ],
        ))
    } else if head == "foreach" {
        // v6: `foreach(pattern,iter,block)`.
        cur.eat('(')?;
        let pat = sig_typed_pattern(cur)?;
        cur.eat(',')?;
        let iter = sig_typed_expr_v3(cur)?;
        cur.eat(',')?;
        let body = sig_typed_block_v3(cur)?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rforeach",
            vec![
                (String::from("pat"), pat),
                (String::from("iter"), iter),
                (String::from("body"), body),
            ],
        ))
    } else if head == "un" {
        // v6: `un(op,expr)` — currently deref (`*x`).
        cur.eat('(')?;
        let op = cur.ident()?;
        cur.eat(',')?;
        let expr = sig_typed_expr_v3(cur)?;
        cur.eat(')')?;
        Ok(typed_kv(
            "runary",
            vec![
                (String::from("op"), px::PxVal::Str(op)),
                (String::from("expr"), expr),
            ],
        ))
    } else if head == "ref" {
        // v6: `ref(imm,expr)` / `ref(mut,expr)`.
        cur.eat('(')?;
        let mode = cur.ident()?;
        cur.eat(',')?;
        let expr = sig_typed_expr_v3(cur)?;
        cur.eat(')')?;
        Ok(typed_kv(
            "rref",
            vec![
                (String::from("mode"), px::PxVal::Str(mode)),
                (String::from("expr"), expr),
            ],
        ))
    } else if head == "eslit" {
        // v5: `eslit(Enum,Variant,[name:value,...])` — struct-variant build.
        cur.eat('(')?;
        let en = cur.ident()?;
        cur.eat(',')?;
        let variant = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut fields = Vec::new();
        while !cur.peek_is(']') {
            let fname = cur.ident()?;
            cur.eat(':')?;
            let value = sig_typed_expr_v3(cur)?;
            fields.push(px::px_attrs(vec![
                (String::from("name"), px::PxVal::Str(fname)),
                (String::from("value"), value),
            ]));
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "reslit",
            vec![
                (String::from("enum"), px::PxVal::Str(en)),
                (String::from("variant"), px::PxVal::Str(variant)),
                (String::from("fields"), px::px_list(fields)),
            ],
        ))
    } else {
        cur.pos = save;
        // int/var leaves via the v2 core.
        sig_typed_expr(cur)
    }
}

/// Block `{ stmt;stmt;...|tail }`; zero statements + tail = `{|expr}`.
/// Returns rblock { stmts; tail } (tail = expr node or rnotail).
fn sig_typed_block_v3(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    cur.eat('{')?;
    let mut stmts = Vec::new();
    loop {
        if cur.peek_is('|') {
            cur.eat('|')?;
            break;
        }
        let save = cur.pos;
        let head = cur.ident()?;
        if head == "ex" {
            cur.eat(' ')?;
            stmts.push(typed_kv(
                "rex",
                vec![(String::from("expr"), sig_typed_expr_v3(cur)?)],
            ));
        } else if head == "let" {
            cur.eat(' ')?;
            // `let mut NAME:_=` (v6) — `mut` is a keyword, never a var name.
            let first = cur.ident()?;
            let (is_mut, name) = if first == "mut" {
                cur.eat(' ')?;
                (true, cur.ident()?)
            } else {
                (false, first)
            };
            cur.eat_str(":_=")?;
            stmts.push(typed_kv(
                "rlet",
                vec![
                    (String::from("name"), px::PxVal::Str(name)),
                    (String::from("mut"), px::PxVal::Bool(is_mut)),
                    (String::from("expr"), sig_typed_expr_v3(cur)?),
                ],
            ));
        } else {
            cur.pos = save;
            return Err(format!("held: sig statement kind {} is outside the typed core", head));
        }
        if cur.peek_is(';') {
            cur.eat(';')?;
        }
    }
    let tail = if cur.peek_is('_') {
        cur.eat('_')?;
        typed_kv("rnotail", vec![])
    } else {
        sig_typed_expr_v3(cur)?
    };
    cur.eat('}')?;
    Ok(typed_kv(
        "rblock",
        vec![
            (String::from("stmts"), px::px_list(stmts)),
            (String::from("tail"), tail),
        ],
    ))
}

/// A type in sig form: `i64` (prim ident) or `N(Name)` (named). v4.
fn sig_typed_type(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let head = cur.ident()?;
    if head == "N" {
        cur.eat('(')?;
        let name = cur.ident()?;
        cur.eat(')')?;
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("named"))),
            (String::from("name"), px::PxVal::Str(name)),
        ]))
    } else if head == "G" {
        // v7: generic type `G(Name,[arg,...])` -> `Name<arg,...>`.
        cur.eat('(')?;
        let name = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut args = Vec::new();
        while !cur.peek_is(']') {
            args.push(sig_typed_type(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("generic"))),
            (String::from("name"), px::PxVal::Str(name)),
            (String::from("args"), px::px_list(args)),
        ]))
    } else if cur.peek_is('(') {
        Err(format!("held: parameterized type {}", head))
    } else {
        Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("prim"))),
            (String::from("name"), px::PxVal::Str(head)),
        ]))
    }
}

/// Optional `<T,U>` generic parameter list after a name/impl (v7). Returns the
/// list of names (empty when absent). Cursor must be at `<` or elsewhere.
fn sig_typed_generics(cur: &mut SigCursor) -> Result<Vec<px::PxVal>, String> {
    let mut out = Vec::new();
    if cur.peek_is('<') {
        cur.eat('<')?;
        while !cur.peek_is('>') {
            out.push(px::PxVal::Str(cur.ident()?));
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat('>')?;
    }
    Ok(out)
}

/// fn/method parameter list `(name:type,...)`.
fn sig_typed_params(cur: &mut SigCursor) -> Result<Vec<px::PxVal>, String> {
    cur.eat('(')?;
    let mut params = Vec::new();
    while !cur.peek_is(')') {
        let pname = cur.ident()?;
        cur.eat(':')?;
        let ty = sig_typed_type(cur)?;
        params.push(px::px_attrs(vec![
            (String::from("name"), px::PxVal::Str(pname)),
            (String::from("ty"), ty),
        ]));
        if cur.peek_is(',') {
            cur.eat(',')?;
        }
    }
    cur.eat(')')?;
    Ok(params)
}

/// One impl method: `name[recv](params)->ret {block}`.
fn sig_typed_method(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let name = cur.ident()?;
    let generics = sig_typed_generics(cur)?;
    cur.eat('[')?;
    // recv is assoc / &self / self / &mut self — read until ']'.
    let mut recv = String::new();
    while !cur.peek_is(']') {
        match cur.peek() {
            Some(c) => {
                recv.push(c);
                cur.pos += 1;
            }
            None => return Err(String::from("sig typed: unterminated method receiver")),
        }
    }
    cur.eat(']')?;
    let params = sig_typed_params(cur)?;
    cur.eat_str("->")?;
    let ret = sig_typed_type(cur)?;
    cur.eat(' ')?;
    let body = sig_typed_block_v3(cur)?;
    Ok(typed_kv(
        "rmethod",
        vec![
            (String::from("name"), px::PxVal::Str(name)),
            (String::from("generics"), px::px_list(generics)),
            (String::from("recv"), px::PxVal::Str(recv)),
            (String::from("params"), px::px_list(params)),
            (String::from("ret"), ret),
            (String::from("body"), body),
        ],
    ))
}

/// One enum variant `Name(tuple_types)[struct_fields]` (v5).
fn sig_typed_variant(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let name = cur.ident()?;
    cur.eat('(')?;
    let mut tuple = Vec::new();
    while !cur.peek_is(')') {
        tuple.push(sig_typed_type(cur)?);
        if cur.peek_is(',') {
            cur.eat(',')?;
        }
    }
    cur.eat(')')?;
    cur.eat('[')?;
    let mut fields = Vec::new();
    while !cur.peek_is(']') {
        let fname = cur.ident()?;
        cur.eat(':')?;
        let ty = sig_typed_type(cur)?;
        fields.push(px::px_attrs(vec![
            (String::from("name"), px::PxVal::Str(fname)),
            (String::from("ty"), ty),
        ]));
        if cur.peek_is(',') {
            cur.eat(',')?;
        }
    }
    cur.eat(']')?;
    Ok(px::px_attrs(vec![
        (String::from("name"), px::PxVal::Str(name)),
        (String::from("tuple"), px::px_list(tuple)),
        (String::from("fields"), px::px_list(fields)),
    ]))
}

/// A match pattern: `bind(name)` / `penum(...)` / `penumstruct(...)` (v5).
fn sig_typed_pattern(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let head = cur.ident()?;
    if head == "bind" {
        cur.eat('(')?;
        let name = cur.ident()?;
        cur.eat(')')?;
        Ok(typed_kv(
            "pbind",
            vec![(String::from("name"), px::PxVal::Str(name))],
        ))
    } else if head == "penum" {
        cur.eat('(')?;
        let en = cur.ident()?;
        cur.eat(',')?;
        let variant = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut binds = Vec::new();
        while !cur.peek_is(']') {
            binds.push(sig_typed_pattern(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(')')?;
        Ok(typed_kv(
            "ppenum",
            vec![
                (String::from("enum"), px::PxVal::Str(en)),
                (String::from("variant"), px::PxVal::Str(variant)),
                (String::from("binds"), px::px_list(binds)),
            ],
        ))
    } else if head == "penumstruct" {
        cur.eat('(')?;
        let en = cur.ident()?;
        cur.eat(',')?;
        let variant = cur.ident()?;
        cur.eat(',')?;
        cur.eat('[')?;
        let mut fields = Vec::new();
        while !cur.peek_is(']') {
            let fname = cur.ident()?;
            cur.eat(':')?;
            let pat = sig_typed_pattern(cur)?;
            fields.push(px::px_attrs(vec![
                (String::from("field"), px::PxVal::Str(fname)),
                (String::from("pat"), pat),
            ]));
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat(']')?;
        cur.eat(',')?;
        let rest = cur.ident()?;
        cur.eat(')')?;
        Ok(typed_kv(
            "ppenumstruct",
            vec![
                (String::from("enum"), px::PxVal::Str(en)),
                (String::from("variant"), px::PxVal::Str(variant)),
                (String::from("fields"), px::px_list(fields)),
                (String::from("rest"), px::PxVal::Str(rest)),
            ],
        ))
    } else {
        Err(format!("held: sig pattern kind {} is outside the typed core", head))
    }
}

fn sig_typed_item(cur: &mut SigCursor) -> Result<px::PxVal, String> {
    let head = cur.ident()?;
    if head == "fn" {
        cur.eat(' ')?;
        let name = cur.ident()?;
        let generics = sig_typed_generics(cur)?;
        let params = sig_typed_params(cur)?;
        cur.eat_str("->")?;
        let ret = sig_typed_type(cur)?;
        cur.eat(' ')?;
        let body = sig_typed_block_v3(cur)?;
        Ok(typed_kv(
            "rfn",
            vec![
                (String::from("name"), px::PxVal::Str(name)),
                (String::from("generics"), px::px_list(generics)),
                (String::from("params"), px::px_list(params)),
                (String::from("ret"), ret),
                (String::from("body"), body),
            ],
        ))
    } else if head == "struct" {
        // `struct Name<G>{f:ty,...}` (v8: generics).
        cur.eat(' ')?;
        let name = cur.ident()?;
        let generics = sig_typed_generics(cur)?;
        cur.eat('{')?;
        let mut fields = Vec::new();
        while !cur.peek_is('}') {
            let fname = cur.ident()?;
            cur.eat(':')?;
            let ty = sig_typed_type(cur)?;
            fields.push(px::px_attrs(vec![
                (String::from("name"), px::PxVal::Str(fname)),
                (String::from("ty"), ty),
            ]));
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat('}')?;
        Ok(typed_kv(
            "rstruct",
            vec![
                (String::from("name"), px::PxVal::Str(name)),
                (String::from("generics"), px::px_list(generics)),
                (String::from("fields"), px::px_list(fields)),
            ],
        ))
    } else if head == "enum" {
        cur.eat(' ')?;
        let name = cur.ident()?;
        let generics = sig_typed_generics(cur)?;
        cur.eat('{')?;
        let mut variants = Vec::new();
        while !cur.peek_is('}') {
            variants.push(sig_typed_variant(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat('}')?;
        return Ok(typed_kv(
            "renum",
            vec![
                (String::from("name"), px::PxVal::Str(name)),
                (String::from("generics"), px::px_list(generics)),
                (String::from("variants"), px::px_list(variants)),
            ],
        ));
    } else if head == "impl" {
        // `impl<G> Target{...}` or `impl Target{...}` (v8: impl generics come
        // right after `impl`, before the space + target).
        let generics = sig_typed_generics(cur)?;
        cur.eat(' ')?;
        let target = sig_typed_type(cur)?;
        cur.eat('{')?;
        let mut methods = Vec::new();
        while !cur.peek_is('}') {
            methods.push(sig_typed_method(cur)?);
            if cur.peek_is(',') {
                cur.eat(',')?;
            }
        }
        cur.eat('}')?;
        Ok(typed_kv(
            "rimpl",
            vec![
                (String::from("generics"), px::px_list(generics)),
                (String::from("target"), target),
                (String::from("methods"), px::px_list(methods)),
            ],
        ))
    } else {
        Err(format!("held: sig item kind {} is outside the typed core", head))
    }
}

/// Whole-program typed parse (v3/v4): `item;item;...`.
pub fn sig_typed_program(text: &str) -> Result<px::PxVal, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut cur = SigCursor { chars: &chars, pos: 0 };
    let mut items = Vec::new();
    while cur.pos < chars.len() {
        items.push(sig_typed_item(&mut cur)?);
        cur.eat(';')?;
    }
    Ok(typed_kv("rprogram", vec![(String::from("items"), px::px_list(items))]))
}

fn typed_list(v: &px::PxVal, name: &str) -> Result<Vec<px::PxVal>, String> {
    match typed_field(v, name)? {
        px::PxVal::List(items) => Ok(items.as_ref().clone()),
        _ => Err(format!("sig typed: field {} is not a list", name)),
    }
}

/// Boolean field, defaulting to false when absent (v6 `mut`).
fn typed_bool(v: &px::PxVal, name: &str) -> bool {
    match typed_field(v, name) {
        Ok(px::PxVal::Bool(b)) => *b,
        _ => false,
    }
}

/// A match pattern back to sig form (v5).
fn sig_pattern(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    if kind == "pbind" {
        Ok(format!("bind({})", typed_field_str(v, "name")?))
    } else if kind == "ppenum" {
        let binds = typed_list(v, "binds")?
            .iter()
            .map(sig_pattern)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "penum({},{},[{}])",
            typed_field_str(v, "enum")?,
            typed_field_str(v, "variant")?,
            binds.join(",")
        ))
    } else if kind == "ppenumstruct" {
        let mut fs = Vec::new();
        for f in typed_list(v, "fields")? {
            fs.push(format!(
                "{}:{}",
                typed_field_str(&f, "field")?,
                sig_pattern(typed_field(&f, "pat")?)?
            ));
        }
        Ok(format!(
            "penumstruct({},{},[{}],{})",
            typed_field_str(v, "enum")?,
            typed_field_str(v, "variant")?,
            fs.join(","),
            typed_field_str(v, "rest")?
        ))
    } else {
        Err(format!("sig pattern render: kind {}", kind))
    }
}

/// One enum variant back to sig form: `Name(types)[fields]` (v5).
fn sig_variant(v: &px::PxVal) -> Result<String, String> {
    let tuple = typed_list(v, "tuple")?
        .iter()
        .map(type_to_sig)
        .collect::<Result<Vec<_>, _>>()?;
    let mut fs = Vec::new();
    for f in typed_list(v, "fields")? {
        fs.push(format!(
            "{}:{}",
            typed_field_str(&f, "name")?,
            type_to_sig(typed_field(&f, "ty")?)?
        ));
    }
    Ok(format!(
        "{}({})[{}]",
        typed_field_str(v, "name")?,
        tuple.join(","),
        fs.join(",")
    ))
}

/// A type node back to sig form (`i64` / `N(Point)` / `G(Name,[args])`).
fn type_to_sig(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    let name = typed_field_str(v, "name")?;
    if kind == "named" {
        Ok(format!("N({})", name))
    } else if kind == "generic" {
        let args = typed_list(v, "args")?
            .iter()
            .map(type_to_sig)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("G({},[{}])", name, args.join(",")))
    } else {
        Ok(name)
    }
}

/// A type node to Rust source (named types drop the `N(...)` wrapper;
/// generics render `Name<args>`).
fn type_to_rust(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    let name = typed_field_str(v, "name")?;
    if kind == "generic" {
        let args = typed_list(v, "args")?
            .iter()
            .map(type_to_rust)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("{}<{}>", name, args.join(", ")))
    } else {
        Ok(name)
    }
}

/// `<T,U>` sig form of a generics list field (empty when absent/empty).
fn generics_to_sig(v: &px::PxVal) -> Result<String, String> {
    let names = match typed_field(v, "generics") {
        Ok(px::PxVal::List(items)) => items.as_ref().clone(),
        _ => Vec::new(),
    };
    if names.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::new();
    for n in &names {
        match n {
            px::PxVal::Str(s) => parts.push(s.clone()),
            _ => return Err(String::from("sig generics: name is not a string")),
        }
    }
    Ok(format!("<{}>", parts.join(",")))
}

/// `<T, U>` Rust form of a generics list field (empty when absent).
fn generics_to_rust(v: &px::PxVal) -> Result<String, String> {
    let names = match typed_field(v, "generics") {
        Ok(px::PxVal::List(items)) => items.as_ref().clone(),
        _ => Vec::new(),
    };
    if names.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::new();
    for n in &names {
        match n {
            px::PxVal::Str(s) => parts.push(s.clone()),
            _ => return Err(String::from("rust generics: name is not a string")),
        }
    }
    Ok(format!("<{}>", parts.join(", ")))
}

/// `name:type,...` sig param list shared by rfn and rmethod.
fn sig_params(v: &px::PxVal) -> Result<String, String> {
    let mut out = String::new();
    let mut first = true;
    for p in typed_list(v, "params")? {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!(
            "{}:{}",
            typed_field_str(&p, "name")?,
            type_to_sig(typed_field(&p, "ty")?)?
        ));
    }
    Ok(out)
}

/// `name: type, ...` Rust param list (with the receiver prefix for methods).
fn rust_params(v: &px::PxVal) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for p in typed_list(v, "params")? {
        out.push(format!(
            "{}: {}",
            typed_field_str(&p, "name")?,
            type_to_rust(typed_field(&p, "ty")?)?
        ));
    }
    Ok(out)
}

/// A match pattern to Rust source (v5).
fn rust_pattern(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    if kind == "pbind" {
        typed_field_str(v, "name")
    } else if kind == "ppenum" {
        let binds = typed_list(v, "binds")?
            .iter()
            .map(rust_pattern)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "{}::{}({})",
            typed_field_str(v, "enum")?,
            typed_field_str(v, "variant")?,
            binds.join(", ")
        ))
    } else if kind == "ppenumstruct" {
        let mut fs = Vec::new();
        for f in typed_list(v, "fields")? {
            fs.push(format!(
                "{}: {}",
                typed_field_str(&f, "field")?,
                rust_pattern(typed_field(&f, "pat")?)?
            ));
        }
        let rest = typed_field_str(v, "rest")?;
        let tail = if rest == "norest" { "" } else { ", .." };
        Ok(format!(
            "{}::{} {{ {}{} }}",
            typed_field_str(v, "enum")?,
            typed_field_str(v, "variant")?,
            fs.join(", "),
            tail
        ))
    } else {
        Err(format!("rust pattern render: kind {}", kind))
    }
}

/// One enum variant to Rust source: tuple / struct / unit (v5).
fn rust_variant(v: &px::PxVal) -> Result<String, String> {
    let name = typed_field_str(v, "name")?;
    let tuple = typed_list(v, "tuple")?;
    let fields = typed_list(v, "fields")?;
    if !tuple.is_empty() {
        let tys = tuple
            .iter()
            .map(type_to_rust)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("{}({})", name, tys.join(", ")))
    } else if !fields.is_empty() {
        let mut fs = Vec::new();
        for f in &fields {
            fs.push(format!(
                "{}: {}",
                typed_field_str(f, "name")?,
                type_to_rust(typed_field(f, "ty")?)?
            ));
        }
        Ok(format!("{} {{ {} }}", name, fs.join(", ")))
    } else {
        Ok(name)
    }
}

/// Byte-identical sig regeneration for the v3 program core.
pub fn sig_typed_render_v3(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    if kind == "rprogram" {
        let mut out = String::new();
        for item in typed_list(v, "items")? {
            out.push_str(&sig_typed_render_v3(&item)?);
            out.push(';');
        }
        Ok(out)
    } else if kind == "rfn" {
        let mut out = format!(
            "fn {}{}(",
            typed_field_str(v, "name")?,
            generics_to_sig(v)?
        );
        out.push_str(&sig_params(v)?);
        out.push_str(&format!(")->{} ", type_to_sig(typed_field(v, "ret")?)?));
        out.push_str(&sig_typed_render_v3(typed_field(v, "body")?)?);
        Ok(out)
    } else if kind == "rstruct" {
        let mut out = format!(
            "struct {}{}{{",
            typed_field_str(v, "name")?,
            generics_to_sig(v)?
        );
        let mut first = true;
        for f in typed_list(v, "fields")? {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!(
                "{}:{}",
                typed_field_str(&f, "name")?,
                type_to_sig(typed_field(&f, "ty")?)?
            ));
        }
        out.push('}');
        Ok(out)
    } else if kind == "rimpl" {
        let mut out = format!(
            "impl{} {}{{",
            generics_to_sig(v)?,
            type_to_sig(typed_field(v, "target")?)?
        );
        let mut first = true;
        for m in typed_list(v, "methods")? {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&sig_typed_render_v3(&m)?);
        }
        out.push('}');
        Ok(out)
    } else if kind == "rmethod" {
        let mut out = format!(
            "{}{}[{}](",
            typed_field_str(v, "name")?,
            generics_to_sig(v)?,
            typed_field_str(v, "recv")?
        );
        out.push_str(&sig_params(v)?);
        out.push_str(&format!(")->{} ", type_to_sig(typed_field(v, "ret")?)?));
        out.push_str(&sig_typed_render_v3(typed_field(v, "body")?)?);
        Ok(out)
    } else if kind == "rfield" {
        Ok(format!(
            "field({},{})",
            sig_typed_render_v3(typed_field(v, "base")?)?,
            typed_field_str(v, "name")?
        ))
    } else if kind == "rslit" {
        let mut out = format!("slit({},[", typed_field_str(v, "ty")?);
        let mut first = true;
        for f in typed_list(v, "fields")? {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!(
                "{}:{}",
                typed_field_str(&f, "name")?,
                sig_typed_render_v3(typed_field(&f, "value")?)?
            ));
        }
        out.push_str("])");
        Ok(out)
    } else if kind == "rpcall" {
        let args = typed_list(v, "args")?
            .iter()
            .map(sig_typed_render_v3)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "pcall({},{},[{}])",
            typed_field_str(v, "ty")?,
            typed_field_str(v, "method")?,
            args.join(",")
        ))
    } else if kind == "rmcall" {
        let args = typed_list(v, "args")?
            .iter()
            .map(sig_typed_render_v3)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "mcall({},{},[],[{}])",
            sig_typed_render_v3(typed_field(v, "recv")?)?,
            typed_field_str(v, "method")?,
            args.join(",")
        ))
    } else if kind == "renum" {
        let mut vs = Vec::new();
        for var in typed_list(v, "variants")? {
            vs.push(sig_variant(&var)?);
        }
        Ok(format!(
            "enum {}{}{{{}}}",
            typed_field_str(v, "name")?,
            generics_to_sig(v)?,
            vs.join(",")
        ))
    } else if kind == "rmatch" {
        let mut arms = Vec::new();
        for a in typed_list(v, "arms")? {
            arms.push(format!(
                "{}=>{}",
                sig_pattern(typed_field(&a, "pat")?)?,
                sig_typed_render_v3(typed_field(&a, "body")?)?
            ));
        }
        Ok(format!(
            "match({},[{}])",
            sig_typed_render_v3(typed_field(v, "scrut")?)?,
            arms.join(",")
        ))
    } else if kind == "reslit" {
        let mut fs = Vec::new();
        for f in typed_list(v, "fields")? {
            fs.push(format!(
                "{}:{}",
                typed_field_str(&f, "name")?,
                sig_typed_render_v3(typed_field(&f, "value")?)?
            ));
        }
        Ok(format!(
            "eslit({},{},[{}])",
            typed_field_str(v, "enum")?,
            typed_field_str(v, "variant")?,
            fs.join(",")
        ))
    } else if kind == "rblock" {
        let mut out = String::from("{");
        let stmts = typed_list(v, "stmts")?;
        let mut first = true;
        for s in &stmts {
            if !first {
                out.push(';');
            }
            first = false;
            out.push_str(&sig_typed_render_v3(s)?);
        }
        out.push('|');
        let tail = typed_field(v, "tail")?;
        if typed_field_str(tail, "kind")? == "rnotail" {
            out.push('_');
        } else {
            out.push_str(&sig_typed_render_v3(tail)?);
        }
        out.push('}');
        Ok(out)
    } else if kind == "rex" {
        Ok(format!("ex {}", sig_typed_render_v3(typed_field(v, "expr")?)?))
    } else if kind == "rlet" {
        let kw = if typed_bool(v, "mut") { "let mut " } else { "let " };
        Ok(format!(
            "{}{}:_={}",
            kw,
            typed_field_str(v, "name")?,
            sig_typed_render_v3(typed_field(v, "expr")?)?
        ))
    } else if kind == "rassign" {
        Ok(format!(
            "assign({},{})",
            sig_typed_render_v3(typed_field(v, "place")?)?,
            sig_typed_render_v3(typed_field(v, "value")?)?
        ))
    } else if kind == "rwhile" {
        Ok(format!(
            "while({},{})",
            sig_typed_render_v3(typed_field(v, "cond")?)?,
            sig_typed_render_v3(typed_field(v, "body")?)?
        ))
    } else if kind == "rforeach" {
        Ok(format!(
            "foreach({},{},{})",
            sig_pattern(typed_field(v, "pat")?)?,
            sig_typed_render_v3(typed_field(v, "iter")?)?,
            sig_typed_render_v3(typed_field(v, "body")?)?
        ))
    } else if kind == "runary" {
        Ok(format!(
            "un({},{})",
            typed_field_str(v, "op")?,
            sig_typed_render_v3(typed_field(v, "expr")?)?
        ))
    } else if kind == "rref" {
        Ok(format!(
            "ref({},{})",
            typed_field_str(v, "mode")?,
            sig_typed_render_v3(typed_field(v, "expr")?)?
        ))
    } else if kind == "rcall" {
        let args = typed_list(v, "args")?
            .iter()
            .map(sig_typed_render_v3)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "call({},[{}])",
            typed_field_str(v, "name")?,
            args.join(",")
        ))
    } else if kind == "rprintln" {
        let args = typed_list(v, "args")?
            .iter()
            .map(sig_typed_render_v3)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "println(\"{}\",[{}])",
            typed_field_str(v, "fmt")?,
            args.join(",")
        ))
    } else if kind == "rif" {
        // v3 `if` carries BLOCK branches; v6 allows a `_` else (no-else).
        let else_e = typed_field(v, "else_e")?;
        let else_sig = if typed_field_str(else_e, "kind")? == "rnoelse" {
            String::from("_")
        } else {
            sig_typed_render_v3(else_e)?
        };
        Ok(format!(
            "if({},{},{})",
            sig_typed_render_v3(typed_field(v, "cond")?)?,
            sig_typed_render_v3(typed_field(v, "then_e")?)?,
            else_sig
        ))
    } else if kind == "rbin" {
        Ok(format!(
            "bin({},{},{})",
            typed_field_str(v, "op")?,
            sig_typed_render_v3(typed_field(v, "lhs")?)?,
            sig_typed_render_v3(typed_field(v, "rhs")?)?
        ))
    } else {
        // v2 leaf kinds (rint/rvar).
        sig_typed_render(v)
    }
}

/// px typed tree -> RUST SOURCE (the reverse joint). Parenthesizes operands
/// conservatively — acceptance is AST identity via ast-canonical, not text.
pub fn rust_render(v: &px::PxVal) -> Result<String, String> {
    let kind = typed_field_str(v, "kind")?;
    if kind == "rprogram" {
        let items = typed_list(v, "items")?
            .iter()
            .map(rust_render)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items.join(" "))
    } else if kind == "rfn" {
        let params = rust_params(v)?;
        let ret = type_to_rust(typed_field(v, "ret")?)?;
        let arrow = if ret == "unit" {
            String::new()
        } else {
            format!(" -> {}", ret)
        };
        Ok(format!(
            "fn {}{}({}){} {}",
            typed_field_str(v, "name")?,
            generics_to_rust(v)?,
            params.join(", "),
            arrow,
            rust_render(typed_field(v, "body")?)?
        ))
    } else if kind == "rstruct" {
        let mut fields = Vec::new();
        for f in typed_list(v, "fields")? {
            fields.push(format!(
                "{}: {}",
                typed_field_str(&f, "name")?,
                type_to_rust(typed_field(&f, "ty")?)?
            ));
        }
        Ok(format!(
            "struct {}{} {{ {} }}",
            typed_field_str(v, "name")?,
            generics_to_rust(v)?,
            fields.join(", ")
        ))
    } else if kind == "rimpl" {
        let mut methods = Vec::new();
        for m in typed_list(v, "methods")? {
            methods.push(rust_render(&m)?);
        }
        Ok(format!(
            "impl{} {} {{ {} }}",
            generics_to_rust(v)?,
            type_to_rust(typed_field(v, "target")?)?,
            methods.join(" ")
        ))
    } else if kind == "rmethod" {
        // Receiver is the first parameter in Rust source; assoc has none.
        let recv = typed_field_str(v, "recv")?;
        let mut params = Vec::new();
        if recv != "assoc" {
            params.push(recv);
        }
        for p in rust_params(v)? {
            params.push(p);
        }
        let ret = type_to_rust(typed_field(v, "ret")?)?;
        let arrow = if ret == "unit" {
            String::new()
        } else {
            format!(" -> {}", ret)
        };
        Ok(format!(
            "fn {}{}({}){} {}",
            typed_field_str(v, "name")?,
            generics_to_rust(v)?,
            params.join(", "),
            arrow,
            rust_render(typed_field(v, "body")?)?
        ))
    } else if kind == "rfield" {
        Ok(format!(
            "{}.{}",
            rust_render(typed_field(v, "base")?)?,
            typed_field_str(v, "name")?
        ))
    } else if kind == "rslit" {
        let mut fields = Vec::new();
        for f in typed_list(v, "fields")? {
            fields.push(format!(
                "{}: {}",
                typed_field_str(&f, "name")?,
                rust_render(typed_field(&f, "value")?)?
            ));
        }
        Ok(format!(
            "{} {{ {} }}",
            typed_field_str(v, "ty")?,
            fields.join(", ")
        ))
    } else if kind == "rpcall" {
        let args = typed_list(v, "args")?
            .iter()
            .map(rust_render)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "{}::{}({})",
            typed_field_str(v, "ty")?,
            typed_field_str(v, "method")?,
            args.join(", ")
        ))
    } else if kind == "rmcall" {
        let args = typed_list(v, "args")?
            .iter()
            .map(rust_render)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "{}.{}({})",
            rust_render(typed_field(v, "recv")?)?,
            typed_field_str(v, "method")?,
            args.join(", ")
        ))
    } else if kind == "renum" {
        let mut vs = Vec::new();
        for var in typed_list(v, "variants")? {
            vs.push(rust_variant(&var)?);
        }
        Ok(format!(
            "enum {}{} {{ {} }}",
            typed_field_str(v, "name")?,
            generics_to_rust(v)?,
            vs.join(", ")
        ))
    } else if kind == "rmatch" {
        let mut arms = Vec::new();
        for a in typed_list(v, "arms")? {
            arms.push(format!(
                "{} => {}",
                rust_pattern(typed_field(&a, "pat")?)?,
                rust_render(typed_field(&a, "body")?)?
            ));
        }
        Ok(format!(
            "match {} {{ {} }}",
            rust_render(typed_field(v, "scrut")?)?,
            arms.join(", ")
        ))
    } else if kind == "reslit" {
        let mut fs = Vec::new();
        for f in typed_list(v, "fields")? {
            fs.push(format!(
                "{}: {}",
                typed_field_str(&f, "name")?,
                rust_render(typed_field(&f, "value")?)?
            ));
        }
        Ok(format!(
            "{}::{} {{ {} }}",
            typed_field_str(v, "enum")?,
            typed_field_str(v, "variant")?,
            fs.join(", ")
        ))
    } else if kind == "rblock" {
        let mut out = String::from("{ ");
        for s in typed_list(v, "stmts")? {
            out.push_str(&rust_render(&s)?);
            out.push(' ');
        }
        let tail = typed_field(v, "tail")?;
        if typed_field_str(tail, "kind")? != "rnotail" {
            out.push_str(&rust_render(tail)?);
            out.push(' ');
        }
        out.push('}');
        Ok(out)
    } else if kind == "rex" {
        Ok(format!("{};", rust_render(typed_field(v, "expr")?)?))
    } else if kind == "rlet" {
        let kw = if typed_bool(v, "mut") { "let mut " } else { "let " };
        Ok(format!(
            "{}{} = {};",
            kw,
            typed_field_str(v, "name")?,
            rust_render(typed_field(v, "expr")?)?
        ))
    } else if kind == "rassign" {
        // As a statement the rex wrapper appends `;`.
        Ok(format!(
            "{} = {}",
            rust_render(typed_field(v, "place")?)?,
            rust_render(typed_field(v, "value")?)?
        ))
    } else if kind == "rwhile" {
        Ok(format!(
            "while {} {}",
            rust_render(typed_field(v, "cond")?)?,
            rust_render(typed_field(v, "body")?)?
        ))
    } else if kind == "rforeach" {
        Ok(format!(
            "for {} in {} {}",
            rust_pattern(typed_field(v, "pat")?)?,
            rust_render(typed_field(v, "iter")?)?,
            rust_render(typed_field(v, "body")?)?
        ))
    } else if kind == "runary" {
        let op = typed_field_str(v, "op")?;
        let sym = if op == "deref" {
            "*"
        } else if op == "neg" {
            "-"
        } else if op == "not" {
            "!"
        } else {
            return Err(format!("rust render: unary op {}", op));
        };
        Ok(format!("{}{}", sym, rust_render(typed_field(v, "expr")?)?))
    } else if kind == "rref" {
        let mode = typed_field_str(v, "mode")?;
        let prefix = if mode == "mut" { "&mut " } else { "&" };
        Ok(format!("{}{}", prefix, rust_render(typed_field(v, "expr")?)?))
    } else if kind == "rcall" {
        let args = typed_list(v, "args")?
            .iter()
            .map(rust_render)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(
            "{}({})",
            typed_field_str(v, "name")?,
            args.join(", ")
        ))
    } else if kind == "rprintln" {
        let mut out = format!("println!(\"{}\"", typed_field_str(v, "fmt")?);
        for a in typed_list(v, "args")? {
            out.push_str(", ");
            out.push_str(&rust_render(&a)?);
        }
        out.push(')');
        Ok(out)
    } else if kind == "rif" {
        let else_e = typed_field(v, "else_e")?;
        if typed_field_str(else_e, "kind")? == "rnoelse" {
            Ok(format!(
                "if {} {}",
                rust_render(typed_field(v, "cond")?)?,
                rust_render(typed_field(v, "then_e")?)?
            ))
        } else {
            Ok(format!(
                "if {} {} else {}",
                rust_render(typed_field(v, "cond")?)?,
                rust_render(typed_field(v, "then_e")?)?,
                rust_render(else_e)?
            ))
        }
    } else if kind == "rint" {
        match typed_field(v, "value")? {
            px::PxVal::Int(n) => Ok(format!("{}", n)),
            _ => Err(String::from("rust render: rint value")),
        }
    } else if kind == "rvar" {
        typed_field_str(v, "name")
    } else if kind == "rbin" {
        let op = typed_field_str(v, "op")?;
        let sym = match op.as_str() {
            "add" => "+",
            "sub" => "-",
            "mul" => "*",
            "div" => "/",
            "rem" => "%",
            "eq" => "==",
            "ne" => "!=",
            "lt" => "<",
            "le" => "<=",
            "gt" => ">",
            "ge" => ">=",
            other => return Err(format!("rust render: op {}", other)),
        };
        Ok(format!(
            "({} {} {})",
            rust_render(typed_field(v, "lhs")?)?,
            sym,
            rust_render(typed_field(v, "rhs")?)?
        ))
    } else {
        Err(format!("held: rust render kind {}", kind))
    }
}

pub struct RustReconRecord {
    pub sig_roundtrip: bool,
    pub ast_identity: bool,
    pub rustc_parity: bool,
    /// Floor-certified well-typedness (rs-meta typeck accepts the residual).
    pub well_typed: bool,
    pub witness: gate::Witness,
}

/// v3 joint: Rust source -> ast-canonical sig -> typed px PROGRAM tree ->
/// (a) byte-identical sig regeneration, (b) RUST reconstruction whose
/// ast-canonical equals the original's, (c) rustc output parity.
pub fn rust_program_reconstruct(
    rust_source: &str,
    bootstrap: &str,
    granted: &[String],
) -> Result<RustReconRecord, String> {
    let sig_raw =
        interop::host_run_bootstrap_inline(bootstrap, "ast-canonical", rust_source, granted)?;
    let sig = sig_raw.trim();
    let typed = sig_typed_program(sig)?;
    let sig_roundtrip = sig_typed_render_v3(&typed)? == sig;

    let rendered = rust_render(&typed)?;
    let sig2 =
        interop::host_run_bootstrap_inline(bootstrap, "ast-canonical", &rendered, granted)?;
    let ast_identity = sig2.trim() == sig;

    let out1 =
        interop::host_run_bootstrap_inline(bootstrap, "native-run", rust_source, granted)?;
    let out2 =
        interop::host_run_bootstrap_inline(bootstrap, "native-run", &rendered, granted)?;
    let rustc_parity = out1 == out2;

    // Well-typed BY CONSTRUCTION, certified by the trusted floor: rs-meta's own
    // typeck accepts the reconstructed residual (proposal 0005). typeck-check
    // proves the floor accepts iff rustc does, so this is a well-typedness
    // guarantee from the meta-circular floor — a static edge a dynamic Lisp
    // meta-circular cannot cheaply provide.
    let well_typed =
        interop::host_run_bootstrap_inline(bootstrap, "typecheck", &rendered, granted).is_ok();

    let ok = sig_roundtrip && ast_identity && rustc_parity && well_typed;
    let mut sorted: Vec<String> = granted.to_vec();
    sorted.sort();
    let witness = gate::Witness {
        direction: String::from("rust-reconstruction"),
        source_lang: String::from("px"),
        target_lang: String::from("rust"),
        input_kind: String::from("typed-program-tree"),
        output_kind: String::from("rust-source"),
        loss_status: String::from(if ok { "lossless" } else { "rejected" }),
        effect_class: String::from("host-call"),
        capability_required: String::from("host-call"),
        in_hash: sha256_hex(sig.as_bytes()),
        out_hash: sha256_hex(rendered.as_bytes()),
        env_hash: sha256_hex(format!("granted={}", sorted.join(",")).as_bytes()),
        status: String::from(if ok { "ok" } else { "rejected" }),
        loss: String::from("none"),
    };
    Ok(RustReconRecord {
        sig_roundtrip,
        ast_identity,
        rustc_parity,
        well_typed,
        witness,
    })
}

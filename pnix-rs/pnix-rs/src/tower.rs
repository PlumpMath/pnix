//! Tower milestone-1 for the pnix-rs lane (P11).
//!
//! Literature-grounded shape (Amin & Rompf POPL'18 stage polymorphism;
//! Jones/Gomard/Sestoft S = L prerequisite; 3-Lisp finite reify/reflect):
//! this milestone ships
//!
//!   - reify:   px AST -> px attrset encoding (AST-as-px-data),
//!   - reflect: encoding -> px AST (the inverse),
//!   - a self-interpreter WRITTEN IN px (`runtime/tower/self_interp.px`) that
//!     evaluates reified programs — the S = L prerequisite demonstrated: px
//!     evaluates its own encoding through the one sacred runtime.
//!
//! HONEST SCOPE (through milestone-3b): the encoding covers int/bool/var/
//! lambda/apply/if/binary(incl. ++ //)/let(RECURSIVE, m2)/str(+interpolation)/
//! list/attrs/select. Guest closures cannot yet be passed INTO host builtins
//! (higher-order builtins in encoded programs, e.g. encoded `map (x: ...)`)
//! — that wrapper is milestone-4; a real cogen and full-language S = L are
//! NOT claimed.

use crate::interop;
use crate::px;
use crate::sha256::sha256_hex;

pub const TOWER_SCHEMA: &str = "pnix-rs.tower.v0";

fn attr(kind: &str, fields: Vec<(String, px::PxVal)>) -> px::PxVal {
    let mut all = vec![(
        String::from("kind"),
        px::PxVal::Str(String::from(kind)),
    )];
    for f in fields {
        all.push(f);
    }
    px::px_attrs(all)
}

fn op_symbol(op: &px::PxOp) -> Result<&'static str, String> {
    match op {
        px::PxOp::Add => Ok("+"),
        px::PxOp::Sub => Ok("-"),
        px::PxOp::Mul => Ok("*"),
        px::PxOp::Div => Ok("/"),
        px::PxOp::Eq => Ok("=="),
        px::PxOp::Ne => Ok("!="),
        px::PxOp::Lt => Ok("<"),
        px::PxOp::Le => Ok("<="),
        px::PxOp::Gt => Ok(">"),
        px::PxOp::Ge => Ok(">="),
        px::PxOp::Concat => Ok("++"),
        px::PxOp::Update => Ok("//"),
        px::PxOp::HasAttr => Ok("?"),
    }
}

fn symbol_op(sym: &str) -> Result<px::PxOp, String> {
    match sym {
        "+" => Ok(px::PxOp::Add),
        "-" => Ok(px::PxOp::Sub),
        "*" => Ok(px::PxOp::Mul),
        "/" => Ok(px::PxOp::Div),
        "==" => Ok(px::PxOp::Eq),
        "!=" => Ok(px::PxOp::Ne),
        "<" => Ok(px::PxOp::Lt),
        "<=" => Ok(px::PxOp::Le),
        ">" => Ok(px::PxOp::Gt),
        ">=" => Ok(px::PxOp::Ge),
        "++" => Ok(px::PxOp::Concat),
        "//" => Ok(px::PxOp::Update),
        "?" => Ok(px::PxOp::HasAttr),
        other => Err(format!("reflect: unknown op {}", other)),
    }
}

/// px AST -> px attrset encoding (AST-as-px-data). Every lambda carries a
/// deterministic pre-order integer label `lid` (pnix-hy M8): the polyvariant
/// specializer keys specialization points by O(1) label instead of an
/// O(size) structural body comparison. All other consumers ignore it.
pub fn reify(e: &px::PxExpr) -> Result<px::PxVal, String> {
    let mut ctr = 0i64;
    reify_with(e, &mut ctr)
}

fn reify_with(e: &px::PxExpr, ctr: &mut i64) -> Result<px::PxVal, String> {
    match e {
        px::PxExpr::DeferredError(_) => {
            Err(String::from("held: deferred import error is internal-only"))
        }
        px::PxExpr::Int(n) => Ok(attr("int", vec![(String::from("value"), px::PxVal::Int(*n))])),
        px::PxExpr::Float(_) => Err(String::from("held: float encoding is future work")),
        px::PxExpr::Bool(b) => Ok(attr(
            "bool",
            vec![(String::from("value"), px::PxVal::Bool(*b))],
        )),
        px::PxExpr::Null => Ok(attr("null", vec![])),
        px::PxExpr::With { .. } => Err(String::from("held: with encoding is future work")),
        px::PxExpr::Isolated { .. } => {
            Err(String::from("held: isolated encoding is future work"))
        }
        px::PxExpr::Var(name) => Ok(attr(
            "var",
            vec![(String::from("name"), px::PxVal::Str(name.clone()))],
        )),
        px::PxExpr::Lambda { param, body } => {
            let lid = *ctr;
            *ctr += 1;
            Ok(attr(
                "lambda",
                vec![
                    (String::from("param"), px::PxVal::Str(param.clone())),
                    (String::from("lid"), px::PxVal::Int(lid)),
                    (String::from("body"), reify_with(body, ctr)?),
                ],
            ))
        }
        px::PxExpr::Apply { func, arg } => Ok(attr(
            "apply",
            vec![
                (String::from("func"), reify_with(func, ctr)?),
                (String::from("arg"), reify_with(arg, ctr)?),
            ],
        )),
        px::PxExpr::If { cond, then_e, else_e } => Ok(attr(
            "if",
            vec![
                (String::from("cond"), reify_with(cond, ctr)?),
                (String::from("then_e"), reify_with(then_e, ctr)?),
                (String::from("else_e"), reify_with(else_e, ctr)?),
            ],
        )),
        px::PxExpr::Binary { op, lhs, rhs } => Ok(attr(
            "binary",
            vec![
                (String::from("op"), px::PxVal::Str(String::from(op_symbol(op)?))),
                (String::from("lhs"), reify_with(lhs, ctr)?),
                (String::from("rhs"), reify_with(rhs, ctr)?),
            ],
        )),
        px::PxExpr::LetIn { bindings, body } => {
            let mut encoded = Vec::new();
            for (name, value) in bindings {
                encoded.push(px::px_attrs(vec![
                    (String::from("name"), px::PxVal::Str(name.clone())),
                    (String::from("value"), reify_with(value, ctr)?),
                ]));
            }
            Ok(attr(
                "let",
                vec![
                    (String::from("bindings"), px::px_list(encoded)),
                    (String::from("body"), reify_with(body, ctr)?),
                ],
            ))
        }
        px::PxExpr::Str(parts) => {
            let mut encoded = Vec::new();
            for part in parts {
                match part {
                    px::PxStrPart::Lit(s) => encoded.push(px::px_attrs(vec![
                        (String::from("kind"), px::PxVal::Str(String::from("lit"))),
                        (String::from("text"), px::PxVal::Str(s.clone())),
                    ])),
                    px::PxStrPart::Sub(sub) => encoded.push(px::px_attrs(vec![
                        (String::from("kind"), px::PxVal::Str(String::from("sub"))),
                        (String::from("node"), reify_with(sub, ctr)?),
                    ])),
                }
            }
            Ok(attr("str", vec![(String::from("parts"), px::px_list(encoded))]))
        }
        px::PxExpr::List(items) => {
            let mut encoded = Vec::new();
            for item in items {
                encoded.push(reify_with(item, ctr)?);
            }
            Ok(attr("list", vec![(String::from("items"), px::px_list(encoded))]))
        }
        px::PxExpr::Attrs(fields) => {
            let mut encoded = Vec::new();
            for (name, value) in fields {
                encoded.push(px::px_attrs(vec![
                    (String::from("name"), px::PxVal::Str(name.clone())),
                    (String::from("value"), reify_with(value, ctr)?),
                ]));
            }
            Ok(attr("attrs", vec![(String::from("fields"), px::px_list(encoded))]))
        }
        px::PxExpr::Select { base, name } => Ok(attr(
            "select",
            vec![
                (String::from("base"), reify_with(base, ctr)?),
                (String::from("name"), px::PxVal::Str(name.clone())),
            ],
        )),
    }
}

fn field<'a>(fields: &'a [(String, px::PxVal)], name: &str) -> Result<&'a px::PxVal, String> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
        .ok_or_else(|| format!("reflect: missing field {}", name))
}

fn field_str(fields: &[(String, px::PxVal)], name: &str) -> Result<String, String> {
    match field(fields, name)? {
        px::PxVal::Str(s) => Ok(s.clone()),
        _ => Err(format!("reflect: field {} is not a string", name)),
    }
}

/// Encoding -> px AST (the inverse of reify).
pub fn reflect(v: &px::PxVal) -> Result<px::PxExpr, String> {
    // deep-force at the boundary: reflect's traversal predates laziness and
    // reads field values directly, so hand it a thunk-free value.
    let fv = px::px_force_deep(v)?;
    let fields = match &fv {
        px::PxVal::Attrs(fields) => fields.as_ref(),
        _ => return Err(String::from("reflect: node is not an attrset")),
    };
    let kind = field_str(fields, "kind")?;
    if kind == "int" {
        match field(fields, "value")? {
            px::PxVal::Int(n) => Ok(px::PxExpr::Int(*n)),
            _ => Err(String::from("reflect: int value")),
        }
    } else if kind == "bool" {
        match field(fields, "value")? {
            px::PxVal::Bool(b) => Ok(px::PxExpr::Bool(*b)),
            _ => Err(String::from("reflect: bool value")),
        }
    } else if kind == "null" {
        Ok(px::PxExpr::Null)
    } else if kind == "var" {
        Ok(px::PxExpr::Var(field_str(fields, "name")?))
    } else if kind == "lambda" {
        Ok(px::PxExpr::Lambda {
            param: field_str(fields, "param")?,
            body: std::rc::Rc::new(reflect(field(fields, "body")?)?),
        })
    } else if kind == "apply" {
        Ok(px::PxExpr::Apply {
            func: Box::new(reflect(field(fields, "func")?)?),
            arg: Box::new(reflect(field(fields, "arg")?)?),
        })
    } else if kind == "if" {
        Ok(px::PxExpr::If {
            cond: Box::new(reflect(field(fields, "cond")?)?),
            then_e: Box::new(reflect(field(fields, "then_e")?)?),
            else_e: Box::new(reflect(field(fields, "else_e")?)?),
        })
    } else if kind == "binary" {
        Ok(px::PxExpr::Binary {
            op: symbol_op(&field_str(fields, "op")?)?,
            lhs: Box::new(reflect(field(fields, "lhs")?)?),
            rhs: Box::new(reflect(field(fields, "rhs")?)?),
        })
    } else if kind == "str" {
        let encoded = match field(fields, "parts")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: str parts")),
        };
        let mut parts = Vec::new();
        for item in encoded.iter() {
            match item {
                px::PxVal::Attrs(pf) => {
                    let pkind = field_str(pf.as_ref(), "kind")?;
                    if pkind == "lit" {
                        parts.push(px::PxStrPart::Lit(field_str(pf.as_ref(), "text")?));
                    } else if pkind == "sub" {
                        parts.push(px::PxStrPart::Sub(reflect(field(pf.as_ref(), "node")?)?));
                    } else {
                        return Err(format!("reflect: str part kind {}", pkind));
                    }
                }
                _ => return Err(String::from("reflect: str part shape")),
            }
        }
        Ok(px::PxExpr::Str(parts))
    } else if kind == "list" {
        let encoded = match field(fields, "items")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: list items")),
        };
        let mut items = Vec::new();
        for item in encoded.iter() {
            items.push(reflect(item)?);
        }
        Ok(px::PxExpr::List(items))
    } else if kind == "attrs" {
        let encoded = match field(fields, "fields")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: attrs fields")),
        };
        let mut out = Vec::new();
        for item in encoded.iter() {
            match item {
                px::PxVal::Attrs(ff) => out.push((
                    field_str(ff.as_ref(), "name")?,
                    reflect(field(ff.as_ref(), "value")?)?,
                )),
                _ => return Err(String::from("reflect: attrs field shape")),
            }
        }
        Ok(px::PxExpr::Attrs(out))
    } else if kind == "select" {
        Ok(px::PxExpr::Select {
            base: Box::new(reflect(field(fields, "base")?)?),
            name: field_str(fields, "name")?,
        })
    } else if kind == "clist" {
        // m6b const list: items are data nodes.
        let encoded = match field(fields, "items")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: clist items")),
        };
        let mut items = Vec::new();
        for item in encoded.iter() {
            items.push(reflect(item)?);
        }
        Ok(px::PxExpr::List(items))
    } else if kind == "cattrs" {
        // m6b const attrset: fields are { name; value = <data node>; }.
        let encoded = match field(fields, "fields")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: cattrs fields")),
        };
        let mut out = Vec::new();
        for item in encoded.iter() {
            match item {
                px::PxVal::Attrs(ff) => out.push((
                    field_str(ff.as_ref(), "name")?,
                    reflect(field(ff.as_ref(), "value")?)?,
                )),
                _ => return Err(String::from("reflect: cattrs field shape")),
            }
        }
        Ok(px::PxExpr::Attrs(out))
    } else if kind == "gbuiltins" {
        Ok(px::PxExpr::Var(String::from("builtins")))
    } else if kind == "bapp" || kind == "bfn" {
        // Residual builtin application: curried apply chain over
        // builtins.<name> (bfn may be partial — fewer args than arity).
        let name = field_str(fields, "name")?;
        let encoded = match field(fields, "args")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: bapp args")),
        };
        let mut expr = px::PxExpr::Select {
            base: Box::new(px::PxExpr::Var(String::from("builtins"))),
            name,
        };
        for arg in encoded.iter() {
            expr = px::PxExpr::Apply {
                func: Box::new(expr),
                arg: Box::new(reflect(arg)?),
            };
        }
        Ok(expr)
    } else if kind == "let" {
        let encoded = match field(fields, "bindings")? {
            px::PxVal::List(items) => items.clone(),
            _ => return Err(String::from("reflect: let bindings")),
        };
        let mut bindings = Vec::new();
        for item in encoded.iter() {
            match item {
                px::PxVal::Attrs(bf) => bindings.push((
                    field_str(bf.as_ref(), "name")?,
                    reflect(field(bf.as_ref(), "value")?)?,
                )),
                _ => return Err(String::from("reflect: binding shape")),
            }
        }
        Ok(px::PxExpr::LetIn {
            bindings,
            body: Box::new(reflect(field(fields, "body")?)?),
        })
    } else {
        Err(format!("reflect: unknown kind {}", kind))
    }
}

/// Run one reified program through the px-written self-interpreter.
/// The encoding embeds via its canonical print (data values print as valid px).
pub fn self_interp_eval(probe_source: &str, granted: &[String]) -> Result<String, String> {
    let ast = px::px_parse(probe_source)?;
    let encoded = reify(&ast)?;
    self_interp_eval_encoded(&px::px_print(&encoded), granted)
}

/// Evaluate an ALREADY-ENCODED node (given as its canonical print) through
/// the px self-interpreter. The P6 v2 join feeds bridge-translated Rust AST
/// nodes in here.
pub fn self_interp_eval_encoded(
    encoded_text: &str,
    granted: &[String],
) -> Result<String, String> {
    let interp_src = interop::host_read_file("runtime/tower/self_interp.px", granted)?;
    let program = format!("({}) [ ] {}", interp_src.trim(), encoded_text);
    px::px_run(&program)
}

/// Deterministic content hash of a probe's encoding.
pub fn encoding_sha256(probe_source: &str) -> Result<String, String> {
    let ast = px::px_parse(probe_source)?;
    let encoded = reify(&ast)?;
    Ok(sha256_hex(px::px_print(&encoded).as_bytes()))
}

// ---- m5: the specializer expressed IN px + cogen acceptance ---------------------

pub struct MixOutcome {
    /// Canonical print of the residual node (always present).
    pub residual_node: String,
    /// Folded ground value when the residual is a data node.
    pub folded_value: Option<String>,
    /// Residual reflected back to px source when it is pure syntax
    /// (no spec-time closure/unsupported nodes survive).
    pub residual_source: Option<String>,
}

/// Run the px-EXPRESSED specializer on a core-subset program: px specializing
/// px. `statics` are integer bindings injected as already-folded nodes.
pub fn mix_in_px(
    px_source: &str,
    statics: &[(String, i64)],
    granted: &[String],
) -> Result<MixOutcome, String> {
    let ast = px::px_parse(px_source)?;
    let encoded = reify(&ast)?;
    let encoded_text = px::px_print(&encoded);
    let mut senv = String::from("{ ");
    for (name, value) in statics {
        senv.push_str(&format!(
            "{} = {{ kind = \"int\"; value = {}; }}; ",
            name, value
        ));
    }
    senv.push('}');
    let mix_src = interop::host_read_file("runtime/tower/mix.px", granted)?;
    let call = format!("({}) {} {}", mix_src.trim(), encoded_text, senv);
    let call_ast = px::px_parse(&call)?;
    let env = Vec::new();
    // deep-force: the residual is a finite reified program node whose fields
    // the extraction below reads directly (kind/value); hand it thunk-free.
    let residual = px::px_force_deep(&px::px_eval(&call_ast, &env)?)?;
    let residual_node = px::px_print(&residual);

    let mut folded_value = None;
    if let px::PxVal::Attrs(fields) = &residual {
        let kind = fields
            .iter()
            .find(|(k, _)| k == "kind")
            .and_then(|(_, v)| match v {
                px::PxVal::Str(s) => Some(s.clone()),
                _ => None,
            });
        if kind.as_deref() == Some("int") || kind.as_deref() == Some("bool") {
            for (k, v) in fields.iter() {
                if k == "value" {
                    folded_value = Some(px::px_print(v));
                }
            }
        }
    }
    let residual_source = match reflect(&residual) {
        Ok(expr) => Some(px::px_emit(&expr)),
        Err(_) => None,
    };
    Ok(MixOutcome {
        residual_node,
        folded_value,
        residual_source,
    })
}

/// A px data VALUE as a mix static node (m6a): ints/bools/strings become the
/// matching data nodes, attrsets become const-attrs (`cattrs`) with data-node
/// fields. Opaque leaves and lists are not embeddable (held).
pub fn value_to_mix_node(v: &px::PxVal) -> Result<px::PxVal, String> {
    let v = px::px_force_deep(v)?;
    match &v {
        px::PxVal::Int(n) => Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("int"))),
            (String::from("value"), px::PxVal::Int(*n)),
        ])),
        px::PxVal::Bool(b) => Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("bool"))),
            (String::from("value"), px::PxVal::Bool(*b)),
        ])),
        px::PxVal::Str(s) => Ok(px::px_attrs(vec![
            (String::from("kind"), px::PxVal::Str(String::from("str"))),
            (
                String::from("parts"),
                px::px_list(vec![px::px_attrs(vec![
                    (String::from("kind"), px::PxVal::Str(String::from("lit"))),
                    (String::from("text"), px::PxVal::Str(s.clone())),
                ])]),
            ),
        ])),
        px::PxVal::Attrs(fields) => {
            let mut encoded = Vec::new();
            for (name, value) in fields.iter() {
                encoded.push(px::px_attrs(vec![
                    (String::from("name"), px::PxVal::Str(name.clone())),
                    (String::from("value"), value_to_mix_node(value)?),
                ]));
            }
            Ok(px::px_attrs(vec![
                (String::from("kind"), px::PxVal::Str(String::from("cattrs"))),
                (String::from("fields"), px::px_list(encoded)),
            ]))
        }
        px::PxVal::List(items) => {
            let mut encoded = Vec::new();
            for item in items.iter() {
                encoded.push(value_to_mix_node(item)?);
            }
            Ok(px::px_attrs(vec![
                (String::from("kind"), px::PxVal::Str(String::from("clist"))),
                (String::from("items"), px::px_list(encoded)),
            ]))
        }
        other => Err(format!("held: {} is not a mix static node", px::px_kind(other))),
    }
}

/// mix_in_px with arbitrary px DATA statics (m6a — the first-projection
/// harness binds the object-language program as a const-attrs node).
pub fn mix_in_px_data(
    px_source: &str,
    statics: &[(String, px::PxVal)],
    granted: &[String],
) -> Result<MixOutcome, String> {
    let ast = px::px_parse(px_source)?;
    let encoded = reify(&ast)?;
    let encoded_text = px::px_print(&encoded);
    let mut senv = String::from("{ ");
    for (name, value) in statics {
        senv.push_str(&format!(
            "{} = {}; ",
            name,
            px::px_print(&value_to_mix_node(value)?)
        ));
    }
    senv.push('}');
    let mix_src = interop::host_read_file("runtime/tower/mix.px", granted)?;
    let call = format!("({}) {} {}", mix_src.trim(), encoded_text, senv);
    let call_ast = px::px_parse(&call)?;
    let env = Vec::new();
    let residual = px::px_force_deep(&px::px_eval(&call_ast, &env)?)?;
    let residual_node = px::px_print(&residual);
    let mut folded_value = None;
    if let px::PxVal::Attrs(fields) = &residual {
        let kind = fields
            .iter()
            .find(|(k, _)| k == "kind")
            .and_then(|(_, v)| match v {
                px::PxVal::Str(s) => Some(s.clone()),
                _ => None,
            });
        if kind.as_deref() == Some("int") || kind.as_deref() == Some("bool") {
            for (k, v) in fields.iter() {
                if k == "value" {
                    folded_value = Some(px::px_print(v));
                }
            }
        }
    }
    let residual_source = match reflect(&residual) {
        Ok(expr) => Some(px::px_emit(&expr)),
        Err(_) => None,
    };
    Ok(MixOutcome {
        residual_node,
        folded_value,
        residual_source,
    })
}

pub struct PolyOutcome {
    pub residual_node: String,
    pub folded_value: Option<String>,
    pub residual_source: Option<String>,
    pub spec_count: usize,
    pub ctr: i64,
}

/// Run the POLYVARIANT px specializer (m6c). Nonempty spec lists assemble
/// into a recursive `let __sK = <param>: <body>; ... in <main>` residual.
pub fn poly_mix_in_px_data(
    px_source: &str,
    statics: &[(String, px::PxVal)],
    granted: &[String],
) -> Result<PolyOutcome, String> {
    // px has no negative literals; "unbounded" is a never-reached sentinel.
    poly_mix_fueled(px_source, statics, granted, 9_000_000_000_000_000)
}

/// Fuel-bounded run (m6f instrumentation): partial specs/ctr are observable
/// at any budget; a huge sentinel acts as unbounded.
pub fn poly_mix_fueled(
    px_source: &str,
    statics: &[(String, px::PxVal)],
    granted: &[String],
    fuel: i64,
) -> Result<PolyOutcome, String> {
    let ast = px::px_parse(px_source)?;
    let encoded = reify(&ast)?;
    let encoded_text = px::px_print(&encoded);
    let mut senv = String::from("{ ");
    for (name, value) in statics {
        senv.push_str(&format!(
            "{} = {}; ",
            name,
            px::px_print(&value_to_mix_node(value)?)
        ));
    }
    senv.push('}');
    let poly_src = interop::host_read_file("runtime/tower/poly_mix.px", granted)?;
    let call = format!(
        "({}) {} {} {{ specs = [ ]; ctr = 0; frames = [ ]; fuel = {}; fvs = [ ]; }}",
        poly_src.trim(),
        encoded_text,
        senv,
        fuel
    );
    let call_ast = px::px_parse(&call)?;
    let env = Vec::new();
    let result = px::px_eval(&call_ast, &env)?;
    poly_result_to_outcome(&result)
}

/// Extract a PolyOutcome from a poly `{ n; st }` RESULT VALUE — shared by
/// direct poly runs and by executing a COGEN artifact (m7), whose value has
/// the same shape.
pub fn poly_result_to_outcome(result: &px::PxVal) -> Result<PolyOutcome, String> {
    let result = &px::px_force_deep(result)?;
    let get = |v: &px::PxVal, name: &str| -> Result<px::PxVal, String> {
        match v {
            px::PxVal::Attrs(fields) => fields
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, val)| val.clone())
                .ok_or_else(|| format!("poly: missing field {}", name)),
            _ => Err(String::from("poly: result is not an attrset")),
        }
    };
    let n = get(result, "n")?;
    let st = get(result, "st")?;
    let specs = match get(&st, "specs")? {
        px::PxVal::List(items) => items.as_ref().clone(),
        _ => return Err(String::from("poly: specs is not a list")),
    };
    let spec_count = specs.len();
    let ctr = match get(&st, "ctr")? {
        px::PxVal::Int(n) => n,
        _ => 0,
    };

    let mut folded_value = None;
    if spec_count == 0 {
        if let px::PxVal::Attrs(fields) = &n {
            let kind = fields
                .iter()
                .find(|(k, _)| k == "kind")
                .and_then(|(_, v)| match v {
                    px::PxVal::Str(s) => Some(s.clone()),
                    _ => None,
                });
            if kind.as_deref() == Some("int") || kind.as_deref() == Some("bool") {
                for (k, v) in fields.iter() {
                    if k == "value" {
                        folded_value = Some(px::px_print(v));
                    }
                }
            }
        }
    }

    let residual_source = (|| -> Result<String, String> {
        let main = reflect(&n)?;
        if spec_count == 0 {
            return Ok(px::px_emit(&main));
        }
        let mut bindings = Vec::new();
        for spec in &specs {
            let name = get(spec, "name").and_then(|v| match v {
                px::PxVal::Str(s) => Ok(s),
                _ => Err(String::from("poly: spec name")),
            })?;
            let param = get(spec, "param").and_then(|v| match v {
                px::PxVal::Str(s) => Ok(s),
                _ => Err(String::from("poly: spec param")),
            })?;
            let body = reflect(&get(spec, "body")?)?;
            bindings.push((
                name,
                px::PxExpr::Lambda {
                    param,
                    body: std::rc::Rc::new(body),
                },
            ));
        }
        Ok(px::px_emit(&px::PxExpr::LetIn {
            bindings,
            body: Box::new(main),
        }))
    })()
    .ok();

    Ok(PolyOutcome {
        residual_node: px::px_print(result),
        folded_value,
        residual_source,
        spec_count,
        ctr,
    })
}

/// Assumed specialization (maps pnix-hy 32 / proposal 0025): a residual is only
/// safe to REUSE if the static assumptions it was built under still hold. This
/// records the assumption fingerprint (the sorted static bindings) so a cached
/// residual can be validated before reuse — reuse iff assumptions hold, else
/// respecialize. Prevents applying a STALE specialization when a static input
/// has changed.
pub fn assumption_hash(statics: &[(String, i64)]) -> String {
    let mut sorted: Vec<(String, i64)> = statics.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut body = String::new();
    for (n, v) in &sorted {
        body.push_str(&format!("{}={};", n, v));
    }
    sha256_hex(body.as_bytes())
}

/// True iff the current static environment matches the residual's assumptions.
pub fn assumptions_hold(built_under: &[(String, i64)], now: &[(String, i64)]) -> bool {
    assumption_hash(built_under) == assumption_hash(now)
}

/// The acceptance criterion for any future third-projection (cogen) attempt,
/// mirroring the pnix-hy harness: a cogen is accepted iff it is
/// SELF-GENERATING — applying it to mix reproduces cogen itself, checked as
/// canonical IR content-hash equality and stamped as a witness. This is the
/// CRITERION machinery; a real cogen remains future work (m5 scope).
pub fn cogen_acceptance(
    cogen_src: &str,
    mix_src: &str,
    apply_fn: &dyn Fn(&str, &str) -> String,
) -> Result<(bool, String, String, crate::gate::Witness), String> {
    let produced = apply_fn(cogen_src, mix_src);
    let ha = crate::ir::ir_of(&produced)?.ir_sha256;
    let hb = crate::ir::ir_of(cogen_src)?.ir_sha256;
    let equal = ha == hb;
    let witness = crate::gate::Witness {
        direction: String::from("cogen-self-generation"),
        source_lang: String::from("px"),
        target_lang: String::from("px"),
        input_kind: String::from("cogen+mix"),
        output_kind: String::from("produced-cogen"),
        loss_status: if equal {
            String::from("lossless")
        } else {
            String::from("rejected")
        },
        effect_class: String::from("pure"),
        capability_required: String::from("-"),
        in_hash: hb.clone(),
        out_hash: ha.clone(),
        env_hash: sha256_hex(mix_src.as_bytes()),
        status: if equal {
            String::from("ok")
        } else {
            String::from("rejected")
        },
        loss: String::from("none"),
    };
    Ok((equal, ha, hb, witness))
}

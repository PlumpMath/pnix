//! IR shape tests - 구조 테스트만, 값 비교 금지

use pnix_core::build_ir::{Arch, Os};
use pnix_core::{check, compile, CompileOptions, SourceUnit};

#[test]
fn compiles_externs_to_ir() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            extern clojure.solve-linear : Matrix -> Vector
            extern py.solve_linear : Matrix -> Vector
        "#
    .into(),
  };

  let out = compile(&src, &CompileOptions::default()).unwrap();

  // "값"이 아니라 "구조"만 검사
  assert_eq!(out.fxcore.morphisms.len(), 2);
  assert_eq!(out.ssa.blocks.len(), 1);
  assert!(!out.artifacts.fxcore_json.is_empty());
  assert!(!out.artifacts.replay_hash.is_empty());
}

#[test]
fn build_ir_includes_deps() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            extern clojure.solve-linear : Matrix -> Vector
            extern py.solve_linear : Matrix -> Vector
            extern deno.render_ui : UiSpec -> Html
        "#
    .into(),
  };

  let out = compile(&src, &CompileOptions::default()).unwrap();

  // 구조 검사: deps에 toolchain이 들어갔는지
  let tools = &out.build_ir.build.deps.toolchain;
  assert!(tools.iter().any(|x| x == "jdk"));
  assert!(tools.iter().any(|x| x == "python"));
  assert!(tools.iter().any(|x| x == "deno"));

  // JSON dump 존재
  assert!(out.artifacts.build_ir_json.contains("\"version\""));
  assert!(out.artifacts.build_ir_json.contains("\"deps\""));
}

#[test]
fn replay_hash_is_stable() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            extern py.solve_linear : Matrix -> Vector
            extern clojure.solve-linear : Matrix -> Vector
        "#
    .into(),
  };

  let a = compile(&src, &CompileOptions::default()).unwrap();
  let b = compile(&src, &CompileOptions::default()).unwrap();

  assert_eq!(a.artifacts.replay_hash, b.artifacts.replay_hash);
  assert!(a.artifacts.replay_hash.len() >= 32);
}

#[test]
fn replay_hash_differs_by_target() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            extern clojure.solve-linear : Matrix -> Vector
        "#
    .into(),
  };

  let opt_linux = CompileOptions {
    target_os: Os::Linux,
    target_arch: Arch::X86_64,
    ..Default::default()
  };

  let opt_darwin = CompileOptions {
    target_os: Os::Darwin,
    target_arch: Arch::Aarch64,
    ..Default::default()
  };

  let a = compile(&src, &opt_linux).unwrap();
  let b = compile(&src, &opt_darwin).unwrap();

  assert_ne!(a.artifacts.replay_hash, b.artifacts.replay_hash);
}

#[test]
fn fails_when_missing_backend_namespace() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            extern solve_linear : Matrix -> Vector
        "#
    .into(),
  };

  let err = check(&src, &CompileOptions::default()).unwrap_err();
  let s = format!("{err}");
  assert!(
    s.contains("meaning closure failed")
      || s.contains("S2")
      || s.contains("unknown morphism/builtin")
      || s.contains("not in spec catalog"),
    "msg={s}"
  );
}

#[test]
fn fails_on_unknown_backend() {
  let src = SourceUnit {
    name: "bad2".into(),
    text: r#"
            type A
            type B
            extern weird.solve : A -> B
        "#
    .into(),
  };

  let err = check(&src, &CompileOptions::default()).unwrap_err();
  let s = format!("{err}");
  // 타입 선언이 있으면 S4(unknown backend)로 실패해야 함
  // 타입 선언이 없으면 S2 이전에 "unknown type"으로 실패할 수 있음
  assert!(
    s.contains("S4")
      || s.contains("unsupported")
      || s.contains("meaning closure")
      || s.contains("unknown type")
  );
}

// Stage-1 tests

#[test]
fn stage1_graph_and_types_ok() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            type Matrix
            type Vector
            type Html

            extern clojure.solve-linear : Matrix -> Vector
            extern deno.render-html : Vector -> Html

            node solve uses clojure.solve-linear
            node render uses deno.render-html

            edge solve -> render
        "#
    .into(),
  };

  let out = check(&src, &CompileOptions::default()).unwrap();
  assert!(out.report.ok);
  assert_eq!(out.fxcore.nodes.len(), 2);
  assert_eq!(out.fxcore.edges.len(), 1);
}

#[test]
fn stage1_edge_type_mismatch_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Matrix
            type Vector
            type Html

            extern clojure.solve-linear : Matrix -> Vector
            extern deno.render-html : Html -> Html

            node solve uses clojure.solve-linear
            node render uses deno.render-html

            edge solve -> render
        "#
    .into(),
  };

  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage1_unknown_node_in_edge_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            extern clojure.solve-linear : Matrix -> Vector

            node solve uses clojure.solve-linear

            edge solve -> unknown
        "#
    .into(),
  };

  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage1_node_uses_unknown_extern_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            extern clojure.solve-linear : Matrix -> Vector

            node solve uses unknown.extern
        "#
    .into(),
  };

  assert!(check(&src, &CompileOptions::default()).is_err());
}

// Stage-2 tests (ported edges)

#[test]
fn stage2_ported_graph_ok() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            type Matrix
            type Vector
            type Html

            extern clojure.solve : (m: Matrix) -> (v: Vector)
            extern deno.render : (data: Vector) -> (html: Html)

            node solve uses clojure.solve
            node render uses deno.render

            edge solve.v -> render.data
        "#
    .into(),
  };

  let out = check(&src, &CompileOptions::default()).unwrap();
  assert!(out.report.ok);
  assert_eq!(out.fxcore.nodes.len(), 2);
  assert_eq!(out.fxcore.edges.len(), 1);
  // Check port info preserved
  assert_eq!(out.fxcore.edges[0].from_port, Some("v".into()));
  assert_eq!(out.fxcore.edges[0].to_port, Some("data".into()));
}

#[test]
fn stage2_multi_input_fan_in_ok() {
  let src = SourceUnit {
    name: "fan_in".into(),
    text: r#"
            type Vector

            extern clojure.gen : (x: Vector) -> (v: Vector)
            extern clojure.add : (a: Vector, b: Vector) -> (sum: Vector)

            node s1 uses clojure.gen
            node s2 uses clojure.gen
            node plus uses clojure.add

            edge s1.v -> plus.a
            edge s2.v -> plus.b
        "#
    .into(),
  };

  let out = check(&src, &CompileOptions::default()).unwrap();
  assert!(out.report.ok);
  assert_eq!(out.fxcore.nodes.len(), 3);
  assert_eq!(out.fxcore.edges.len(), 2);
}

#[test]
fn stage2_ported_type_mismatch_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Vector
            type Html

            extern clojure.solve : (m: Vector) -> (v: Vector)
            extern deno.render : (data: Html) -> (html: Html)

            node solve uses clojure.solve
            node render uses deno.render

            edge solve.v -> render.data
        "#
    .into(),
  };

  // Vector != Html 타입 불일치
  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage2_unknown_output_port_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Vector

            extern clojure.solve : (m: Vector) -> (v: Vector)
            extern clojure.consume : (x: Vector) -> (y: Vector)

            node solve uses clojure.solve
            node consume uses clojure.consume

            edge solve.unknown_port -> consume.x
        "#
    .into(),
  };

  // unknown_port는 solve의 output port에 없음
  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage2_unknown_input_port_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Vector

            extern clojure.solve : (m: Vector) -> (v: Vector)
            extern clojure.consume : (x: Vector) -> (y: Vector)

            node solve uses clojure.solve
            node consume uses clojure.consume

            edge solve.v -> consume.bad_port
        "#
    .into(),
  };

  // bad_port는 consume의 input port에 없음
  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage2_morphism_ports_preserved() {
  let src = SourceUnit {
    name: "ports".into(),
    text: r#"
            type A
            type B
            type C

            extern clojure.multi : (x: A, y: B) -> (p: B, q: C)
        "#
    .into(),
  };

  let out = check(&src, &CompileOptions::default()).unwrap();
  assert!(out.report.ok);

  let mor = &out.fxcore.morphisms[0];
  assert_eq!(mor.inputs.len(), 2);
  assert_eq!(mor.outputs.len(), 2);
  assert_eq!(mor.inputs[0].name, "x");
  assert_eq!(mor.inputs[0].ty, "A");
  assert_eq!(mor.inputs[1].name, "y");
  assert_eq!(mor.inputs[1].ty, "B");
  assert_eq!(mor.outputs[0].name, "p");
  assert_eq!(mor.outputs[0].ty, "B");
  assert_eq!(mor.outputs[1].name, "q");
  assert_eq!(mor.outputs[1].ty, "C");
}

// Stage-2 input tests

#[test]
fn stage2_input_graph_ok() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            type Matrix
            type Vector
            type Html

            input M1 : Matrix
            input M2 : Matrix

            extern clojure.solve : (m: Matrix) -> (v: Vector)
            extern clojure.add : (a: Vector, b: Vector) -> (sum: Vector)
            extern deno.render : (data: Vector) -> (html: Html)

            node s1 uses clojure.solve
            node s2 uses clojure.solve
            node plus uses clojure.add
            node r uses deno.render

            edge input.M1 -> s1.m
            edge input.M2 -> s2.m
            edge s1.v -> plus.a
            edge s2.v -> plus.b
            edge plus.sum -> r.data
        "#
    .into(),
  };

  let out = check(&src, &CompileOptions::default()).unwrap();
  assert!(out.report.ok);
  assert_eq!(out.fxcore.inputs.len(), 2);
  assert_eq!(out.fxcore.nodes.len(), 4);
  assert_eq!(out.fxcore.edges.len(), 5);
  // Verify input info
  assert_eq!(out.fxcore.inputs[0].name, "M1");
  assert_eq!(out.fxcore.inputs[0].ty, "Matrix");
}

#[test]
fn stage2_input_edge_preserved() {
  let src = SourceUnit {
    name: "demo".into(),
    text: r#"
            type Matrix
            type Vector

            input M : Matrix

            extern clojure.solve : (m: Matrix) -> (v: Vector)

            node solve uses clojure.solve

            edge input.M -> solve.m
        "#
    .into(),
  };

  let out = check(&src, &CompileOptions::default()).unwrap();
  assert!(out.report.ok);

  // Verify input edge
  let edge = &out.fxcore.edges[0];
  assert!(edge.is_input_source());
  assert_eq!(edge.from_input, Some("M".to_string()));
  assert_eq!(edge.to, "solve");
  assert_eq!(edge.to_port, Some("m".to_string()));
}

#[test]
fn stage2_unknown_input_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Matrix
            type Vector

            input M1 : Matrix

            extern clojure.solve : (m: Matrix) -> (v: Vector)

            node solve uses clojure.solve

            edge input.UNKNOWN -> solve.m
        "#
    .into(),
  };

  // UNKNOWN input은 선언되지 않음
  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage2_input_type_mismatch_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Matrix
            type Vector

            input V : Vector

            extern clojure.solve : (m: Matrix) -> (v: Vector)

            node solve uses clojure.solve

            edge input.V -> solve.m
        "#
    .into(),
  };

  // Vector != Matrix 타입 불일치
  assert!(check(&src, &CompileOptions::default()).is_err());
}

#[test]
fn stage2_input_without_declaration_fails() {
  let src = SourceUnit {
    name: "bad".into(),
    text: r#"
            type Matrix
            type Vector

            extern clojure.solve : (m: Matrix) -> (v: Vector)

            node solve uses clojure.solve

            edge input.M -> solve.m
        "#
    .into(),
  };

  // input M 선언 없이 input.M 참조
  assert!(check(&src, &CompileOptions::default()).is_err());
}

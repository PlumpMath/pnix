//! batch 263 (2026-04-18): pnix-eval builtin parity 확장 검증.
//!
//! 신규 builtins (substring / stringLength / elem / listToAttrs / removeAttrs
//! / lessThan / add / sub / mul / div) 가 기대대로 동작하는지 확인. 이
//! builtins 는 M1-11 legacy fallback 제거의 전제.

use pnix_eval::{eval_expr, Value};

fn eval(src: &str) -> Value {
  eval_expr(src).unwrap_or_else(|e| panic!("eval error on `{}`: {}", src, e))
}

#[test]
fn substring_takes_slice_by_char() {
  let v = eval(r#"builtins.substring 0 5 "hello world""#);
  assert!(matches!(v, Value::String(ref s) if s == "hello"));
  let v = eval(r#"builtins.substring 6 5 "hello world""#);
  assert!(matches!(v, Value::String(ref s) if s == "world"));
}

#[test]
fn substring_handles_korean_chars() {
  // Nix-correct: substring indices are byte-based, not char-based.
  // Each Hangul syllable is 3 UTF-8 bytes, so to slice "빛은" out
  // of "빛은 뭐야?" we ask for 6 bytes starting at 0. The
  // implementation snaps the end down to the nearest UTF-8
  // boundary so the result is still valid UTF-8 even if the
  // requested length lands mid-codepoint.
  let v = eval(r#"builtins.substring 0 6 "빛은 뭐야?""#);
  assert!(matches!(v, Value::String(ref s) if s == "빛은"));
}

#[test]
fn string_length_counts_bytes() {
  // Nix-correct: stringLength returns the *byte* length of the
  // string (the spec follows C-string / UTF-8 byte count, not
  // Unicode codepoint count). nixpkgs and the upstream test
  // corpus both rely on byte semantics — code that wants
  // codepoint counts can layer it in `.px` or via a domain-
  // specific helper rather than baking that policy into the
  // evaluator.
  let v = eval(r#"builtins.stringLength "hello""#);
  assert!(matches!(v, Value::Int(5)));
  // "안녕요" — 3 syllables × 3 UTF-8 bytes each = 9 bytes.
  let v = eval(r#"builtins.stringLength "안녕요""#);
  assert!(matches!(v, Value::Int(9)));
}

#[test]
fn elem_finds_item_in_list() {
  let v = eval(r#"builtins.elem 2 [1 2 3]"#);
  assert!(matches!(v, Value::Bool(true)));
  let v = eval(r#"builtins.elem 42 [1 2 3]"#);
  assert!(matches!(v, Value::Bool(false)));
  let v = eval(r#"builtins.elem "b" ["a" "b" "c"]"#);
  assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn list_to_attrs_converts_name_value_pairs() {
  let v = eval(r#"builtins.listToAttrs [ { name = "a"; value = 1; } { name = "b"; value = 2; } ]"#);
  match v {
    Value::AttrSet(m) => {
      assert_eq!(m.get("a").and_then(|v| v.as_f64()), Some(1.0));
      assert_eq!(m.get("b").and_then(|v| v.as_f64()), Some(2.0));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn remove_attrs_drops_listed_keys() {
  let v = eval(r#"builtins.removeAttrs { a = 1; b = 2; c = 3; } ["b"]"#);
  match v {
    Value::AttrSet(m) => {
      assert!(m.contains_key("a"));
      assert!(!m.contains_key("b"));
      assert!(m.contains_key("c"));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn less_than_compares_numbers() {
  assert!(matches!(eval("builtins.lessThan 1 2"), Value::Bool(true)));
  assert!(matches!(eval("builtins.lessThan 5 2"), Value::Bool(false)));
  assert!(matches!(eval("builtins.lessThan 2 2"), Value::Bool(false)));
  assert!(matches!(
    eval("builtins.lessThan 1.5 2.0"),
    Value::Bool(true)
  ));
}

#[test]
fn arithmetic_builtins_do_basic_ops() {
  assert!(matches!(eval("builtins.add 2 3"), Value::Int(5)));
  assert!(matches!(eval("builtins.sub 10 3"), Value::Int(7)));
  assert!(matches!(eval("builtins.mul 4 5"), Value::Int(20)));
  assert!(matches!(eval("builtins.div 20 4"), Value::Int(5)));
  // int/float promote.
  match eval("builtins.add 1 2.5") {
    Value::Float(f) => assert!((f - 3.5).abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
}

#[test]
fn new_builtins_compose_with_existing() {
  // listToAttrs + attrNames
  let v = eval(
    r#"builtins.attrNames (builtins.listToAttrs [ { name = "x"; value = 1; } { name = "y"; value = 2; } ])"#,
  );
  match v {
    Value::List(items) => {
      let names: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
      assert_eq!(names, vec!["x", "y"]);
    }
    other => panic!("expected List, got {:?}", other),
  }
  // elem + filter
  let v = eval(r#"builtins.filter (x: builtins.elem x [2 4 6]) [1 2 3 4 5 6 7]"#);
  match v {
    Value::List(items) => {
      let nums: Vec<i64> = items
        .iter()
        .filter_map(|v| match v {
          Value::Int(i) => Some(*i),
          _ => None,
        })
        .collect();
      assert_eq!(nums, vec![2, 4, 6]);
    }
    other => panic!("expected List, got {:?}", other),
  }
}

#[test]
fn current_system_and_nix_version_are_exposed_as_values() {
  let system = eval("builtins.currentSystem");
  assert!(matches!(system, Value::String(ref s) if s.contains('-')));

  let nix_version = eval("builtins.nixVersion");
  assert!(matches!(nix_version, Value::String(ref s) if s == "2.18.0-pnix"));

  let store_dir = eval("builtins.storeDir");
  assert!(matches!(store_dir, Value::String(ref s) if s == "/nix/store"));
}

#[test]
fn version_helpers_follow_legacy_shapes() {
  assert!(matches!(
    eval(r#"builtins.compareVersions "1.2.3" "1.10.0""#),
    Value::Int(-1)
  ));
  assert!(matches!(
    eval(r#"builtins.compareVersions "2.0" "2.0""#),
    Value::Int(0)
  ));
  assert!(matches!(
    eval(r#"builtins.compareVersions "2.0" "1.9""#),
    Value::Int(1)
  ));

  let split = eval(r#"builtins.splitVersion "1.2-rc1""#);
  match split {
    Value::List(items) => {
      let parts: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
      assert_eq!(parts, vec!["1", "2", "rc", "1"]);
    }
    other => panic!("expected List, got {:?}", other),
  }

  let drv = eval(r#"builtins.parseDrvName "hello-1.2.3""#);
  match drv {
    Value::AttrSet(map) => {
      assert_eq!(map.get("name").and_then(|v| v.as_str()), Some("hello"));
      assert_eq!(map.get("version").and_then(|v| v.as_str()), Some("1.2.3"));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn abort_and_throw_surface_errors() {
  let abort_err =
    pnix_eval::eval_expr(r#"builtins.abort "stop now""#).expect_err("abort must error");
  assert!(abort_err
    .to_string()
    .contains("evaluation aborted: stop now"));

  let throw_err = pnix_eval::eval_expr(r#"builtins.throw "boom""#).expect_err("throw must error");
  assert!(throw_err.to_string().contains("boom"));
}

#[test]
fn get_attr_reads_named_field_or_errors() {
  let value = eval(r#"builtins.getAttr "answer" { answer = 42; }"#);
  assert!(matches!(value, Value::Int(42)));

  let err = pnix_eval::eval_expr(r#"builtins.getAttr "missing" { answer = 42; }"#)
    .expect_err("missing attr must error");
  assert!(err.to_string().contains("attribute 'missing' not found"));
}

#[test]
fn try_eval_catches_failures_even_when_builtin_is_aliased() {
  let ok = eval(r#"builtins.tryEval (1 + 2)"#);
  match ok {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("success"), Some(Value::Bool(true))));
      assert!(matches!(map.get("value"), Some(Value::Int(3))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let failed = eval(r#"let t = builtins.tryEval; in t (builtins.throw "boom")"#);
  match failed {
    Value::AttrSet(map) => {
      // Nix-correct shape: { success = false; value = false; }.
      // Previous expectation `value = null` was wrong — see the
      // 2026-05-04 audit slice that fixed `try_eval_result` to
      // return `Value::Bool(false)` on the error branch (matches
      // the manual: "value, equal to e if successful and false on
      // error"). Pinned in `eval_tryeval_paths.rs`.
      assert!(matches!(map.get("success"), Some(Value::Bool(false))));
      assert!(matches!(map.get("value"), Some(Value::Bool(false))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn seq_and_deep_seq_return_second_value() {
  assert!(matches!(eval("builtins.seq 1 2"), Value::Int(2)));
  assert!(matches!(
    eval(r#"builtins.deepSeq { a = [1 2 3]; } "done""#),
    Value::String(ref s) if s == "done"
  ));
}

#[test]
fn match_and_split_follow_regex_contract() {
  let matched = eval(r#"builtins.match "a(b+)" "abbb""#);
  match matched {
    Value::List(items) => {
      let captures: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
      assert_eq!(captures, vec!["bbb"]);
    }
    other => panic!("expected List, got {:?}", other),
  }

  assert!(matches!(
    eval(r#"builtins.match "a(b+)" "ccc""#),
    Value::Null
  ));

  let split = eval(r#"builtins.split "," "a,b,c""#);
  match split {
    Value::List(items) => {
      assert_eq!(items.len(), 5);
      assert!(matches!(items[0], Value::String(ref s) if s == "a"));
      assert!(matches!(items[1], Value::List(ref empty) if empty.is_empty()));
      assert!(matches!(items[2], Value::String(ref s) if s == "b"));
      assert!(matches!(items[3], Value::List(ref empty) if empty.is_empty()));
      assert!(matches!(items[4], Value::String(ref s) if s == "c"));
    }
    other => panic!("expected List, got {:?}", other),
  }
}

#[test]
fn attrset_higher_order_builtins_work() {
  let mapped = eval(
    r#"
    builtins.mapAttrs
      (name: value: if name == "a" then value + 10 else value + 20)
      { a = 1; b = 2; }
  "#,
  );
  match mapped {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("a"), Some(Value::Int(11))));
      assert!(matches!(map.get("b"), Some(Value::Int(22))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let filtered = eval(
    r#"
    builtins.filterAttrs
      (name: value: name == "keep" && value > 0)
      { keep = 1; drop = 0; }
  "#,
  );
  match filtered {
    Value::AttrSet(map) => {
      assert!(map.contains_key("keep"));
      assert!(!map.contains_key("drop"));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let intersected = eval(
    r#"
    builtins.intersectAttrs { a = 1; b = 2; } { b = 9; c = 3; }
  "#,
  );
  match intersected {
    Value::AttrSet(map) => {
      assert_eq!(map.len(), 1);
      // Nix-compat: `intersectAttrs e1 e2` returns the attributes of
      // `e2` whose names also exist in `e1`. Values come from the SECOND
      // argument, so `b` here is 9 (from {b=9; c=3;}), not 2.
      assert!(matches!(map.get("b"), Some(Value::Int(9))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn list_partitioning_builtins_work() {
  let grouped = eval(r#"builtins.groupBy (x: if x < 3 then "small" else "big") [1 2 3 4]"#);
  match grouped {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("small"), Some(Value::List(items)) if items.len() == 2));
      assert!(matches!(map.get("big"), Some(Value::List(items)) if items.len() == 2));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let cat = eval(r#"builtins.catAttrs "x" [ { x = 1; } { y = 2; } { x = 3; } ]"#);
  match cat {
    Value::List(items) => {
      let nums: Vec<i64> = items
        .iter()
        .filter_map(|v| match v {
          Value::Int(i) => Some(*i),
          _ => None,
        })
        .collect();
      assert_eq!(nums, vec![1, 3]);
    }
    other => panic!("expected List, got {:?}", other),
  }

  let partitioned = eval(r#"builtins.partition (x: x < 3) [1 2 3 4]"#);
  match partitioned {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("right"), Some(Value::List(items)) if items.len() == 2));
      assert!(matches!(map.get("wrong"), Some(Value::List(items)) if items.len() == 2));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn trace_returns_second_argument() {
  assert!(matches!(
    eval(r#"builtins.trace "hello" 42"#),
    Value::Int(42)
  ));
}

#[test]
fn function_args_reports_named_params_and_defaults() {
  let value = eval(r#"builtins.functionArgs ({ x, y ? 1, ... }: x + y)"#);
  match value {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("x"), Some(Value::Bool(false))));
      assert!(matches!(map.get("y"), Some(Value::Bool(true))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let builtin = eval(r#"builtins.functionArgs builtins.map"#);
  match builtin {
    Value::AttrSet(map) => assert!(map.is_empty()),
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn zip_attrs_with_merges_union_of_keys() {
  let value = eval(
    r#"
    builtins.zipAttrsWith
      (name: values:
        builtins.concatStringsSep ":" [name (builtins.toString (builtins.length values))])
      [
        { a = 1; b = 2; }
        { a = 3; c = 4; }
      ]
  "#,
  );

  match value {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("a"), Some(Value::String(s)) if s == "a:2"));
      assert!(matches!(map.get("b"), Some(Value::String(s)) if s == "b:1"));
      assert!(matches!(map.get("c"), Some(Value::String(s)) if s == "c:1"));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn boolean_and_comparison_aliases_work() {
  assert!(matches!(eval("builtins.and true true"), Value::Bool(true)));
  assert!(matches!(eval("builtins.or false true"), Value::Bool(true)));
  assert!(matches!(eval("builtins.not false"), Value::Bool(true)));
  assert!(matches!(
    eval(r#"builtins.eq "ab" ("a" + "b")"#),
    Value::Bool(true)
  ));
  assert!(matches!(eval("builtins.lt 1 2"), Value::Bool(true)));
  assert!(matches!(eval("builtins.le 2 2"), Value::Bool(true)));
  assert!(matches!(eval("builtins.gt 3 2"), Value::Bool(true)));
  assert!(matches!(eval("builtins.ge 3 3"), Value::Bool(true)));
}

#[test]
fn math_builtins_cover_numeric_surface_used_by_fixtures() {
  assert!(matches!(eval("builtins.mod 17 5"), Value::Int(2)));
  assert!(matches!(eval("builtins.neg 3"), Value::Int(-3)));
  assert!(matches!(eval("builtins.abs (-4)"), Value::Int(4)));
  assert!(matches!(eval("builtins.pow 2 5"), Value::Int(32)));
  match eval("builtins.sqrt 25") {
    Value::Float(v) => assert!((v - 5.0).abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
  assert!(matches!(eval("builtins.floor 3.9"), Value::Int(3)));
  assert!(matches!(eval("builtins.ceil 3.1"), Value::Int(4)));
  match eval("builtins.exp 1") {
    Value::Float(v) => assert!((v - std::f64::consts::E).abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
  match eval("builtins.ln 1") {
    Value::Float(v) => assert!(v.abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
  match eval("builtins.sin 0") {
    Value::Float(v) => assert!(v.abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
  match eval("builtins.cos 0") {
    Value::Float(v) => assert!((v - 1.0).abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
  match eval("builtins.atan2 0 1") {
    Value::Float(v) => assert!(v.abs() < 1e-9),
    other => panic!("expected Float, got {:?}", other),
  }
}

#[test]
fn warn_trace_verbose_and_x3d_builtins_work_on_direct_path() {
  assert!(matches!(eval(r#"builtins.warn "hello" 7"#), Value::Int(7)));
  assert!(matches!(
    eval(r#"builtins.traceVerbose "hello" 9"#),
    Value::Int(9)
  ));

  let value = eval(
    r#"
    let
      xml = ''
      <X3D profile='Interchange' version='4.0'>
        <Scene>
          <Transform DEF='Mover'>
            <Shape><Box size='1 1 1'/></Shape>
          </Transform>
          <TimeSensor DEF='Clock' cycleInterval='2' loop='true'/>
          <PositionInterpolator DEF='Move' key='0 1' keyValue='0 0 0  1 0 0'/>
          <ROUTE fromNode='Clock' fromField='fraction_changed' toNode='Move' toField='set_fraction'/>
          <ROUTE fromNode='Move' fromField='value_changed' toNode='Mover' toField='translation'/>
        </Scene>
      </X3D>
      '';
      json = builtins.x3dXmlToJson xml;
      validate = builtins.x3dSchemaValidate json;
      explain = builtins.x3dSchemaExplain json;
      frp = builtins.x3dFrpGraph xml;
    in {
      json = json;
      validate = validate;
      explain = explain;
      frp = frp;
    }
  "#,
  );

  match value {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("json"), Some(Value::AttrSet(attrs)) if attrs.contains_key("name")));
      assert!(matches!(
        map.get("validate"),
        Some(Value::AttrSet(attrs)) if attrs.contains_key("ok") && attrs.contains_key("errors")
      ));
      assert!(matches!(map.get("explain"), Some(Value::String(_))));
      match map.get("frp") {
        Some(Value::AttrSet(frp)) => {
          assert!(matches!(
            frp.get("external_inputs"),
            Some(Value::AttrSet(_))
          ));
          assert!(matches!(
            frp.get("signals"),
            Some(Value::List(signals)) if !signals.is_empty()
          ));
          assert!(matches!(
            frp.get("routes"),
            Some(Value::List(routes)) if routes.len() == 2
          ));
        }
        other => panic!("expected frp attrset, got {:?}", other),
      }
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn x3d_sync_plan_emits_patch_friendly_ops() {
  let value = eval(
    r#"
    builtins.x3dSyncPlan
      ''
      <X3D><Scene><Transform DEF='Mover' translation='0 0 0'/></Scene></X3D>
      ''
      ''
      <X3D><Scene><Transform DEF='Mover' translation='1 0 0'/></Scene></X3D>
      ''
  "#,
  );

  match value {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("mode"), Some(Value::String(mode)) if mode == "patch"));
      assert!(matches!(map.get("changed"), Some(Value::Bool(true))));
      assert!(matches!(map.get("scene"), Some(Value::AttrSet(_))));
      assert!(matches!(
        map.get("ops"),
        Some(Value::List(ops)) if ops.iter().any(|op| matches!(op, Value::AttrSet(attrs) if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs")))
      ));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn x3d_x3dom_webview_payload_is_directly_renderable() {
  let value = eval(
    r#"
    let
      previous = ''
      <X3D><Scene><Transform DEF='Mover' translation='0 0 0'><Shape><Box size='1 1 1'/></Shape></Transform></Scene></X3D>
      '';
      next = ''
      <X3D><Scene><Transform DEF='Mover' translation='1 0 0'><Shape><Box size='1 1 1'/></Shape></Transform></Scene></X3D>
      '';
    in {
      fragment = builtins.x3dX3domFragment next;
      html = builtins.x3dX3domHtml next;
      patch = builtins.x3dX3domPatch previous next;
    }
  "#,
  );

  match value {
    Value::AttrSet(map) => {
      assert!(matches!(
        map.get("fragment"),
        Some(Value::String(fragment))
          if fragment.contains("data-node-id=\"def:mover\"")
            && fragment.contains(
              "data-pnix-address=\"root/x3d[0]/scene[0]/transform#mover\""
            )
      ));
      assert!(matches!(
        map.get("html"),
        Some(Value::String(html)) if html.contains("data-pnix-x3d-root=\"true\"")
      ));
      match map.get("patch") {
        Some(Value::AttrSet(patch)) => {
          assert!(matches!(
            patch.get("protocol"),
            Some(Value::String(protocol)) if protocol == "pnix.x3dom.patch.v1"
          ));
          assert!(matches!(
            patch.get("engine"),
            Some(Value::String(engine)) if engine == "x3dom-ssr"
          ));
          assert!(matches!(
            patch.get("ops"),
            Some(Value::List(ops))
              if ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
                if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
                  && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "root/x3d[0]/scene[0]/transform#mover")))
          ));
        }
        other => panic!("expected patch attrset, got {:?}", other),
      }
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn x3d_render_packet_treats_process_as_api_packet() {
  let value = eval(
    r#"
    builtins.x3dRenderPacket
      ''
      <X3D><Scene><Transform DEF='Mover' translation='0 0 0'/><HAnimHumanoid DEF='Avatar' version='2.0'><HAnimJoint DEF='Root' name='humanoid_root'/></HAnimHumanoid></Scene></X3D>
      ''
      ''
      <X3D><Scene><Transform DEF='Mover' translation='1 0 0'/><TimeSensor DEF='Clock' cycleInterval='2' loop='true'/><PositionInterpolator DEF='Move' key='0 1' keyValue='0 0 0  1 0 0'/><ROUTE fromNode='Clock' fromField='fraction_changed' toNode='Move' toField='set_fraction'/><ROUTE fromNode='Move' fromField='value_changed' toNode='Mover' toField='translation'/><ParticleSystem DEF='Dust'><ForcePhysicsModel DEF='Gravity' enabled='true'/></ParticleSystem><HAnimHumanoid DEF='Avatar' version='2.0'><HAnimJoint DEF='Root' name='humanoid_root'><HAnimJoint DEF='Hip' name='l_hip_joint'/></HAnimJoint></HAnimHumanoid></Scene></X3D>
      ''
  "#,
  );

  match value {
    Value::AttrSet(map) => {
      assert!(matches!(
        map.get("protocol"),
        Some(Value::String(protocol)) if protocol == "pnix.render.packet.v1"
      ));
      assert!(matches!(map.get("process_api"), Some(Value::Bool(true))));
      assert!(matches!(
        map.get("lowerings"),
        Some(Value::AttrSet(lowerings))
          if matches!(lowerings.get("x3dom"), Some(Value::AttrSet(_)))
            && matches!(lowerings.get("wgpu"), Some(Value::AttrSet(wgpu)) if matches!(wgpu.get("status"), Some(Value::String(status)) if status == "pending"))
      ));
      match map.get("simulation") {
        Some(Value::AttrSet(simulation)) => {
          assert!(matches!(
            simulation.get("frp"),
            Some(Value::AttrSet(frp)) if matches!(frp.get("signal_count"), Some(Value::Int(count)) if *count > 0)
          ));
          assert!(matches!(
            simulation.get("physics"),
            Some(Value::AttrSet(physics)) if matches!(physics.get("detected_tags"), Some(Value::List(tags)) if tags.iter().any(|tag| matches!(tag, Value::String(name) if name == "force-physics-model")))
          ));
          assert!(matches!(
            simulation.get("hanim"),
            Some(Value::AttrSet(hanim)) if matches!(hanim.get("root_joint"), Some(Value::String(root)) if root == "humanoid_root")
          ));
          assert!(matches!(
            simulation.get("symbolic"),
            Some(Value::AttrSet(symbolic))
              if matches!(symbolic.get("present"), Some(Value::Bool(true)))
                && matches!(symbolic.get("equation_count"), Some(Value::Int(count)) if *count >= 4)
                && matches!(symbolic.get("state_variables"), Some(Value::List(vars)) if vars.iter().any(|var| matches!(var, Value::String(name) if name == "Clock.fraction_changed")))
          ));
        }
        other => panic!("expected simulation attrset, got {:?}", other),
      }
      assert!(matches!(
        map.get("memory"),
        Some(Value::AttrSet(memory))
          if matches!(memory.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.world.memory.v1")
            && matches!(memory.get("surfaces"), Some(Value::List(surfaces)) if surfaces.iter().any(|item| matches!(item, Value::String(name) if name == "x3d")))
      ));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn list_and_attrset_aliases_work() {
  assert!(matches!(
    eval("builtins.append [1 2] [3 4]"),
    Value::List(items) if matches!(items.as_slice(), [Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)])
  ));
  assert!(matches!(
    eval("builtins.cons 0 [1 2]"),
    Value::List(items) if matches!(items.as_slice(), [Value::Int(0), Value::Int(1), Value::Int(2)])
  ));
  assert!(matches!(
    eval("builtins.take 2 [1 2 3]"),
    Value::List(items) if matches!(items.as_slice(), [Value::Int(1), Value::Int(2)])
  ));
  assert!(matches!(
    eval("builtins.drop 1 [1 2 3]"),
    Value::List(items) if matches!(items.as_slice(), [Value::Int(2), Value::Int(3)])
  ));
  assert!(matches!(
    eval("builtins.reverseList [1 2 3]"),
    Value::List(items) if matches!(items.as_slice(), [Value::Int(3), Value::Int(2), Value::Int(1)])
  ));

  let zipped = eval(r#"builtins.zip [1 2] ["a" "b" "c"]"#);
  match zipped {
    Value::List(items) => {
      assert_eq!(items.len(), 2);
      assert!(
        matches!(items[0], Value::List(ref pair) if matches!(pair.as_slice(), [Value::Int(1), Value::String(ref s)] if s == "a"))
      );
      assert!(
        matches!(items[1], Value::List(ref pair) if matches!(pair.as_slice(), [Value::Int(2), Value::String(ref s)] if s == "b"))
      );
    }
    other => panic!("expected List, got {:?}", other),
  }

  let flattened = eval("builtins.flatten [1 [2 [3]]]");
  assert!(matches!(
    flattened,
    Value::List(items) if matches!(items.as_slice(), [Value::Int(1), Value::Int(2), Value::Int(3)])
  ));

  let concat_mapped = eval("builtins.concatMap (x: [x (x + 10)]) [1 2]");
  assert!(matches!(
    concat_mapped,
    Value::List(items) if matches!(items.as_slice(), [Value::Int(1), Value::Int(11), Value::Int(2), Value::Int(12)])
  ));

  let found = eval("builtins.find 2 [1 2 3]");
  assert!(matches!(found, Value::Int(2)));

  let get = eval(r#"builtins.get { x = 1; } "x""#);
  assert!(matches!(get, Value::Int(1)));
  assert!(matches!(
    eval(r#"builtins.get { x = 1; } "y""#),
    Value::Null
  ));

  let set = eval(r#"builtins.set { x = 1; } "y" 2"#);
  match set {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("x"), Some(Value::Int(1))));
      assert!(matches!(map.get("y"), Some(Value::Int(2))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let keys = eval(r#"builtins.keys { a = 1; b = 2; }"#);
  assert!(matches!(
    keys,
    Value::List(items) if matches!(items.as_slice(), [Value::String(ref a), Value::String(ref b)] if a == "a" && b == "b")
  ));

  let values = eval(r#"builtins.values { a = 1; b = 2; }"#);
  assert!(matches!(
    values,
    Value::List(items) if matches!(items.as_slice(), [Value::Int(1), Value::Int(2)])
  ));

  let merged = eval(r#"builtins.merge { a = 1; } { b = 2; }"#);
  match merged {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("a"), Some(Value::Int(1))));
      assert!(matches!(map.get("b"), Some(Value::Int(2))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn schema_builtins_validate_normalize_and_explain() {
  let normalized = eval(
    r#"
    builtins.schemaNormalize
      (rec {
        string = { kind = "string"; };
        root = {
          kind = "record";
          fields = {
            name = string;
            enabled = { kind = "bool"; default = true; };
          };
          optional = [ "enabled" ];
        };
      })
      { name = "demo"; }
  "#,
  );
  match normalized {
    Value::AttrSet(map) => {
      assert_eq!(map.get("name").and_then(Value::as_str), Some("demo"));
      assert!(matches!(map.get("enabled"), Some(Value::Bool(true))));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let report = eval(
    r#"
    builtins.schemaValidate
      { kind = "record"; fields = { name = { kind = "string"; }; }; }
      { name = "ok"; }
  "#,
  );
  match report {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("success"), Some(Value::Bool(true))));
      assert!(matches!(map.get("errors"), Some(Value::List(items)) if items.is_empty()));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let explanation = eval(
    r#"
    builtins.schemaExplain
      { kind = "record"; fields = { name = { kind = "string"; }; }; }
      { name = 1; }
  "#,
  );
  assert!(matches!(
    explanation,
    Value::String(ref text) if text.contains("expected string")
  ));
}

#[test]
fn xml_and_html_builtins_roundtrip_simple_markup() {
  let parsed = eval(r#"builtins.xmlParse "<root a=\"1\"><child>text</child></root>""#);
  match parsed {
    Value::AttrSet(map) => {
      assert_eq!(map.get("kind").and_then(Value::as_str), Some("element"));
      assert_eq!(map.get("name").and_then(Value::as_str), Some("root"));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let emitted = eval(
    r#"
    builtins.xmlEmit {
      kind = "element";
      name = "root";
      attrs = { a = "1"; };
      children = [
        {
          kind = "element";
          name = "child";
          children = [ { kind = "text"; value = "text"; } ];
        }
      ];
    }
  "#,
  );
  assert!(matches!(
    emitted,
    Value::String(ref text) if text == "<root a=\"1\"><child>text</child></root>"
  ));

  let html_parsed = eval(r#"builtins.htmlParse "<div class=\"test\">Hello</div>""#);
  match html_parsed {
    Value::AttrSet(map) => {
      assert_eq!(map.get("kind").and_then(Value::as_str), Some("document"));
    }
    other => panic!("expected document AttrSet, got {:?}", other),
  }

  let html_emitted =
    eval(r#"builtins.htmlEmit (builtins.htmlParse "<div class=\"test\">Hello</div>")"#);
  assert!(matches!(
    html_emitted,
    Value::String(ref text) if text.contains("<div class=\"test\">Hello</div>")
  ));
}

#[test]
fn mathml_and_openmath_builtins_roundtrip_domain_markup() {
  let mathml_xml = eval(
    r#"
    builtins.mathmlEmit {
      kind = "mathml";
      display = "inline";
      elements = [
        {
          name = "msqrt";
          children = [ { name = "mn"; text = "9"; } ];
        }
      ];
    }
  "#,
  );
  assert!(matches!(
    mathml_xml,
    Value::String(ref text) if text.contains("<math") && text.contains("<msqrt>")
  ));

  let mathml_json = eval(r#"builtins.mathmlXmlToJson "<math><mi>x</mi></math>""#);
  match mathml_json {
    Value::AttrSet(map) => assert_eq!(map.get("name").and_then(Value::as_str), Some("math")),
    other => panic!("expected AttrSet, got {:?}", other),
  }

  let openmath_xml = eval(
    r#"
    builtins.openmathEmit {
      expressions = [
        {
          name = "OMA";
          children = [
            { name = "OMS"; attrs = { cd = "pnix-arith"; name = "plus"; }; }
            { name = "OMI"; text = "1"; }
            { name = "OMI"; text = "2"; }
          ];
        }
      ];
    }
  "#,
  );
  assert!(matches!(
    openmath_xml,
    Value::String(ref text) if text.contains("<OMA") && text.contains("cd=\"pnix-arith\"")
  ));

  let openmath_json = eval(r#"builtins.openmathXmlToJson "<OMOBJ><OMI>1</OMI></OMOBJ>""#);
  match openmath_json {
    Value::AttrSet(map) => assert_eq!(map.get("name").and_then(Value::as_str), Some("OMOBJ")),
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn svg_schema_builtins_close_2d_schema_surface() {
  let normalized = eval(
    r##"
    builtins.svgSchemaNormalize ''
      <svg width="10" height="10">
        <a href="#demo"/>
        <circle cx="5" cy="5" r="4"/>
      </svg>
    ''
  "##,
  );
  match normalized {
    Value::AttrSet(map) => {
      assert!(matches!(
        map.get("name"),
        Some(Value::String(name)) if name == "svg"
      ));
      assert!(matches!(
        map.get("children"),
        Some(Value::List(children))
          if children.iter().any(|child| matches!(child, Value::AttrSet(node) if matches!(node.get("name"), Some(Value::String(name)) if name == "a") && matches!(node.get("attrs"), Some(Value::AttrSet(attrs)) if matches!(attrs.get("show"), Some(Value::String(show)) if show == "replace"))))
      ));
    }
    other => panic!("expected normalized svg attrset, got {:?}", other),
  }

  let validation = eval(
    r#"
    builtins.svgSchemaValidate {
      kind = "element";
      name = "svg";
      attrs = { version = "3.0"; };
      children = [
        {
          kind = "element";
          name = "circle";
          attrs = { bogus = "1"; };
        }
      ];
    }
  "#,
  );
  match validation {
    Value::AttrSet(map) => {
      assert!(matches!(map.get("ok"), Some(Value::Bool(false))));
      assert!(matches!(
        map.get("version"),
        Some(Value::String(version)) if version == "3.0"
      ));
      assert!(matches!(
        map.get("errors"),
        Some(Value::List(errors))
          if errors.iter().any(|err| matches!(err, Value::String(text) if text.contains("unsupported version '3.0'")))
            && errors.iter().any(|err| matches!(err, Value::String(text) if text.contains("unknown attribute 'bogus'")))
      ));
    }
    other => panic!("expected svg validation report, got {:?}", other),
  }

  let explanation = eval(
    r#"
    builtins.svgSchemaExplain {
      kind = "element";
      name = "svg";
      attrs = {};
      children = [ { kind = "element"; name = "foo"; attrs = {}; } ];
    }
  "#,
  );
  assert!(matches!(
    explanation,
    Value::String(text) if text.contains("unknown SVG element 'foo'")
  ));
}

#[test]
fn svg_emit_and_render_packet_close_2d_render_surface() {
  let emitted = eval(
    r##"
    builtins.svgEmit {
      kind = "element";
      name = "svg";
      attrs = { width = "10"; height = "10"; };
      children = [
        { kind = "element"; name = "circle"; attrs = { id = "dot"; cx = "5"; cy = "5"; r = "2"; }; }
      ];
    }
  "##,
  );
  assert!(matches!(
    emitted,
    Value::String(text)
      if text.contains("<svg")
        && text.contains("xmlns=\"http://www.w3.org/2000/svg\"")
        && text.contains("<circle")
  ));

  let packet = eval(
    r##"
    builtins.svgRenderPacket
      ''
      <svg width="10" height="10"><circle id="dot" cx="5" cy="5" r="2"/></svg>
      ''
      ''
      <svg width="10" height="10"><circle id="dot" cx="7" cy="5" r="2" fill="blue"/></svg>
      ''
  "##,
  );
  match packet {
    Value::AttrSet(map) => {
      assert!(matches!(
        map.get("protocol"),
        Some(Value::String(protocol)) if protocol == "pnix.render.packet.v1"
      ));
      assert!(matches!(
        map.get("family"),
        Some(Value::String(family)) if family == "svg"
      ));
      assert!(matches!(map.get("process_api"), Some(Value::Bool(true))));
      assert!(matches!(
        map.get("memory"),
        Some(Value::AttrSet(memory))
          if matches!(memory.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.world.memory.v1")
            && matches!(memory.get("surfaces"), Some(Value::List(surfaces)) if surfaces.iter().any(|item| matches!(item, Value::String(name) if name == "svg")))
      ));
      assert!(matches!(
        map.get("lowerings"),
        Some(Value::AttrSet(lowerings))
          if matches!(lowerings.get("html"), Some(Value::AttrSet(html))
            if matches!(html.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.svg.html.v1")
              && matches!(html.get("fragment"), Some(Value::String(fragment)) if fragment.contains("data-node-id=\"id:dot\""))
              && matches!(html.get("ops"), Some(Value::List(ops))
                if ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                  if matches!(op.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
                    && matches!(op.get("target_node_id"), Some(Value::String(id)) if id == "id:dot")))))
      ));
      assert!(matches!(
        map.get("sync"),
        Some(Value::AttrSet(sync))
          if matches!(sync.get("mode"), Some(Value::String(mode)) if mode == "patch")
            && matches!(sync.get("ops"), Some(Value::List(ops))
              if ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                if matches!(op.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
                  && matches!(op.get("node_id"), Some(Value::String(id)) if id == "id:dot"))))
      ));
    }
    other => panic!("expected svg render packet attrset, got {:?}", other),
  }
}

#[test]
fn svg_render_packet_emits_child_delta_ops() {
  let packet = eval(
    r##"
    builtins.svgRenderPacket
      ''
      <svg width="10" height="10">
        <circle id="a" cx="1" cy="1" r="1"/>
        <circle id="b" cx="2" cy="2" r="1"/>
        <circle id="d" cx="3" cy="3" r="1"/>
      </svg>
      ''
      ''
      <svg width="10" height="10">
        <circle id="b" cx="2" cy="2" r="1" fill="blue"/>
        <circle id="a" cx="1" cy="1" r="1"/>
        <rect id="c" width="1" height="1"/>
      </svg>
      ''
  "##,
  );

  match packet {
    Value::AttrSet(map) => {
      assert!(matches!(
        map.get("sync"),
        Some(Value::AttrSet(sync))
          if matches!(sync.get("ops"), Some(Value::List(ops))
            if ops.iter().any(|op| matches!(op, Value::AttrSet(op)
              if matches!(op.get("op"), Some(Value::String(kind)) if kind == "remove_child")
                && matches!(op.get("child_node_id"), Some(Value::String(id)) if id == "id:d")))
              && ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                if matches!(op.get("op"), Some(Value::String(kind)) if kind == "insert_child")
                  && matches!(op.get("child_node_id"), Some(Value::String(id)) if id == "id:c")))
              && ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                if matches!(op.get("op"), Some(Value::String(kind)) if kind == "reorder_children")
                  && matches!(op.get("child_node_ids"), Some(Value::List(child_ids))
                    if matches!(child_ids.as_slice(),
                      [Value::String(first), Value::String(second), Value::String(third)]
                        if first == "id:b" && second == "id:a" && third == "id:c")))))
      ));
      assert!(matches!(
        map.get("lowerings"),
        Some(Value::AttrSet(lowerings))
          if matches!(lowerings.get("html"), Some(Value::AttrSet(html))
            if matches!(html.get("ops"), Some(Value::List(ops))
              if ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                if matches!(op.get("op"), Some(Value::String(kind)) if kind == "remove_child")
                  && matches!(op.get("child_node_id"), Some(Value::String(id)) if id == "id:d")))
                && ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                  if matches!(op.get("op"), Some(Value::String(kind)) if kind == "insert_child")
                    && matches!(op.get("child_node_id"), Some(Value::String(id)) if id == "id:c")))
                && ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                  if matches!(op.get("op"), Some(Value::String(kind)) if kind == "reorder_children")
                    && matches!(op.get("child_node_ids"), Some(Value::List(child_ids))
                      if matches!(child_ids.as_slice(),
                        [Value::String(first), Value::String(second), Value::String(third)]
                          if first == "id:b" && second == "id:a" && third == "id:c"))))))
      ));
    }
    other => panic!("expected svg render packet attrset, got {:?}", other),
  }
}

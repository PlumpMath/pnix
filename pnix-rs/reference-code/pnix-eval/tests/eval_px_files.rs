//! doghouse data/*.px 파일들을 pnix-eval로 evaluate하는 테스트.

use pnix_eval::{eval_file, Value};
use std::path::Path;

fn data_path(name: &str) -> std::path::PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
  Path::new(&base).join("../doghouse-core/data").join(name)
}

fn fixture_path(group: &str, name: &str) -> std::path::PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
  Path::new(&base)
    .join("../../fixtures")
    .join(group)
    .join(name)
}

#[test]
fn eval_absorb_policy_px() {
  let path = data_path("absorb-policy.px");
  if !path.exists() {
    return;
  } // CI에서 경로가 다를 수 있음
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(
      map.contains_key("thresholds"),
      "absorb-policy.px must have thresholds"
    );
    assert!(
      map.contains_key("source-tiers"),
      "absorb-policy.px must have source-tiers"
    );
  } else {
    panic!("absorb-policy.px must eval to attrset, got: {:?}", v);
  }
}

#[test]
fn eval_unit_conversions_px() {
  let path = data_path("unit-conversions.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(
      map.contains_key("conversions"),
      "unit-conversions.px must have conversions"
    );
  } else {
    panic!("unit-conversions.px must eval to attrset");
  }
}

#[test]
fn eval_auto_learn_policy_px() {
  let path = data_path("auto-learn-policy.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(
      map.contains_key("enabled"),
      "auto-learn-policy.px must have enabled"
    );
    assert!(
      map.contains_key("daily-learn-limit"),
      "must have daily-learn-limit"
    );
  } else {
    panic!("auto-learn-policy.px must eval to attrset");
  }
}

#[test]
fn eval_nix_stub_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario10-nix-stubs.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(map.get("addErrorContext"), Some(Value::Int(123))));
    assert!(matches!(
      map.get("discardContext"),
      Some(Value::String(s)) if s == "hello"
    ));
    assert!(matches!(map.get("hasContext"), Some(Value::Bool(false))));
    // Nix-compat: `unsafeGetAttrPos "x" { x = 1; }` returns a
    // `{ file, line, column }` attrset when the attribute exists. pnix
    // doesn't track per-attribute source positions yet, so we return a
    // placeholder attrset (line/column = 0, file = "<unknown>"). The
    // shape match is enough for the stub fixture.
    assert!(matches!(map.get("attrPos"), Some(Value::AttrSet(_))));
  } else {
    panic!("scenario10-nix-stubs.px must eval to attrset, got: {:?}", v);
  }
}

#[test]
fn eval_pure_builtins_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario11-pure-builtins.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(map.get("bool_alias"), Some(Value::Bool(true))));
    assert!(matches!(map.get("cmp_alias"), Some(Value::Bool(true))));
    assert!(matches!(map.get("mod_alias"), Some(Value::Int(2))));
    assert!(matches!(map.get("floor_alias"), Some(Value::Int(3))));
    assert!(matches!(map.get("find_alias"), Some(Value::Int(2))));
    assert!(matches!(map.get("get_alias"), Some(Value::Int(1))));
    assert!(matches!(
      map.get("merge_alias"),
      Some(Value::AttrSet(attrs)) if attrs.contains_key("a") && attrs.contains_key("b")
    ));
  } else {
    panic!(
      "scenario11-pure-builtins.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_schema_and_markup_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario12-schema-markup.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(
      map.get("normalized"),
      Some(Value::AttrSet(attrs)) if matches!(attrs.get("enabled"), Some(Value::Bool(true)))
    ));
    assert!(matches!(
      map.get("validation"),
      Some(Value::AttrSet(attrs)) if matches!(attrs.get("ok"), Some(Value::Bool(true)))
    ));
    assert!(matches!(
      map.get("explanation"),
      Some(Value::String(text)) if text.contains("expected string")
    ));
    assert!(matches!(
      map.get("xmlEmitted"),
      Some(Value::String(text)) if text == "<root a=\"1\"><child>text</child></root>"
    ));
    assert!(matches!(
      map.get("htmlEmitted"),
      Some(Value::String(text)) if text.contains("<div class=\"test\">Hello</div>")
    ));
  } else {
    panic!(
      "scenario12-schema-markup.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_mathml_openmath_fixture_px() {
  let path = fixture_path("pnix_expr", "five_layer_mathml_openmath.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(
      map.get("mathml_xml"),
      Some(Value::String(text)) if text.contains("<math")
    ));
    assert!(matches!(
      map.get("openmath_xml"),
      Some(Value::String(text)) if text.contains("<OMA")
    ));
    assert!(matches!(
      map.get("mathml_xml_json"),
      Some(Value::AttrSet(attrs)) if attrs.contains_key("name")
    ));
    assert!(matches!(
      map.get("openmath_xml_json"),
      Some(Value::AttrSet(attrs)) if attrs.contains_key("name")
    ));
  } else {
    panic!(
      "five_layer_mathml_openmath.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_x3d_basic_fixture_px() {
  let path = fixture_path("pnix_expr", "five_layer_x3d_basic.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(map.get("ok"), Some(Value::Bool(_))));
    assert!(matches!(
      map.get("normalized"),
      Some(Value::AttrSet(attrs)) if attrs.contains_key("attrs")
    ));
    assert!(matches!(map.get("explanation"), Some(Value::String(_))));
  } else {
    panic!("five_layer_x3d_basic.px must eval to attrset, got: {:?}", v);
  }
}

#[test]
fn eval_x3d_frp_fixture_px() {
  let path = fixture_path("pnix_expr", "five_layer_x3d_frp.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(frp) = &v {
    assert!(matches!(
      frp.get("signals"),
      Some(Value::List(signals)) if !signals.is_empty()
    ));
    assert!(matches!(
      frp.get("external_inputs"),
      Some(Value::AttrSet(_))
    ));
  } else {
    panic!(
      "five_layer_x3d_frp.px must eval to frp attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_x3d_sync_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario13-x3d-sync.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(map.get("mode"), Some(Value::String(mode)) if mode == "patch"));
    assert!(matches!(map.get("changed"), Some(Value::Bool(true))));
    assert!(matches!(
      map.get("ops"),
      Some(Value::List(ops)) if !ops.is_empty()
    ));
  } else {
    panic!("scenario13-x3d-sync.px must eval to attrset, got: {:?}", v);
  }
}

#[test]
fn eval_x3dom_webview_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario14-x3dom-webview.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
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
    assert!(matches!(
      map.get("protocol"),
      Some(Value::String(protocol)) if protocol == "pnix.x3dom.patch.v1"
    ));
    assert!(matches!(
      map.get("ops"),
      Some(Value::List(ops))
        if ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
          if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
            && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "root/x3d[0]/scene[0]/transform#mover")))
    ));
  } else {
    panic!(
      "scenario14-x3dom-webview.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_render_process_api_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario15-render-process-api.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
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
    assert!(matches!(
      map.get("simulation"),
      Some(Value::AttrSet(simulation))
        if matches!(simulation.get("hanim"), Some(Value::AttrSet(hanim)) if matches!(hanim.get("root_joint"), Some(Value::String(root)) if root == "humanoid_root"))
          && matches!(simulation.get("symbolic"), Some(Value::AttrSet(symbolic)) if matches!(symbolic.get("present"), Some(Value::Bool(true))) && matches!(symbolic.get("equation_count"), Some(Value::Int(count)) if *count >= 4))
    ));
    assert!(matches!(
      map.get("memory"),
      Some(Value::AttrSet(memory))
        if matches!(memory.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.world.memory.v1")
          && matches!(memory.get("surfaces"), Some(Value::List(surfaces)) if surfaces.iter().any(|item| matches!(item, Value::String(name) if name == "x3d")))
    ));
  } else {
    panic!(
      "scenario15-render-process-api.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_svg_schema_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario16-svg-schema.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(
      map.get("normalized"),
      Some(Value::AttrSet(attrs))
        if matches!(attrs.get("children"), Some(Value::List(children)) if children.iter().any(|child| matches!(child, Value::AttrSet(node) if matches!(node.get("name"), Some(Value::String(name)) if name == "a"))))
    ));
    assert!(matches!(
      map.get("validation"),
      Some(Value::AttrSet(attrs)) if matches!(attrs.get("ok"), Some(Value::Bool(true)))
    ));
    assert!(matches!(
      map.get("explanation"),
      Some(Value::String(text))
        if text.contains("unsupported version '3.0'") && text.contains("unknown attribute 'bogus'")
    ));
  } else {
    panic!(
      "scenario16-svg-schema.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_svg_render_packet_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario17-svg-render-packet.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(
      map.get("emitted"),
      Some(Value::String(text)) if text.contains("<svg") && text.contains("<circle")
    ));
    assert!(matches!(
      map.get("packet"),
      Some(Value::AttrSet(packet))
        if matches!(packet.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.render.packet.v1")
          && matches!(packet.get("family"), Some(Value::String(family)) if family == "svg")
          && matches!(packet.get("memory"), Some(Value::AttrSet(memory)) if matches!(memory.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.world.memory.v1"))
          && matches!(packet.get("sync"), Some(Value::AttrSet(sync))
            if matches!(sync.get("mode"), Some(Value::String(mode)) if mode == "patch")
              && matches!(sync.get("ops"), Some(Value::List(ops))
                if ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                  if matches!(op.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
                    && matches!(op.get("node_id"), Some(Value::String(id)) if id == "id:dot")
                    && matches!(op.get("target_pnix_address"), Some(Value::String(address)) if address == "svg/circle#dot")))))
    ));
  } else {
    panic!(
      "scenario17-svg-render-packet.px must eval to attrset, got: {:?}",
      v
    );
  }
}

#[test]
fn eval_svg_child_delta_fixture_px() {
  let path = fixture_path("pnix_expr", "scenario18-svg-child-delta.px");
  if !path.exists() {
    return;
  }
  let v = eval_file(&path).unwrap();
  if let Value::AttrSet(map) = &v {
    assert!(matches!(
      map.get("packet"),
      Some(Value::AttrSet(packet))
        if matches!(packet.get("sync"), Some(Value::AttrSet(sync))
          if matches!(sync.get("ops"), Some(Value::List(ops))
            if ops.iter().any(|op| matches!(op, Value::AttrSet(op)
              if matches!(op.get("op"), Some(Value::String(kind)) if kind == "remove_child")
                && matches!(op.get("node_pnix_address"), Some(Value::String(address)) if address == "svg")
                && matches!(op.get("child_node_id"), Some(Value::String(id)) if id == "id:d")
                && matches!(op.get("child_pnix_address"), Some(Value::String(address)) if address == "svg/circle#d")))
              && ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                if matches!(op.get("op"), Some(Value::String(kind)) if kind == "insert_child")
                  && matches!(op.get("node_pnix_address"), Some(Value::String(address)) if address == "svg")
                  && matches!(op.get("child_node_id"), Some(Value::String(id)) if id == "id:c")
                  && matches!(op.get("child_pnix_address"), Some(Value::String(address)) if address == "svg/rect#c")))
              && ops.iter().any(|op| matches!(op, Value::AttrSet(op)
                if matches!(op.get("op"), Some(Value::String(kind)) if kind == "reorder_children")
                  && matches!(op.get("node_pnix_address"), Some(Value::String(address)) if address == "svg")
                  && matches!(op.get("child_node_ids"), Some(Value::List(child_ids))
                    if matches!(child_ids.as_slice(),
                      [Value::String(first), Value::String(second), Value::String(third)]
                        if first == "id:b" && second == "id:a" && third == "id:c"))))))
    ));
  } else {
    panic!(
      "scenario18-svg-child-delta.px must eval to attrset, got: {:?}",
      v
    );
  }
}
